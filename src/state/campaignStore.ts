import { create } from "zustand";
import type { CampaignSnapshot, WorldEconomy, HouseBrief, CampaignDiagnostics } from "../types";
import {
  campaignStartSim,
  campaignAdvance,
  campaignGetState,
  campaignGetWorldEconomy,
  campaignGetHouses,
  campaignDiagnostics,
} from "../bridge/tauri";

interface CampaignStore {
  snapshot: CampaignSnapshot | null;
  worldEconomy: WorldEconomy | null;
  houses: HouseBrief[];
  diagnostics: CampaignDiagnostics | null;
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
  diagnostics: null,
  busy: false,
  error: null,

  refresh: async () => {
    try {
      const [snap, houses, diag] = await Promise.all([
        campaignGetState(), campaignGetHouses(), campaignDiagnostics(),
      ]);
      set({ snapshot: snap, houses, diagnostics: diag, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  start: async (seed) => {
    if (get().busy) return;
    set({ busy: true, error: null });
    try {
      const snap = await campaignStartSim(seed);
      const [we, houses, diag] = await Promise.all([
        campaignGetWorldEconomy(), campaignGetHouses(), campaignDiagnostics(),
      ]);
      set({ snapshot: snap, worldEconomy: we, houses, diagnostics: diag });
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
      const [we, houses, diag] = await Promise.all([
        campaignGetWorldEconomy(), campaignGetHouses(), campaignDiagnostics(),
      ]);
      set({ snapshot: snap, worldEconomy: we, houses, diagnostics: diag });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ busy: false });
    }
  },
}));
