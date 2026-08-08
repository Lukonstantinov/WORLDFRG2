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

  // A transient highlight pin (e.g. the searched settlement). `nonce` lets the
  // overlay restart its pulse animation each time a new pin is dropped.
  searchPin: { wx: number; wy: number; nonce: number } | null;

  // A war highlight: the two belligerent seats — a = attacker (drawn red), b =
  // defender (drawn blue) — lit on the map when a war is picked in the War Council.
  warHighlight: { ax: number; ay: number; bx: number; by: number; nonce: number } | null;

  setScreen: (w: number, h: number) => void;
  setCamera: (x: number, y: number, scale: number) => void;
  invalidateTiles: () => void;
  focusOn: (wx: number, wy: number) => void;
  setSearchPin: (wx: number, wy: number) => void;
  clearSearchPin: () => void;
  setWarHighlight: (ax: number, ay: number, bx: number, by: number) => void;
  clearWarHighlight: () => void;
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
  searchPin: null,
  warHighlight: null,

  setScreen: (w, h) => set({ screenWidth: w, screenHeight: h }),

  setCamera: (x, y, scale) => {
    set({ x, y, scale });
  },

  invalidateTiles: () => set((s) => ({ tileVersion: s.tileVersion + 1 })),

  focusOn: (wx, wy) =>
    set((s) => ({ focusTarget: { wx, wy, nonce: (s.focusTarget?.nonce ?? 0) + 1 } })),

  setSearchPin: (wx, wy) =>
    set((s) => ({ searchPin: { wx, wy, nonce: (s.searchPin?.nonce ?? 0) + 1 } })),
  clearSearchPin: () => set({ searchPin: null }),
  setWarHighlight: (ax, ay, bx, by) =>
    set((s) => ({ warHighlight: { ax, ay, bx, by, nonce: (s.warHighlight?.nonce ?? 0) + 1 } })),
  clearWarHighlight: () => set({ warHighlight: null }),
}));
