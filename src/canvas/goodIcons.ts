// Trade-good icons on the MAP (region centroids, trade-lane cargo chips). The
// same 24×24 pixel sprites every panel uses (`goodArt.ts`) — one visual
// language for goods everywhere. Only the map-specific inputs live here: the
// radius-based size the overlay code speaks in, and the gemstones sublabel.

import { drawPixelIcon } from "./goodArt";

/** Per-gemstone tint, keyed by the region sublabel (Ruby/Sapphire/…). */
const GEM_COLORS: Record<string, string> = {
  Ruby: "#e0245e",
  Sapphire: "#2f6fe0",
  Emerald: "#2faa55",
  Diamond: "#cfeaf0",
  Topaz: "#e0a83a",
  Amethyst: "#9b59d0",
};

export interface IconOpts { sublabel?: string }

/** Draw one good's sprite centred at (cx, cy) filling a 2r square, in the
 *  current (already-transformed) canvas space. */
export function drawGoodIcon(
  ctx: CanvasRenderingContext2D, name: string, cx: number, cy: number, r: number,
  baseColor: string, opts: IconOpts = {},
) {
  const color = name === "gemstones" && opts.sublabel && GEM_COLORS[opts.sublabel]
    ? GEM_COLORS[opts.sublabel] : baseColor;
  drawPixelIcon(ctx, cx, cy, r * 2, color, name);
}
