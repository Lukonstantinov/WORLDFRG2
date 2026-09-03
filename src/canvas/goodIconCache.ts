// One offscreen canvas per (good, size, treatment, tint). `drawIcon` allocates
// several intermediate canvases and reads pixels back twice per call, so a list
// of 85 goods re-rendering on every React pass is real cost — every icon is
// rasterised once here and blitted after that.

import { drawIcon, drawIconVictorian } from "./goodArt";

export type GoodTreatment = "pixel" | "victorian";

/** Icons are authored at 2× the displayed size and shown at half — that is what
 *  gives the pixel treatment its crisp one-pixel edge. */
export const ICON_AUTHOR_SCALE = 2;

const cache = new Map<string, HTMLCanvasElement>();

/** Rasterise (or fetch) one good's icon. `size` is the CSS size in px; the
 *  returned canvas is `size * scale` device px square. */
export function goodIconCanvas(
  name: string, color: string, size: number, treatment: GoodTreatment = "pixel",
  scale = ICON_AUTHOR_SCALE,
): HTMLCanvasElement {
  const key = `${name}:${size}:${treatment}:${color}:${scale}`;
  const hit = cache.get(key);
  if (hit) return hit;

  const px = Math.max(8, Math.round(size * scale));
  const c = document.createElement("canvas");
  c.width = px; c.height = px;
  const ctx = c.getContext("2d");
  if (ctx) {
    if (treatment === "victorian") drawIconVictorian(ctx, px / 2, px / 2, px, color, name);
    else drawIcon(ctx, px / 2, px / 2, px, color, name, { grid: Math.max(20, Math.round(px * 0.36)) });
  }
  cache.set(key, c);
  return c;
}

/** Drop every rasterised icon — call when the goods spec's colours change. */
export function clearGoodIconCache() { cache.clear(); }
