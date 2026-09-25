// The pixel treatment for PEOPLE and BUILDING art (`cultureDress.ts`,
// `marketSquareArt.ts`): any vector draw function rendered onto a coarse grid,
// edged and bevelled, then upscaled with smoothing off. Split out of the old
// `goodArt.ts` when goods moved to hand-placed 24×24 sprites — goods no longer
// go through this path.

type Ctx = CanvasRenderingContext2D;

function hx(h: string): [number, number, number] {
  let s = (h || "#888888").replace("#", "");
  if (s.length === 3) s = s.split("").map((c) => c + c).join("");
  return [parseInt(s.slice(0, 2), 16), parseInt(s.slice(2, 4), 16), parseInt(s.slice(4, 6), 16)];
}
function rgb(c: number[]): string { return `rgb(${c[0] | 0},${c[1] | 0},${c[2] | 0})`; }
export function shade(hex: string, f: number): string {
  const c = hx(hex);
  return rgb(f >= 1
    ? [c[0] + (255 - c[0]) * (f - 1), c[1] + (255 - c[1]) * (f - 1), c[2] + (255 - c[2]) * (f - 1)]
    : [c[0] * f, c[1] * f, c[2] * f]);
}

function mkCanvas(w: number, h: number): HTMLCanvasElement {
  const c = document.createElement("canvas"); c.width = w; c.height = h; return c;
}

/** The SET'S PIXEL TREATMENT, applied to any draw function instead of a good's
 *  recipe: art rendered onto a coarse grid, stamped with a one-pixel dark edge,
 *  given a shiny top-left rim and a shaded bottom-right one, then upscaled with
 *  smoothing off. `draw(c)` authors into a 100-wide box whose height follows the
 *  requested aspect. `dx,dy` is the box's top-left in device space. */
export function pixelize(
  ctx: Ctx, dx: number, dy: number, dw: number, dh: number, cols: number, draw: (c: Ctx) => void,
) {
  const pad = 3, C = Math.max(8, Math.round(cols)), rows = Math.max(6, Math.round(C * dh / dw));
  const W = C + pad * 2, H = rows + pad * 2;
  const mk = () => mkCanvas(W, H);
  const art = mk(), a = art.getContext("2d")!;
  a.save(); a.translate(pad, pad); a.scale(C / 100, C / 100); a.lineJoin = "round"; a.lineCap = "round";
  draw(a); a.restore();
  const sil = (fill: string) => {
    const c = mk(), x = c.getContext("2d")!; x.drawImage(art, 0, 0);
    x.globalCompositeOperation = "source-in"; x.fillStyle = fill; x.fillRect(0, 0, W, H); return c;
  };
  const dark = sil("#0d0b08"), light = sil("#ffffff");
  const out = mk(), o = out.getContext("2d")!;
  for (const [ox, oy] of [[-1, 0], [1, 0], [0, -1], [0, 1], [-1, -1], [1, -1], [-1, 1], [1, 1]]) o.drawImage(dark, ox, oy);
  o.drawImage(art, 0, 0);
  const band = (src: HTMLCanvasElement, ox: number, oy: number) => {
    const c = mk(), x = c.getContext("2d")!; x.drawImage(src, 0, 0);
    x.globalCompositeOperation = "destination-out"; x.drawImage(art, ox, oy); return c;
  };
  o.globalCompositeOperation = "source-atop";
  o.globalAlpha = 0.72; o.drawImage(band(light, 1, 1), 0, 0);
  o.globalAlpha = 0.36; o.drawImage(band(dark, -1, -1), 0, 0);
  o.globalAlpha = 1; o.globalCompositeOperation = "source-over";
  const px = dw / C;
  ctx.save(); ctx.imageSmoothingEnabled = false;
  ctx.drawImage(out, dx - pad * px, dy - pad * px, W * px, H * px);
  ctx.restore();
}
