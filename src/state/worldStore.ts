import { create } from "zustand";
import type { WorldMeta, RiverData, LakeData, Settlement } from "../types";

/** Live latitude framing, kept SEPARATE from `meta` on purpose. Dragging the
 *  Latitude Frame sliders mutates only this slice so the lat-line overlay tracks
 *  live, without churning the `meta` object identity (which ~9 expensive
 *  MapCanvas effects depend on — tile-cache rebuilds + sim IPC). It is reseeded
 *  from `meta` whenever a world loads or the backend persists a change. */
export interface LatConfig {
  equatorOffset: number;
  latScale: number;
}

interface WorldStore {
  meta: WorldMeta | null;
  isLoaded: boolean;
  /** Live latitude framing for the overlay (see LatConfig). */
  latConfig: LatConfig;
  rivers: RiverData[];
  lakes: LakeData[];
  settlements: Settlement[];
  setMeta: (meta: WorldMeta) => void;
  /** Update only the live latitude framing while dragging the sliders. Does NOT
   *  touch `meta`, so heavy meta-keyed effects stay quiet during the drag. */
  setLatConfig: (equatorOffset: number, latScale: number) => void;
  setRivers: (rivers: RiverData[]) => void;
  setLakes: (lakes: LakeData[]) => void;
  setSettlements: (settlements: Settlement[]) => void;
  clear: () => void;
}

const DEFAULT_LAT: LatConfig = { equatorOffset: 0.5, latScale: 1 };

const latFromMeta = (meta: WorldMeta): LatConfig => ({
  equatorOffset: meta.equator_offset ?? 0.5,
  latScale: meta.lat_scale ?? 1,
});

export const useWorldStore = create<WorldStore>((set) => ({
  meta: null,
  isLoaded: false,
  latConfig: DEFAULT_LAT,
  rivers: [],
  lakes: [],
  settlements: [],
  // Setting meta reseeds the live latitude framing so the two stay in sync on
  // world load and after a persisted slider change.
  setMeta: (meta) => set({ meta, isLoaded: true, latConfig: latFromMeta(meta) }),
  setLatConfig: (equatorOffset, latScale) => set({ latConfig: { equatorOffset, latScale } }),
  setRivers: (rivers) => set({ rivers }),
  setLakes: (lakes) => set({ lakes }),
  setSettlements: (settlements) => set({ settlements }),
  clear: () =>
    set({ meta: null, isLoaded: false, latConfig: DEFAULT_LAT, rivers: [], lakes: [], settlements: [] }),
}));
