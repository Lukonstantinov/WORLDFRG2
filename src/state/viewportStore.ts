import { create } from "zustand";

interface ViewportStore {
  // Camera state (world coordinates of the viewport center)
  x: number;
  y: number;
  scale: number;

  // Screen dimensions
  screenWidth: number;
  screenHeight: number;

  // Visible tile range (computed)
  tileRange: { txMin: number; txMax: number; tyMin: number; tyMax: number } | null;

  // Invalidation counter — bump to force tile refetch
  tileVersion: number;

  // Pan/zoom target requested from elsewhere (e.g. clicking a city in the list).
  // `nonce` bumps so re-selecting the same cell still re-triggers a focus.
  focusTarget: { wx: number; wy: number; nonce: number } | null;

  setScreen: (w: number, h: number) => void;
  setCamera: (x: number, y: number, scale: number) => void;
  invalidateTiles: () => void;
  focusOn: (wx: number, wy: number) => void;
}

export const useViewportStore = create<ViewportStore>((set, get) => ({
  x: 0,
  y: 0,
  scale: 1,
  screenWidth: 1400,
  screenHeight: 900,
  tileRange: null,
  tileVersion: 0,
  focusTarget: null,

  setScreen: (w, h) => set({ screenWidth: w, screenHeight: h }),

  setCamera: (x, y, scale) => {
    set({ x, y, scale });
  },

  invalidateTiles: () => set((s) => ({ tileVersion: s.tileVersion + 1 })),

  focusOn: (wx, wy) =>
    set((s) => ({ focusTarget: { wx, wy, nonce: (s.focusTarget?.nonce ?? 0) + 1 } })),
}));
