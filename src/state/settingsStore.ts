import { create } from "zustand";
import { LINE_COLOR_DEFAULTS, type LineColorKey, setLineColors } from "../canvas/OverlayManager";
import { setAppearance } from "@bridge";

/** Adjustable appearance settings — the user-editable overlay/connection-line
 *  palette. Persisted to localStorage (per-machine default) and, when a world /
 *  campaign is open, saved alongside it (file value wins on open via `hydrate`).
 *  The store is the single source of truth; every change is mirrored into
 *  `OverlayManager`'s live `lineColors` registry via `setLineColors`. */

export type LineColors = Record<LineColorKey, string>;

const LS_KEY = "wf2.appearance.lineColors";

/** Preset palettes (sparse overrides on top of the defaults). */
export const COLOR_PRESETS: Record<string, Partial<LineColors>> = {
  Default: {},
  "High contrast": {
    tradeTrunk: "#ffd000", dynamicFlow: "#00e5ff", tradeLand: "#ff9d3a",
    tradeSea: "#36c6ff", tradeRiver: "#5bff7a", corridor: "#19f0b0",
    manufactory: "#2bff6a", estate: "#ffe000",
  },
  "Colour-blind safe": {
    // Okabe–Ito–leaning hues (avoid red/green confusion)
    tradeTrunk: "#e69f00", tradeTrunkMinor: "#cc9b54", dynamicFlow: "#56b4e9",
    tradeLand: "#e69f00", tradeSea: "#56b4e9", tradeRiver: "#0072b2",
    corridor: "#009e8e", corridorArrow: "#4fd0c0", merchantIn: "#56b4e9",
    merchantOut: "#f0e442", manufactory: "#009e73", estate: "#f0e442",
  },
};
export type PresetKey = keyof typeof COLOR_PRESETS;
export type ThemeName = PresetKey | "Custom";

function loadLocal(): Partial<LineColors> {
  try {
    const s = localStorage.getItem(LS_KEY);
    return s ? (JSON.parse(s) as Partial<LineColors>) : {};
  } catch { return {}; }
}
function saveLocal(c: LineColors) {
  try { localStorage.setItem(LS_KEY, JSON.stringify(c)); } catch { /* ignore */ }
}

/** Merge defaults + a sparse override into a full palette. */
function merge(over: Partial<LineColors>): LineColors {
  return { ...LINE_COLOR_DEFAULTS, ...over } as LineColors;
}

/** A full palette → the sparse override (only keys differing from defaults). */
function toOverride(c: LineColors): Partial<LineColors> {
  const out: Partial<LineColors> = {};
  (Object.keys(LINE_COLOR_DEFAULTS) as LineColorKey[]).forEach((k) => {
    if (c[k] !== LINE_COLOR_DEFAULTS[k]) out[k] = c[k];
  });
  return out;
}

const initial = merge(loadLocal());
setLineColors(initial); // seed the renderer registry before first paint

interface SettingsState {
  lineColors: LineColors;
  preset: ThemeName;
  setLineColor: (k: LineColorKey, hex: string) => void;
  resetLineColor: (k: LineColorKey) => void;
  resetAll: () => void;
  applyPreset: (p: PresetKey) => void;
  /** Hydrate from a world/campaign file (file value wins, no localStorage clobber
   *  of the file's intent — but we also persist locally so it sticks). */
  hydrate: (over: Partial<LineColors> | null | undefined) => void;
  /** Current palette as a sparse override vs defaults (for saving with the file). */
  asOverride: () => Partial<LineColors>;
}

/** When false (during file hydrate) we skip writing back to the world file. */
function apply(
  set: (s: Partial<SettingsState>) => void, colors: LineColors, preset: ThemeName,
  persistToFile = true,
) {
  setLineColors(colors);
  saveLocal(colors);
  if (persistToFile) setAppearance(JSON.stringify(toOverride(colors))).catch(() => {});
  set({ lineColors: colors, preset });
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  lineColors: initial,
  preset: "Default",
  setLineColor: (k, hex) => apply(set, { ...get().lineColors, [k]: hex }, "Custom"),
  resetLineColor: (k) =>
    apply(set, { ...get().lineColors, [k]: LINE_COLOR_DEFAULTS[k] }, "Custom"),
  resetAll: () => apply(set, merge({}), "Default"),
  applyPreset: (p) => apply(set, merge(COLOR_PRESETS[p] ?? {}), p),
  // From a world/campaign file: file value wins, and we DON'T write it straight
  // back (persistToFile = false).
  hydrate: (over) => apply(set, merge(over ?? {}), over && Object.keys(over).length ? "Custom" : "Default", false),
  asOverride: () => toOverride(get().lineColors),
}));
