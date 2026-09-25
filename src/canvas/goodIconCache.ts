// One offscreen canvas per (good, size, tint, device scale) for the React
// `GoodIcon` — the sprite is already cached by `goodSprite`, this caches the
// scaled blit so a list of every good costs one drawImage per row.

import { drawPixelIcon } from "./goodArt";

const cache = new Map<string, HTMLCanvasElement>();

/** Rasterise (or fetch) one good's icon. `size` is the CSS size in px; the
 *  returned canvas is `size * scale` device px square. */
export function goodIconCanvas(name: string, color: string, size: number, scale = 1): HTMLCanvasElement {
  const key = `${name}:${size}:${color}:${scale}`;
  const hit = cache.get(key);
  if (hit) return hit;

  const px = Math.max(8, Math.round(size * scale));
  const c = document.createElement("canvas");
  c.width = px; c.height = px;
  const ctx = c.getContext("2d");
  if (ctx) drawPixelIcon(ctx, px / 2, px / 2, px, color, name);
  cache.set(key, c);
  return c;
}
