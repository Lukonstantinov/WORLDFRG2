import { create } from "zustand";
import type { CampaignSnapshot, WorldEconomy, HouseBrief, CampaignDiagnostics } from "../types";
import {
  campaignStartSim,
  campaignNewGame,
  campaignAdvance,
  campaignGetState,
  campaignGetWorldEconomy,
  campaignGetHouses,
  campaignDiagnostics,
  campaignPersist,
  saveCampaignAs,
} from "../bridge/tauri";

/** Auto-advance step size for the campaign clock. */
export type CampaignSpeed = "week" | "month" | "year";
export const SPEED_TICKS: Record<CampaignSpeed, number> = { week: 7, month: 30, year: 365 };

interface CampaignStore {
  snapshot: CampaignSnapshot | null;
  worldEconomy: WorldEconomy | null;
  houses: HouseBrief[];
  diagnostics: CampaignDiagnostics | null;
  busy: boolean;
  error: string | null;
  /** The campaign clock lives in the STORE so any surface (top HUD, ledger rail)
   *  can drive or reflect it — exactly one auto-advance loop ever runs. */
  playing: boolean;
  speed: CampaignSpeed;
  setSpeed: (s: CampaignSpeed) => void;
  /** Start auto-advancing one `speed` step per beat until pause() (or an error). */
  play: () => Promise<void>;
  /** Stop the clock; the running loop then refreshes panels + flushes to disk. */
  pause: () => void;
  /** Index of the house focused in the Houses panel — the map highlights only it
   *  (its sphere, routes, offices). null = show all houses. */
  selectedHouseIdx: number | null;
  setSelectedHouse: (idx: number | null) => void;

  /** Load the current sim state (called when the campaign step opens). */
  refresh: () => Promise<void>;
  /** Seed a brand-new living-trade simulation. */
  start: (seed: number) => Promise<void>;
  /** Start a NEW dynamic campaign on the same world — first SAVES the current running
   *  campaign to its own .campaign file (a running campaign is never wiped), then
   *  reseeds fresh. Returns false if the user cancelled the save dialog. */
  newGame: (seed: number) => Promise<boolean>;
  /** Advance N days. `heavy` (default true) also refreshes the world-economy +
   *  houses + diagnostics; pass false during fast Play to update only the snapshot
   *  (clock + map markers) and skip the costlier panel queries. */
  advance: (ticks: number, heavy?: boolean) => Promise<void>;
}

export const useCampaignStore = create<CampaignStore>((set, get) => ({
  snapshot: null,
  worldEconomy: null,
  houses: [],
  diagnostics: null,
  busy: false,
  error: null,
  playing: false,
  speed: "month",
  selectedHouseIdx: null,
  setSelectedHouse: (idx) => set({ selectedHouseIdx: idx }),
  setSpeed: (s) => set({ speed: s }),
  pause: () => set({ playing: false }),

  play: async () => {
    if (get().playing) return;
    set({ playing: true });
    let i = 0;
    while (get().playing && get().snapshot?.active === true) {
      i++;
      // Heavy panel refresh (economy / houses / diagnostics) only every 6th step;
      // the other steps update just the snapshot (clock + live map markers), so
      // fast Play doesn't fire the costlier panel queries every single tick.
      await get().advance(SPEED_TICKS[get().speed], i % 6 === 0);
      if (get().error) break;
      await new Promise((r) => setTimeout(r, 450));
    }
    // Paused (or errored): bring the panels current and flush the resident sim.
    set({ playing: false });
    await get().refresh();
    campaignPersist().catch(() => {});
  },

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

  newGame: async (seed) => {
    if (get().busy) return false;
    set({ playing: false }); // stop the clock before swapping campaigns
    const snap = get().snapshot;
    const running = snap?.active === true && (snap?.clock.tick ?? 0) > 0;
    // A running campaign must be preserved in its OWN file before a new one starts.
    if (running) {
      let path: string | null = null;
      try {
        const { save } = await import("@tauri-apps/plugin-dialog");
        path = await save({
          title: "Save the current campaign before starting a new one",
          filters: [{ name: "WorldForge Campaign", extensions: ["campaign"] }],
          defaultPath: "campaign.campaign",
        });
      } catch {
        path = prompt("Save the current campaign to path (.campaign):", "campaign.campaign");
      }
      if (!path) return false; // cancelled → keep the running game untouched
      set({ busy: true, error: null });
      try {
        await campaignPersist();
        await saveCampaignAs(path);
      } catch (e) {
        set({ error: String(e), busy: false });
        return false;
      }
    } else {
      set({ busy: true, error: null });
    }
    try {
      const s = await campaignNewGame(seed);
      const [we, houses, diag] = await Promise.all([
        campaignGetWorldEconomy(), campaignGetHouses(), campaignDiagnostics(),
      ]);
      set({ snapshot: s, worldEconomy: we, houses, diagnostics: diag });
      return true;
    } catch (e) {
      set({ error: String(e) });
      return false;
    } finally {
      set({ busy: false });
    }
  },

  advance: async (ticks, heavy = true) => {
    if (get().busy) return;
    set({ busy: true, error: null });
    try {
      const snap = await campaignAdvance(ticks);
      if (heavy) {
        const [we, houses, diag] = await Promise.all([
          campaignGetWorldEconomy(), campaignGetHouses(), campaignDiagnostics(),
        ]);
        set({ snapshot: snap, worldEconomy: we, houses, diagnostics: diag });
      } else {
        // Fast Play tick: update only the snapshot (clock + live map markers).
        set({ snapshot: snap });
      }
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ busy: false });
    }
  },
}));
