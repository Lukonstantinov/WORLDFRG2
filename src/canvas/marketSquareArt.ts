// THE CITY MARKET SQUARE — a front elevation at eye level: the city closing the
// square behind, four trestle stalls across the front, and the crowd that
// actually buys, drawn from the same culture kits as the dress plates, so a
// port's people are visible in its market.
//
// The backdrop is FLAT FACADES, not the isometric `buildingArt` set: the square
// is seen at eye level, and a bird's-eye skyline over ground-level figures reads
// as two different worlds stacked.

import { pixelize, drawIcon } from "./goodArt";
import { flag } from "./buildingArt";
import { drawFigure, resolveKit, type KitSpec } from "@ui/campaign/cultureDress";

type Ctx = CanvasRenderingContext2D;
const T2 = Math.PI * 2;

/** The square's logical size. Render at 2× device scale. */
export const SQUARE_W = 962, SQUARE_H = 404;

const P = (c: Ctx, pts: number[][], f: string) => {
  c.beginPath(); c.moveTo(pts[0][0], pts[0][1]);
  for (let i = 1; i < pts.length; i++) c.lineTo(pts[i][0], pts[i][1]);
  c.closePath(); c.fillStyle = f; c.fill();
};
const L = (c: Ctx, pts: number[][], s: string, w: number) => {
  c.beginPath(); c.moveTo(pts[0][0], pts[0][1]);
  for (let i = 1; i < pts.length; i++) c.lineTo(pts[i][0], pts[i][1]);
  c.strokeStyle = s; c.lineWidth = w; c.lineCap = "round"; c.stroke();
};
const R = (c: Ctx, x: number, y: number, w: number, h: number, f: string) => { c.fillStyle = f; c.fillRect(x, y, w, h); };

function hx(h: string): [number, number, number] {
  let s = (h || "#888888").replace("#", "");
  if (s.length === 3) s = s.split("").map((c) => c + c).join("");
  return [parseInt(s.slice(0, 2), 16), parseInt(s.slice(2, 4), 16), parseInt(s.slice(4, 6), 16)];
}
function shade(hex: string, f: number): string {
  const c = hx(hex);
  const v = f >= 1
    ? [c[0] + (255 - c[0]) * (f - 1), c[1] + (255 - c[1]) * (f - 1), c[2] + (255 - c[2]) * (f - 1)]
    : [c[0] * f, c[1] * f, c[2] * f];
  return `rgb(${v[0] | 0},${v[1] | 0},${v[2] | 0})`;
}

/** `ctx.roundRect` is not reliably present on WebKit2GTK, which Tauri uses on
 *  Linux — the same platform trap CLAUDE.md §8.11 records for `letterSpacing`. */
function roundRectPath(c: Ctx, x: number, y: number, w: number, h: number, r: number) {
  c.beginPath(); c.moveTo(x + r, y);
  c.arcTo(x + w, y, x + w, y + h, r); c.arcTo(x + w, y + h, x, y + h, r);
  c.arcTo(x, y + h, x, y, r); c.arcTo(x, y, x + w, y, r); c.closePath();
}

interface Facade {
  w: number; h: number; roof: string; wall: string; roofc: string;
  storeys: number; base: string; timber?: number; stone?: number;
}

const FACADES: Facade[] = [
  { w: 108, h: 128, roof: "gable", wall: "#8a5c46", roofc: "#7a3a34", storeys: 3, base: "shop" },
  { w: 132, h: 104, roof: "hip", wall: "#9c7a52", roofc: "#6a4a38", storeys: 2, base: "arcade" },
  { w: 86, h: 152, roof: "gable", wall: "#6e5a48", roofc: "#5a3a34", storeys: 4, base: "door", timber: 1 },
  { w: 152, h: 118, roof: "hip", wall: "#a89272", roofc: "#7a4a3a", storeys: 2, base: "arcade", stone: 1 },
  { w: 96, h: 168, roof: "tower", wall: "#9a8a70", roofc: "#5a5a52", storeys: 3, base: "door", stone: 1 },
  { w: 120, h: 96, roof: "shed", wall: "#7e6a50", roofc: "#6a4a3a", storeys: 1, base: "shop" },
  { w: 100, h: 136, roof: "gable", wall: "#8e6a4e", roofc: "#6a3a32", storeys: 3, base: "shop", timber: 1 },
];

function facadeArt(c: Ctx, spec: Facade, AH: number, lit: boolean) {
  const roofH = spec.roof === "tower" ? AH * 0.3 : AH * 0.22;
  const wallTop = roofH, wallH = AH - roofH;
  const wd = shade(spec.wall, 0.78), wl = shade(spec.wall, 1.14);
  // wall
  R(c, 0, wallTop, 100, wallH, spec.wall);
  R(c, 0, wallTop, 4, wallH, wl);
  R(c, 92, wallTop, 8, wallH, wd);
  if (spec.stone) for (let i = 0; i < 7; i++) { const y = wallTop + 4 + i * (wallH / 7); L(c, [[0, y], [100, y]], "rgba(30,20,12,0.16)", 1.6); }
  if (spec.timber) {
    for (const x of [14, 50, 86]) R(c, x - 3, wallTop, 6, wallH, "#5a4029");
    L(c, [[6, wallTop + wallH * 0.5], [94, wallTop + wallH * 0.48]], "#5a4029", 4);
  }
  // roof
  const rc = spec.roofc, rl = shade(rc, 1.2), rd = shade(rc, 0.72);
  if (spec.roof === "gable") { P(c, [[-6, wallTop + 3], [50, 0], [106, wallTop + 3]], rc); P(c, [[50, 0], [106, wallTop + 3], [50, wallTop + 3]], rd); }
  else if (spec.roof === "hip") { P(c, [[-6, wallTop + 3], [18, 0], [82, 0], [106, wallTop + 3]], rc); P(c, [[82, 0], [106, wallTop + 3], [50, wallTop + 3], [50, 0]], rd); }
  else if (spec.roof === "shed") { P(c, [[-6, wallTop + 3], [-4, 2], [104, wallTop - 10], [106, wallTop + 3]], rc); }
  else {
    R(c, 26, roofH * 0.42, 48, wallTop - roofH * 0.42, spec.wall);
    P(c, [[20, roofH * 0.46], [50, -4], [80, roofH * 0.46]], rc);
    P(c, [[50, -4], [80, roofH * 0.46], [50, roofH * 0.46]], rd);
    L(c, [[50, -4], [50, -16]], "#c9a227", 2.4); L(c, [[45, -11], [55, -11]], "#c9a227", 2);
  }
  R(c, -6, wallTop, 112, 5, rl);                     // eaves
  R(c, -6, wallTop + 4, 112, 2, "rgba(0,0,0,0.34)");
  // storeys of windows
  const n = spec.storeys, cols = spec.w > 130 ? 4 : spec.w > 96 ? 3 : 2;
  const baseH = 26, top = wallTop + 10, avail = wallH - 10 - baseH;
  for (let r = 0; r < n; r++) for (let i = 0; i < cols; i++) {
    const ww = 62 / cols, x = 19 + i * (62 / cols) + ww * 0.16, y = top + r * (avail / n) + 3;
    const hh = Math.min(16, avail / n - 7);
    R(c, x - 1.5, y - 1.5, ww * 0.68 + 3, hh + 3, "#2c2018");
    R(c, x, y, ww * 0.68, hh, lit && (r + i) % 3 === 0 ? "#e8bd6a" : "#2f3a46");
    if (spec.roof === "tower" || spec.stone) {
      c.beginPath(); c.arc(x + ww * 0.34, y, ww * 0.34, Math.PI, 0);
      c.fillStyle = lit && (r + i) % 3 === 0 ? "#e8bd6a" : "#2f3a46"; c.fill();
    }
  }
  // the ground storey the square actually touches
  const gy = AH - baseH;
  if (spec.base === "arcade") {
    for (let i = 0; i < 3; i++) {
      const x = 12 + i * 27; R(c, x, gy + 6, 20, baseH, "#241a12");
      c.beginPath(); c.arc(x + 10, gy + 6, 10, Math.PI, 0); c.fillStyle = "#241a12"; c.fill();
    }
  } else if (spec.base === "shop") {
    R(c, 8, gy + 4, 84, baseH, wd); R(c, 14, gy + 9, 30, baseH - 8, "#2b3038"); R(c, 52, gy + 9, 34, baseH - 8, "#241a12");
    R(c, 6, gy, 88, 5, spec.roofc);
  } else {
    R(c, 38, gy + 4, 24, baseH, "#241a12");
    c.beginPath(); c.arc(50, gy + 4, 12, Math.PI, 0); c.fillStyle = "#241a12"; c.fill();
  }
}

/** One front-elevation facade, in the set's pixel treatment. */
function facade(ctx: Ctx, cx: number, baseY: number, scale: number, spec: Facade, lit: boolean) {
  const w = spec.w * scale, h = spec.h * scale;
  pixelize(ctx, cx - w / 2, baseY - h, w, h, Math.max(14, Math.round(w / 3.4)), (c) => facadeArt(c, spec, 100 * h / w, lit));
}

export interface Stall {
  kit: KitSpec;
  /** [good id, tint] pairs laid out along the board. */
  goods: [string, string][];
  /** [good id, tint, label, price ×base] pinned over the stall. */
  chip?: [string, string, string, number];
}

/** A trestle stall: striped awning on poles, table, crates, wares, and its keeper. */
function stall(ctx: Ctx, x: number, w: number, awnY: number, tableY: number, k: KitSpec, goods: [string, string][], glow: number) {
  const K = resolveKit(k), a = K.trim, b = K.robe;
  // cast shadow on the paving
  ctx.fillStyle = "rgba(0,0,0,0.32)";
  ctx.beginPath(); ctx.ellipse(x + w / 2, tableY + 26, w * 0.58, 11, 0, 0, T2); ctx.fill();
  // poles
  for (const px of [x + 5, x + w - 5]) {
    ctx.fillStyle = "#2e2216"; ctx.fillRect(px - 3, awnY, 6, tableY + 24 - awnY);
    ctx.fillStyle = "#4a3823"; ctx.fillRect(px - 3, awnY, 2.5, tableY + 24 - awnY);
  }
  // the keeper stands behind the board, drawn AFTER the awning so the hem never
  // crops the head; sized so the head clears the scallops with air to spare
  const keeper = () => drawFigure(ctx, x + w / 2 - 21, tableY - 88, 42, k, { occasion: "everyday", cols: 28 });
  // awning — the stripes are the keeper's own culture colours
  const n = Math.max(5, Math.round(w / 30)), sw = w / n, dep = 18;
  for (let i = 0; i < n; i++) {
    ctx.fillStyle = i % 2 ? a : b;
    ctx.beginPath();
    ctx.moveTo(x + i * sw, awnY); ctx.lineTo(x + (i + 1) * sw, awnY);
    ctx.lineTo(x + (i + 1) * sw + 5, awnY + dep); ctx.lineTo(x + i * sw + 5, awnY + dep);
    ctx.closePath(); ctx.fill();
    ctx.beginPath(); ctx.arc(x + i * sw + sw / 2 + 5, awnY + dep, sw / 2, 0, Math.PI); ctx.fill();
  }
  ctx.fillStyle = "rgba(255,240,210,0.16)"; ctx.fillRect(x, awnY, w, 2.5);
  ctx.fillStyle = "rgba(0,0,0,0.32)"; ctx.fillRect(x + 5, awnY + dep - 2, w, 2);
  keeper();
  // the board and its cloth
  ctx.fillStyle = "#4a3520"; ctx.fillRect(x - 4, tableY, w + 8, 7);
  ctx.fillStyle = "#694d2c"; ctx.fillRect(x - 4, tableY, w + 8, 2.5);
  ctx.fillStyle = K.cloth2; ctx.fillRect(x, tableY + 7, w, 17);
  ctx.fillStyle = "rgba(255,255,255,0.10)"; ctx.fillRect(x, tableY + 7, w, 2);
  ctx.fillStyle = "rgba(0,0,0,0.32)"; ctx.fillRect(x, tableY + 22, w, 2);
  // crates and a sack beneath it
  ctx.fillStyle = "#33251a"; ctx.fillRect(x + 10, tableY + 24, 21, 18);
  ctx.fillStyle = "#4a3823"; ctx.fillRect(x + 10, tableY + 24, 21, 3);
  ctx.fillStyle = "#5a4a30"; ctx.beginPath(); ctx.ellipse(x + w - 20, tableY + 34, 12, 8, 0, 0, T2); ctx.fill();
  // the wares on the board
  const g = goods.length, step = g > 1 ? (w - 52) / (g - 1) : 0;
  goods.forEach((it, i) => drawIcon(ctx, x + 26 + i * step, tableY - 18, 36, it[1], it[0], { glow }));
}

/** The price chip pinned over a stall: the good, and what it goes for here. */
function chip(ctx: Ctx, cx: number, y: number, id: string, color: string, label: string, xw: number, glow: number) {
  ctx.font = "600 11px system-ui,sans-serif";
  const w = 34 + ctx.measureText(label).width + 34;
  const x = cx - w / 2;
  ctx.fillStyle = "rgba(9,15,24,0.86)";
  roundRectPath(ctx, x, y, w, 24, 12); ctx.fill();
  ctx.strokeStyle = "rgba(216,178,74,0.34)"; ctx.lineWidth = 1; ctx.stroke();
  drawIcon(ctx, x + 14, y + 12, 20, color, id, { glow });
  ctx.fillStyle = "#cfe2f6"; ctx.textBaseline = "middle"; ctx.textAlign = "left";
  ctx.fillText(label, x + 27, y + 12.5);
  ctx.fillStyle = xw > 1.3 ? "#e08080" : xw < 0.77 ? "#7fd0a0" : "#c0d0e0";
  ctx.font = "700 11px ui-monospace,Menlo,monospace";
  ctx.fillText(xw.toFixed(2) + "×", x + w - 32, y + 12.5);
}

// Where the crowd stands, and who stands there — the kit is stored per slot so
// no two neighbours (and no stall keeper) repeat.
const CROWD: number[][] = [
  [110, 214, 25, 12], [206, 209, 23, 5], [322, 215, 25, 9], [418, 210, 23, 16], [556, 214, 25, 2],
  [652, 209, 23, 11], [772, 216, 25, 7], [886, 211, 23, 14], [270, 212, 24, 1],
  [150, 386, 54, 7], [352, 380, 50, 14], [536, 388, 55, 3], [742, 382, 51, 9], [910, 386, 48, 5],
  [56, 382, 50, 12], [252, 390, 56, 16], [452, 384, 52, 10], [648, 390, 55, 1], [844, 384, 52, 8],
  [400, 382, 49, 13], [492, 211, 23, 15], [700, 388, 53, 2],
];

export interface MarketSquareOpts {
  stalls?: Stall[];
  /** 0..22 figures. */
  crowd?: number;
  flags?: boolean;
  glow?: number;
  chips?: boolean;
}

/**
 * The market square: the city behind, the stalls across the front, and the
 * crowd that actually buys — drawn from the same culture kits, so a port's
 * people are visible in its market.
 */
export function marketSquare(ctx: Ctx, W: number, H: number, opts: MarketSquareOpts = {}) {
  const HORIZON = 168;
  const g = ctx.createLinearGradient(0, 0, 0, HORIZON);
  g.addColorStop(0, "#14213a"); g.addColorStop(0.55, "#39465a"); g.addColorStop(1, "#6e5c43");
  ctx.fillStyle = g; ctx.fillRect(0, 0, W, HORIZON);
  const sg = ctx.createRadialGradient(W * 0.74, HORIZON - 10, 8, W * 0.74, HORIZON - 10, 340);
  sg.addColorStop(0, "rgba(240,196,112,0.55)"); sg.addColorStop(1, "rgba(240,196,112,0)");
  ctx.fillStyle = sg; ctx.fillRect(0, 0, W, HORIZON);

  // the city, two receding ranks of facades closing the square
  for (const [rank, alpha, sc, by] of [[0, 0.46, 0.62, 152], [1, 0.86, 0.84, 172]]) {
    ctx.save(); ctx.globalAlpha = alpha;
    const n = rank ? 7 : 9;
    for (let i = 0; i < n; i++) {
      const spec = FACADES[(i * 5 + rank * 3) % FACADES.length];
      const x = 24 + i * ((W - 30) / (n - 1)) + (rank ? 52 : 0);
      facade(ctx, x, by, sc, spec, rank === 1);
      if (opts.flags !== false && i % 3 === 1) flag(ctx, x, by - spec.h * sc - 6, 16, "#c1553f");
    }
    ctx.restore();
  }

  // the paving: warm stone flags, foreshortened towards the viewer
  const pg = ctx.createLinearGradient(0, HORIZON, 0, H);
  pg.addColorStop(0, "#7a6a50"); pg.addColorStop(0.4, "#5b4d3a"); pg.addColorStop(1, "#3c3227");
  ctx.fillStyle = pg; ctx.fillRect(0, HORIZON, W, H - HORIZON);
  ctx.fillStyle = "rgba(0,0,0,0.40)"; ctx.fillRect(0, HORIZON, W, 5);
  // courses, and the flags within each course converging on the horizon
  ctx.save();
  ctx.beginPath(); ctx.rect(0, HORIZON + 5, W, H - HORIZON - 5); ctx.clip();
  const VPX = W / 2;
  // flag joints first: true radials from the vanishing point
  for (let i = -22; i <= 22; i++) {
    if (!i) continue;
    const xb = VPX + i * 96;
    ctx.strokeStyle = "rgba(24,16,9,0.34)"; ctx.lineWidth = 2;
    ctx.beginPath(); ctx.moveTo(VPX + (xb - VPX) * 0.16, HORIZON + 5); ctx.lineTo(xb, H); ctx.stroke();
    ctx.strokeStyle = "rgba(255,238,204,0.06)"; ctx.lineWidth = 1.4;
    ctx.beginPath(); ctx.moveTo(VPX + (xb - VPX) * 0.16 + 2, HORIZON + 5); ctx.lineTo(xb + 5, H); ctx.stroke();
  }
  // courses, kept quiet so they read as joints between flags, not planks
  for (let i = 0; i < 13; i++) {
    const t = (i + 1) / 13, y = HORIZON + 5 + Math.pow(t, 1.85) * (H - HORIZON - 5);
    ctx.strokeStyle = "rgba(24,16,9,0.15)"; ctx.lineWidth = Math.max(1, t * 1.6);
    ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(W, y); ctx.stroke();
  }
  ctx.restore();

  const stalls = opts.stalls || [];
  const sw = 196, gap = (W - 70 - stalls.length * sw) / Math.max(1, stalls.length - 1);
  const sx = (i: number) => 35 + i * (sw + gap);
  const glow = opts.glow ?? 1;

  // crowd behind and between the stalls
  const crowd = Math.max(0, Math.min(CROWD.length, opts.crowd ?? 16));
  const seen = CROWD.slice(0, crowd);
  for (const [x, y, w, kit] of seen.filter((s) => s[1] < 300)) {
    ctx.save(); ctx.globalAlpha = 0.78;
    ctx.fillStyle = "rgba(0,0,0,0.34)";
    ctx.beginPath(); ctx.ellipse(x, y + 2, w * 0.42, w * 0.12, 0, 0, T2); ctx.fill();
    drawFigure(ctx, x - w / 2, y - w * 2.1, w, kit, { occasion: kit % 5 === 0 ? "ceremonial" : "everyday", cols: 22 });
    ctx.restore();
  }

  stalls.forEach((s, i) => stall(ctx, sx(i), sw, 196, 292, s.kit, s.goods, glow));

  // the near rank walks in front of the stalls
  for (const [x, y, w, kit] of seen.filter((s) => s[1] >= 300)) {
    ctx.fillStyle = "rgba(0,0,0,0.42)";
    ctx.beginPath(); ctx.ellipse(x, y - 2, w * 0.44, w * 0.13, 0, 0, T2); ctx.fill();
    drawFigure(ctx, x - w / 2, y - w * 2.1, w, kit, { occasion: kit % 7 === 0 ? "ceremonial" : "everyday", cols: 30 });
  }

  // the prices, pinned over the stall they belong to
  if (opts.chips !== false) {
    stalls.forEach((s, i) => {
      if (!s.chip) return;
      chip(ctx, sx(i) + sw / 2, 146, s.chip[0], s.chip[1], s.chip[2], s.chip[3], glow);
    });
  }
}
