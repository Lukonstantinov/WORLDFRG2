// One offscreen canvas per (good, size, tint, device scale) for the React
// `GoodIcon` — the sprite is already cached by `goodSprite`, this caches the
// scaled blit so a list of every good costs one drawImage per row.

import { drawPixelIcon, setCustomPixels } from "./goodArt";

const cache = new Map<string, HTMLCanvasElement>();

/** Register every spec's author-drawn icon (Goods Builder) and drop the stale
 *  blits, so each icon redraws with it everywhere in the app. */
const lastPixels = new Map<string, string | null>();
export function registerCustomIcons(specs: { id: string; pixels?: string | null }[]) {
  for (const s of specs) {
    const enc = s.pixels ?? null;
    if ((lastPixels.get(s.id) ?? null) === enc) continue;
    lastPixels.set(s.id, enc);
    setCustomPixels(s.id, enc);
    for (const k of [...cache.keys()]) if (k.startsWith(s.id + ":")) cache.delete(k);
  }
}

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
