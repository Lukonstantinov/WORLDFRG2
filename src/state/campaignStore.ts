import { create } from "zustand";
import type { CampaignSnapshot, WorldEconomy, HouseBrief, CampaignDiagnostics, ContractRow } from "../types";
import {
  campaignStartSim,
  campaignAdvance,
  campaignGetState,
  campaignGetWorldEconomy,
  campaignGetHouses,
  campaignGetContracts,
  campaignDiagnostics,
} from "../bridge/tauri";

interface CampaignStore {
  snapshot: CampaignSnapshot | null;
  worldEconomy: WorldEconomy | null;
  houses: HouseBrief[];
  contracts: ContractRow[];
  diagnostics: CampaignDiagnostics | null;
  busy: boolean;
  error: string | null;
  /** Index of the house focused in the Houses panel — the map highlights only it
   *  (its sphere, routes, offices). null = show all houses. */
  selectedHouseIdx: number | null;
  setSelectedHouse: (idx: number | null) => void;

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
  contracts: [],
  diagnostics: null,
  busy: false,
  error: null,
  selectedHouseIdx: null,
  setSelectedHouse: (idx) => set({ selectedHouseIdx: idx }),

  refresh: async () => {
    try {
      const [snap, houses, contracts, diag] = await Promise.all([
        campaignGetState(), campaignGetHouses(), campaignGetContracts(), campaignDiagnostics(),
      ]);
      set({ snapshot: snap, houses, contracts, diagnostics: diag, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  start: async (seed) => {
    if (get().busy) return;
    set({ busy: true, error: null });
    try {
      const snap = await campaignStartSim(seed);
      const [we, houses, contracts, diag] = await Promise.all([
        campaignGetWorldEconomy(), campaignGetHouses(), campaignGetContracts(), campaignDiagnostics(),
      ]);
      set({ snapshot: snap, worldEconomy: we, houses, contracts, diagnostics: diag });
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
      const [we, houses, contracts, diag] = await Promise.all([
        campaignGetWorldEconomy(), campaignGetHouses(), campaignGetContracts(), campaignDiagnostics(),
      ]);
      set({ snapshot: snap, worldEconomy: we, houses, contracts, diagnostics: diag });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ busy: false });
    }
  },
}));
