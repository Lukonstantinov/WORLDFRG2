import { create } from "zustand";
import {
  LINE_COLOR_DEFAULTS, type LineColorKey, setLineColors,
  LABEL_STYLE_DEFAULTS, LABEL_THEMES, type LabelKey, type LabelStyle,
  setLabelStyles, resolveLabelTheme,
} from "@canvas/OverlayManager";
import { setAppearance } from "@bridge";

/** Adjustable appearance settings — the user-editable overlay/connection-line
 *  palette. Persisted to localStorage (per-machine default) and, when a world /
 *  campaign is open, saved alongside it (file value wins on open via `hydrate`).
 *  The store is the single source of truth; every change is mirrored into
 *  `OverlayManager`'s live `lineColors` registry via `setLineColors`. */

export type LineColors = Record<LineColorKey, string>;
export type LabelStyles = Record<LabelKey, LabelStyle>;
/** Sparse per-class typography override (only what differs from the defaults). */
export type LabelOverride = Partial<Record<LabelKey, Partial<LabelStyle>>>;

const LS_KEY = "wf2.appearance.lineColors";
const LS_LABEL_KEY = "wf2.appearance.labelStyles";

/** Label typography themes, re-exported so the panel can list them by name. */
export const LABEL_THEME_NAMES = Object.keys(LABEL_THEMES);
export const DEFAULT_LABEL_THEME = "Mixed Contrast";

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
function loadLocalLabels(): { over: LabelOverride; theme: ThemeName } {
  try {
    const s = localStorage.getItem(LS_LABEL_KEY);
    if (!s) return { over: {}, theme: DEFAULT_LABEL_THEME };
    const p = JSON.parse(s) as { over?: LabelOverride; theme?: ThemeName };
    return { over: p.over ?? {}, theme: p.theme ?? DEFAULT_LABEL_THEME };
  } catch { return { over: {}, theme: DEFAULT_LABEL_THEME }; }
}
function saveLocalLabels(over: LabelOverride, theme: ThemeName) {
  try { localStorage.setItem(LS_LABEL_KEY, JSON.stringify({ over, theme })); } catch { /* ignore */ }
}

/** Full label styles → the sparse override (only fields differing from defaults). */
function labelsToOverride(s: LabelStyles): LabelOverride {
  const out: LabelOverride = {};
  (Object.keys(LABEL_STYLE_DEFAULTS) as LabelKey[]).forEach((k) => {
    const d = LABEL_STYLE_DEFAULTS[k];
    const cur = s[k];
    const diff: Partial<LabelStyle> = {};
    (Object.keys(d) as (keyof LabelStyle)[]).forEach((f) => {
      if (cur[f] !== d[f]) (diff as Record<string, unknown>)[f] = cur[f];
    });
    if (Object.keys(diff).length > 0) out[k] = diff;
  });
  return out;
}
/** Defaults + a sparse override → a full style set. */
function mergeLabels(over: LabelOverride): LabelStyles {
  return resolveLabelTheme(over ?? {});
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
const initialLabelsLS = loadLocalLabels();
const initialLabels = mergeLabels(initialLabelsLS.over);
setLabelStyles(initialLabels);

/** What we persist into the world/campaign file.
 *
 *  The stored value used to be a BARE sparse `Partial<LineColors>`. It is now a
 *  versioned envelope so label typography can ride along; `readEnvelope` still
 *  understands the old flat shape, so worlds saved before this change open fine.
 *  The backend treats the value as an opaque string, so no Rust change is needed. */
interface AppearanceEnvelope { v: 2; lineColors: Partial<LineColors>; labels: LabelOverride }

function writeEnvelope(colors: LineColors, labels: LabelStyles): string {
  const env: AppearanceEnvelope = {
    v: 2, lineColors: toOverride(colors), labels: labelsToOverride(labels),
  };
  return JSON.stringify(env);
}

export function readEnvelope(
  raw: Partial<LineColors> | AppearanceEnvelope | null | undefined,
): { colors: Partial<LineColors>; labels: LabelOverride } {
  if (!raw || typeof raw !== "object") return { colors: {}, labels: {} };
  if ((raw as AppearanceEnvelope).v === 2) {
    const e = raw as AppearanceEnvelope;
    return { colors: e.lineColors ?? {}, labels: e.labels ?? {} };
  }
  // Legacy: a flat map of line-colour keys, written before the envelope existed.
  return { colors: raw as Partial<LineColors>, labels: {} };
}

interface SettingsState {
  lineColors: LineColors;
  preset: ThemeName;
  setLineColor: (k: LineColorKey, hex: string) => void;
  resetLineColor: (k: LineColorKey) => void;
  resetAll: () => void;
  applyPreset: (p: PresetKey) => void;
  /** Hydrate from a world/campaign file (file value wins, no localStorage clobber
   *  of the file's intent — but we also persist locally so it sticks). Accepts the
   *  v2 envelope or the legacy flat colour map. */
  hydrate: (raw: unknown) => void;
  /** Current palette as a sparse override vs defaults (for saving with the file). */
  asOverride: () => Partial<LineColors>;

  // ── Map label typography ──
  labelStyles: LabelStyles;
  labelTheme: ThemeName;
  setLabelStyle: (k: LabelKey, patch: Partial<LabelStyle>) => void;
  resetLabelStyle: (k: LabelKey) => void;
  applyLabelTheme: (name: string) => void;
  resetLabels: () => void;
}

/** Push a full appearance state into the renderer, localStorage and (unless we are
 *  hydrating FROM a file) the open world/campaign. The file carries colours and label
 *  typography together in one envelope, so every write sends both. */
function apply(
  set: (s: Partial<SettingsState>) => void,
  colors: LineColors, preset: ThemeName,
  labels: LabelStyles, labelTheme: ThemeName,
  persistToFile = true,
) {
  setLineColors(colors);
  setLabelStyles(labels);
  saveLocal(colors);
  saveLocalLabels(labelsToOverride(labels), labelTheme);
  if (persistToFile) setAppearance(writeEnvelope(colors, labels)).catch(() => {});
  set({ lineColors: colors, preset, labelStyles: labels, labelTheme });
}

export const useSettingsStore = create<SettingsState>((set, get) => {
  /** Colour-only change — carries the current labels through unchanged. */
  const applyColors = (colors: LineColors, preset: ThemeName) => {
    const s = get();
    apply(set, colors, preset, s.labelStyles, s.labelTheme);
  };
  /** Label-only change — carries the current colours through unchanged. */
  const applyLabels = (labels: LabelStyles, labelTheme: ThemeName) => {
    const s = get();
    apply(set, s.lineColors, s.preset, labels, labelTheme);
  };
  return {
    lineColors: initial,
    preset: "Default",
    setLineColor: (k, hex) => applyColors({ ...get().lineColors, [k]: hex }, "Custom"),
    resetLineColor: (k) =>
      applyColors({ ...get().lineColors, [k]: LINE_COLOR_DEFAULTS[k] }, "Custom"),
    resetAll: () => applyColors(merge({}), "Default"),
    applyPreset: (p) => applyColors(merge(COLOR_PRESETS[p] ?? {}), p),
    // From a world/campaign file: file value wins, and we DON'T write it straight
    // back (persistToFile = false). Accepts the v2 envelope or the legacy flat map.
    hydrate: (raw) => {
      const { colors, labels } = readEnvelope(raw as Parameters<typeof readEnvelope>[0]);
      apply(
        set,
        merge(colors), Object.keys(colors).length ? "Custom" : "Default",
        mergeLabels(labels), Object.keys(labels).length ? "Custom" : DEFAULT_LABEL_THEME,
        false,
      );
    },
    asOverride: () => toOverride(get().lineColors),

    // ── Map label typography ──
    labelStyles: initialLabels,
    labelTheme: initialLabelsLS.theme,
    setLabelStyle: (k, patch) =>
      applyLabels({ ...get().labelStyles, [k]: { ...get().labelStyles[k], ...patch } }, "Custom"),
    resetLabelStyle: (k) =>
      applyLabels({ ...get().labelStyles, [k]: { ...LABEL_STYLE_DEFAULTS[k] } }, "Custom"),
    applyLabelTheme: (name) =>
      applyLabels(resolveLabelTheme(LABEL_THEMES[name] ?? {}), name),
    resetLabels: () => applyLabels(resolveLabelTheme({}), DEFAULT_LABEL_THEME),
  };
});
