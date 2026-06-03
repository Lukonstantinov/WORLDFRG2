import { create } from "zustand";
import type { WorldMeta, RiverData, LakeData, Settlement } from "../types";

interface WorldStore {
  meta: WorldMeta | null;
  isLoaded: boolean;
  rivers: RiverData[];
  lakes: LakeData[];
  settlements: Settlement[];
  setMeta: (meta: WorldMeta) => void;
  /** Patch just the latitude framing on the current meta (optimistic, so the
   *  lat-line overlay moves live while dragging the sliders). */
  setLatConfig: (equatorOffset: number, latScale: number) => void;
  setRivers: (rivers: RiverData[]) => void;
  setLakes: (lakes: LakeData[]) => void;
  setSettlements: (settlements: Settlement[]) => void;
  clear: () => void;
}

export const useWorldStore = create<WorldStore>((set) => ({
  meta: null,
  isLoaded: false,
  rivers: [],
  lakes: [],
  settlements: [],
  setMeta: (meta) => set({ meta, isLoaded: true }),
  setLatConfig: (equator_offset, lat_scale) =>
    set((s) => (s.meta ? { meta: { ...s.meta, equator_offset, lat_scale } } : s)),
  setRivers: (rivers) => set({ rivers }),
  setLakes: (lakes) => set({ lakes }),
  setSettlements: (settlements) => set({ settlements }),
  clear: () => set({ meta: null, isLoaded: false, rivers: [], lakes: [], settlements: [] }),
}));
