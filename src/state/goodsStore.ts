import { create } from "zustand";
import type { GoodSpec } from "../types";
import {
  getGoodsSpec, setGoodsSpec, getGoodsLibrary, saveGoodsLibrary, defaultGoods,
} from "../bridge/tauri";
import { GOOD_DEFS } from "../goods";

/** Fallback metadata from the static defaults (used before specs are fetched). */
const FALLBACK = new Map(GOOD_DEFS.map((g) => [g.name, g]));

export interface GoodMeta { name: string; icon: string; color: string; }

interface GoodsStore {
  /** Active working list (the world's goods, editable in the editor). */
  specs: GoodSpec[];
  editorOpen: boolean;
  dirty: boolean;

  loadFromWorld: () => Promise<void>;
  setOpen: (open: boolean) => void;
  update: (index: number, patch: Partial<GoodSpec>) => void;
  addCustom: () => void;
  duplicate: (index: number) => void;
  remove: (index: number) => void;

  applyToWorld: () => Promise<void>;
  saveToLibrary: () => Promise<void>;
  loadFromLibrary: () => Promise<void>;
  resetDefaults: () => Promise<void>;

  /** Display metadata for a good id (active spec, else static fallback). */
  meta: (id: string) => GoodMeta;
}

function newCustom(n: number): GoodSpec {
  return {
    id: `custom_${Date.now().toString(36)}_${n}`,
    name: "New Good",
    icon: "\u{1F4E6}",
    color: "#cccccc",
    enabled: true,
    domain: "continental",
    distribution: "local",
    rarity: 0.5,
    desire: 0.45,
    category: "misc",
    need_tier: 1,
    base_value: 1,
    network_luxury: false,
    builtin: false,
    deposit: null,
    bulk: 1,
    perishable: 0,
    inputs: [],
    labor: 1,
    scoring: {
      climate: [],
      temp: [18, 8],
      precip: [400, 1600, 500],
      elevation: [0.0, 0.5, 0.25],
      abs_lat: null,
      fertility: 0.4,
      coast_bonus: 0.0,
    },
  };
}

export const useGoodsStore = create<GoodsStore>((set, get) => ({
  specs: [],
  editorOpen: false,
  dirty: false,

  loadFromWorld: async () => {
    try {
      const specs = await getGoodsSpec();
      set({ specs, dirty: false });
    } catch { /* no world yet */ }
  },

  setOpen: (editorOpen) => set({ editorOpen }),

  update: (index, patch) =>
    set((s) => ({
      specs: s.specs.map((g, i) => (i === index ? { ...g, ...patch } : g)),
      dirty: true,
    })),

  addCustom: () =>
    set((s) => ({ specs: [...s.specs, newCustom(s.specs.length)], dirty: true })),

  duplicate: (index) =>
    set((s) => {
      const src = s.specs[index];
      if (!src) return s;
      const copy: GoodSpec = {
        ...src,
        id: `custom_${Date.now().toString(36)}_${s.specs.length}`,
        name: `${src.name} (copy)`,
        builtin: false,
        // A copy of a built-in becomes a custom good; give it an editable
        // envelope so it can be tuned independently.
        scoring: src.scoring ?? { climate: [], temp: [18, 8], precip: [400, 1600, 500], elevation: [0, 0.5, 0.25], abs_lat: null, fertility: 0.4, coast_bonus: 0 },
      };
      return { specs: [...s.specs, copy], dirty: true };
    }),

  remove: (index) =>
    set((s) => ({ specs: s.specs.filter((_, i) => i !== index), dirty: true })),

  applyToWorld: async () => {
    await setGoodsSpec(get().specs);
    set({ dirty: false });
  },

  saveToLibrary: async () => { await saveGoodsLibrary(get().specs); },

  loadFromLibrary: async () => {
    const specs = await getGoodsLibrary();
    set({ specs, dirty: true });
  },

  resetDefaults: async () => {
    const specs = await defaultGoods();
    set({ specs, dirty: true });
  },

  meta: (id) => {
    const spec = get().specs.find((g) => g.id === id);
    if (spec) return { name: spec.name, icon: spec.icon, color: spec.color };
    const fb = FALLBACK.get(id);
    return fb ? { name: fb.label, icon: fb.emoji, color: fb.color } : { name: id, icon: "\u{1F4E6}", color: "#cccccc" };
  },
}));
