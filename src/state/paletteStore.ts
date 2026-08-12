import { create } from "zustand";
import { getRenderPalettes } from "@bridge";
import type { RenderPalettes } from "@types";

/** THE RENDERER'S OWN COLOUR TABLES, fetched once and cached.
 *
 *  The legend used to keep hand-maintained copies of these — four of them, across
 *  three files, none checked against the Rust that paints the pixels. They drifted:
 *  the elevation sea key was copied from the wrong renderer and ran BACKWARDS, and
 *  the histogram disagreed with the map by ΔE 19–24 in the high bands.
 *
 *  Fetching them removes the second copy rather than testing it, so the legend
 *  cannot be wrong about the map without the map being wrong about itself. */
interface PaletteState {
  palettes: RenderPalettes | null;
  loading: boolean;
  load: () => Promise<void>;
}

export const usePaletteStore = create<PaletteState>((set, get) => ({
  palettes: null,
  loading: false,
  load: async () => {
    if (get().palettes || get().loading) return;
    set({ loading: true });
    try {
      set({ palettes: await getRenderPalettes(), loading: false });
    } catch {
      // A failed fetch just means no legend this session — never block the map.
      set({ loading: false });
    }
  },
}));

/** A CSS `linear-gradient` for a continuous ramp, with each stop placed at its TRUE
 *  position in the ramp's own units rather than at an even interval.
 *
 *  This is the bug the old legend had in its land bands: six equal blocks labelled
 *  0/1500/3000/5000/7000/8848 m described a ramp whose stops actually fell at
 *  1327/3097/5309/7521 m, so a colour read off the map resolved to the wrong height
 *  and the interpolation between labels was wrong too. */
export function rampGradient(stops: { at: number; color: string }[]): string {
  if (stops.length === 0) return "transparent";
  const lo = stops[0].at;
  const hi = stops[stops.length - 1].at;
  const span = hi - lo || 1;
  const parts = stops.map((s) => `${s.color} ${(((s.at - lo) / span) * 100).toFixed(2)}%`);
  return `linear-gradient(to right, ${parts.join(", ")})`;
}

/** A CLASSED scale draws as hard steps, not a blend — that is the whole point of
 *  classing rainfall, and a gradient would misrepresent it as continuous. */
export function bandGradient(bands: { at: number; color: string }[]): string {
  if (bands.length === 0) return "transparent";
  const n = bands.length;
  const parts = bands.flatMap((b, i) => {
    const from = ((i / n) * 100).toFixed(2);
    const to = (((i + 1) / n) * 100).toFixed(2);
    return [`${b.color} ${from}%`, `${b.color} ${to}%`];
  });
  return `linear-gradient(to right, ${parts.join(", ")})`;
}

/** Mix two hex colours at fraction `k` (0 = `a`, 1 = `b`). A small general-purpose
 *  helper — the legend's own swatches need it wherever a served ramp gives colours
 *  but a UI ladder wants an intermediate shade (e.g. the goods-quality hue-mode
 *  legend, which mixes a neutral grey toward itself to stand in for "however strong
 *  a good's own colour would show here"). */
export function mixHex(a: string, b: string, k: number): string {
  const pa = parseInt(a.slice(1), 16);
  const pb = parseInt(b.slice(1), 16);
  const mix = (sh: number) => {
    const ca = (pa >> sh) & 255;
    const cb = (pb >> sh) & 255;
    return Math.round(ca + (cb - ca) * k);
  };
  return `#${[16, 8, 0].map((sh) => mix(sh).toString(16).padStart(2, "0")).join("")}`;
}

/** Sample a ramp at a position in its own units — used to colour the elevation
 *  distribution bars from the SAME table the map draws, instead of the fourth
 *  hand-copied palette the standalone histogram used to carry (it disagreed with
 *  the map by ΔE 19–24 in the high bands). */
export function rampAt(stops: { at: number; color: string }[], x: number): string {
  if (stops.length === 0) return "#6e7660";
  if (x <= stops[0].at) return stops[0].color;
  const last = stops[stops.length - 1];
  if (x >= last.at) return last.color;
  for (let i = 0; i < stops.length - 1; i++) {
    const a = stops[i];
    const b = stops[i + 1];
    if (x <= b.at) {
      const t = b.at === a.at ? 0 : (x - a.at) / (b.at - a.at);
      const pa = parseInt(a.color.slice(1), 16);
      const pb = parseInt(b.color.slice(1), 16);
      const mix = (sh: number) => {
        const ca = (pa >> sh) & 255;
        const cb = (pb >> sh) & 255;
        return Math.round(ca + (cb - ca) * t);
      };
      return `rgb(${mix(16)}, ${mix(8)}, ${mix(0)})`;
    }
  }
  return last.color;
}
