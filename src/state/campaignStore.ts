import { create } from "zustand";
import type { CampaignSnapshot, WorldEconomy, HouseBrief } from "../types";
import {
  campaignStartSim,
  campaignAdvance,
  campaignGetState,
  campaignGetWorldEconomy,
  campaignGetHouses,
} from "../bridge/tauri";

interface CampaignStore {
  snapshot: CampaignSnapshot | null;
  worldEconomy: WorldEconomy | null;
  houses: HouseBrief[];
  busy: boolean;
  error: string | null;

  /** Load the current sim state (called when the campaign step opens). */
  refresh: () => Promise<void>;
  /** Seed a brand-new living-trade simulation. */
  start: (seed: number) => Promise<void>;
  /** Advance N days and refresh the world-economy + houses data. */
  advance: (ticks: number) => Promise<void>;
}

export const useCampaignStore = create<CampaignStore>((set, get) => ({
  snapshot: null,
  worldEconomy: null,
  houses: [],
  busy: false,
  error: null,

  refresh: async () => {
    try {
      const [snap, houses] = await Promise.all([campaignGetState(), campaignGetHouses()]);
      set({ snapshot: snap, houses, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  start: async (seed) => {
    if (get().busy) return;
    set({ busy: true, error: null });
    try {
      const snap = await campaignStartSim(seed);
      const [we, houses] = await Promise.all([campaignGetWorldEconomy(), campaignGetHouses()]);
      set({ snapshot: snap, worldEconomy: we, houses });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ busy: false });
    }
  },

  advance: async (ticks) => {
    if (get().busy) return;
    set({ busy: true, error: null });
    try {
      const snap = await campaignAdvance(ticks);
      const [we, houses] = await Promise.all([campaignGetWorldEconomy(), campaignGetHouses()]);
      set({ snapshot: snap, worldEconomy: we, houses });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ busy: false });
    }
  },
}));
