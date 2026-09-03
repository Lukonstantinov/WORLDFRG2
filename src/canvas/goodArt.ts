// Trade-good ARTWORK. Every good draws from its OWN recipe — a shaded vector
// illustration on a 100x100 art box — so no two goods share a picture.
// Coordinates are 0..100; the renderer scales to the requested size.
//
// Two finished treatments sit on top of the illustrations:
//   drawIcon           pixel-art (hard dark edge + one-pixel bevel) — dense panels
//   drawIconVictorian  a Victoria II ledger card (aged paper, bronze frame)
//
// This is a SEPARATE module from `goodIcons.ts`, which keeps the EU4-style map
// medallion the world overlay draws at arbitrary zoom. Different surface,
// different treatment; neither should be made to serve the other.

const T2 = Math.PI * 2;

type Pt = number[];

/** Recipe variation bag. Every field is optional; a family reads only its own. */
export interface GoodParams {
  n?: number; droop?: number; awn?: number; kr?: number; kw?: number; leaf?: number;
  rows?: number; taper?: number; pos?: Pt[]; oblate?: number; segments?: number;
  neck?: number; belly?: number; foot?: number; top?: number; bot?: number;
  nk?: number; bel?: number; lip?: number; handles?: number; cork?: number; stem?: number;
  hoops?: number; fill?: number;
  kind?: string; pattern?: number; cut?: string; palm?: number;
  depth?: number; y?: number; dorsal?: number; stripes?: number; spots?: number; dried?: number;
  fur?: number; second?: number; petals?: number; clasp?: number; scoop?: number;
  veins?: number; inclusion?: number; grains?: number; spill?: Pt[]; round?: boolean;
}

/** The shaded colour set a family draws with, derived from the good's tint. */
interface Cols {
  base: string; lt: string; hi: string; dk: string; stem: string; leaf: string;
}

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
export function lum(hex: string): number {
  const c = hx(hex);
  return (0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]) / 255;
}

// ── primitives ───────────────────────────────────────────────────────────────
type Ctx = CanvasRenderingContext2D;
type Paint = string | CanvasGradient;

function P(ctx: Ctx, pts: Pt[], fill?: Paint, close = true) {
  ctx.beginPath(); ctx.moveTo(pts[0][0], pts[0][1]);
  for (let i = 1; i < pts.length; i++) {
    const p = pts[i];
    if (p.length === 4) ctx.quadraticCurveTo(p[0], p[1], p[2], p[3]); else ctx.lineTo(p[0], p[1]);
  }
  if (close) ctx.closePath();
  if (fill) { ctx.fillStyle = fill; ctx.fill(); }
}
function E(ctx: Ctx, x: number, y: number, rx: number, ry: number, rot: number, fill: Paint) {
  ctx.beginPath(); ctx.ellipse(x, y, rx, ry, rot || 0, 0, T2); ctx.fillStyle = fill; ctx.fill();
}
function L(ctx: Ctx, pts: Pt[], stroke: Paint, w: number, cap: CanvasLineCap = "round") {
  ctx.beginPath(); ctx.moveTo(pts[0][0], pts[0][1]);
  for (let i = 1; i < pts.length; i++) {
    const p = pts[i];
    if (p.length === 4) ctx.quadraticCurveTo(p[0], p[1], p[2], p[3]); else ctx.lineTo(p[0], p[1]);
  }
  ctx.strokeStyle = stroke; ctx.lineWidth = w; ctx.lineCap = cap; ctx.lineJoin = "round"; ctx.stroke();
}
function lg(ctx: Ctx, x0: number, y0: number, x1: number, y1: number, stops: [number, string][]) {
  const g = ctx.createLinearGradient(x0, y0, x1, y1);
  stops.forEach(([o, c]) => g.addColorStop(o, c));
  return g;
}
function rg(ctx: Ctx, x: number, y: number, r0: number, r1: number, stops: [number, string][]) {
  const g = ctx.createRadialGradient(x - r1 * 0.25, y - r1 * 0.3, r0, x, y, r1);
  stops.forEach(([o, c]) => g.addColorStop(o, c));
  return g;
}
/** A shaded volume fill for a good's base colour. */
function vol(ctx: Ctx, x: number, y: number, r: number, C: { base: string; lt: string; dk: string }) {
  return rg(ctx, x, y, r * 0.1, r * 1.15, [[0, C.lt], [0.45, C.base], [1, C.dk]]);
}

// ── shape families. Each takes (ctx, C, p) with p carrying the variation. ────

/** Cereal ear: stalk + kernel rows. Grain species vary by kernel shape, awns, droop. */
function ear(ctx: Ctx, C: Cols, p: GoodParams) {
  const n = p.n || 5, drp = p.droop || 0, aw = p.awn || 0, kr = p.kr || 7, kw = p.kw || 4.5;
  const topY = 22 + drp * 10, tipX = 50 + drp * 16;
  L(ctx, [[46, 92], [48, 70, 50, 46], [52, 32, tipX - drp * 4, topY]], C.stem, 4.5);
  if (p.leaf) P(ctx, [[47, 72], [26, 62, 30, 80], [40, 86, 47, 80]], C.leaf || C.stem);
  for (let i = 0; i < n; i++) {
    const t = i / (n - 1 || 1), y = topY + 8 + t * (64 - topY), x = 50 + drp * (1 - t) * 14;
    for (const d of [-1, 1]) {
      const kx = x + d * kw, ky = y - 2;
      E(ctx, kx, ky, kr * 0.42, kr * 0.62, d * 0.55, i % 2 ? C.base : C.lt);
      E(ctx, kx - d * 1, ky - 1.5, kr * 0.2, kr * 0.3, d * 0.55, C.hi);
      if (aw) L(ctx, [[kx, ky - kr * 0.5], [kx + d * (6 + aw * 10), ky - kr * 1.4 - aw * 18]], C.dk, 1.4);
    }
  }
  E(ctx, 50 + drp * 14, topY, kr * 0.4, kr * 0.7, 0, C.lt);
}

/** A head/panicle of small round grains (millet, sorghum). */
function panicle(ctx: Ctx, C: Cols, p: GoodParams) {
  L(ctx, [[50, 92], [50, 60, 50, 40]], C.stem, 4.5);
  P(ctx, [[50, 70], [28, 64, 34, 82], [46, 84, 50, 78]], C.leaf || C.stem);
  const rows = p.rows || 7;
  for (let r = 0; r < rows; r++) {
    const t = r / (rows - 1), y = 20 + t * 36, w = (p.taper ? (1 - Math.abs(t - 0.35)) * 1.3 : 1) * (16 - t * 4);
    const cnt = Math.max(2, Math.round(w / 4));
    for (let i = 0; i < cnt; i++) {
      const x = 50 - w / 2 + (cnt > 1 ? i * (w / (cnt - 1)) : 0);
      E(ctx, x, y, 3.1, 3.1, 0, (i + r) % 2 ? C.base : C.lt);
      E(ctx, x - 1, y - 1, 1.2, 1.2, 0, C.hi);
    }
  }
}

/** Rounded fruit / berry cluster. */
function fruit(ctx: Ctx, C: Cols, p: GoodParams) {
  const n = p.n || 1;
  const pos: Pt[] = p.pos || (n === 1 ? [[50, 54, 26]]
    : n === 3 ? [[36, 46, 17], [64, 46, 17], [50, 70, 18]]
    : [[38, 44, 14], [62, 44, 14], [30, 66, 13], [50, 64, 15], [70, 66, 13]]);
  pos.forEach(([x, y, r]) => {
    E(ctx, x, y, r, r * (p.oblate || 1), 0, vol(ctx, x, y, r, C));
    E(ctx, x - r * 0.32, y - r * 0.38, r * 0.28, r * 0.2, -0.6, "rgba(255,255,255,0.5)");
    if (p.segments) {
      for (let i = 0; i < 7; i++) {
        const a = i / 7 * T2;
        L(ctx, [[x, y], [x + Math.cos(a) * r * 0.86, y + Math.sin(a) * r * 0.86]], C.hi, 1.4);
      }
      E(ctx, x, y, r * 0.16, r * 0.16, 0, C.hi);
    }
  });
  if (p.leaf) {
    P(ctx, [[52, 30], [70, 10, 80, 26], [62, 36, 52, 30]], C.leaf || "#5f8a3e");
    L(ctx, [[50, 34], [50, 22]], C.stem || "#5d4022", 4);
  }
}

/** Vessel profile: amphora, jar, bottle, goblet, barrel — one shape, many waists. */
function vessel(ctx: Ctx, C: Cols, p: GoodParams) {
  const nk = p.neck ?? 10, bel = p.belly ?? 32, foot = p.foot ?? 12, top = p.top ?? 16, bot = p.bot ?? 86;
  const body: Pt[] = [[50 - nk, top], [50 - bel, top + 18, 50 - bel * 0.9, bot - 22], [50 - foot, bot],
    [50 + foot, bot], [50 + bel * 0.9, bot - 22, 50 + bel, top + 18], [50 + nk, top]];
  P(ctx, body, lg(ctx, 50 - bel, 0, 50 + bel, 0, [[0, C.dk], [0.4, C.base], [0.72, C.lt], [1, C.dk]]));
  if (p.lip) P(ctx, [[50 - nk - 5, top], [50 + nk + 5, top], [50 + nk + 3, top + 6], [50 - nk - 3, top + 6]], C.lt);
  if (p.handles) for (const d of [-1, 1]) L(ctx, [[50 + d * nk, top + 5], [50 + d * (bel + 10), top + 14, 50 + d * bel * 0.8, top + 30]], C.dk, 5);
  if (p.cork) P(ctx, [[50 - nk * 0.7, top], [50 + nk * 0.7, top], [50 + nk * 0.7, top - 9], [50 - nk * 0.7, top - 9]], "#8a6a42");
  if (p.stem) {
    P(ctx, [[46, bot - 2], [54, bot - 2], [54, bot + 6], [46, bot + 6]], C.dk);
    P(ctx, [[36, bot + 6], [64, bot + 6], [66, bot + 11], [34, bot + 11]], C.base);
  }
  if (p.hoops) for (const y of [top + 16, (top + bot) / 2, bot - 14]) L(ctx, [[50 - bel * 0.96, y], [50 + bel * 0.96, y]], "rgba(40,28,16,0.55)", 3.5, "butt");
  if (p.fill != null) {
    const fy = top + 8 + (1 - p.fill) * (bot - top - 20);
    P(ctx, [[50 - bel * 0.8, fy], [50 + bel * 0.8, fy], [50 + bel * 0.8, fy + 6], [50 - bel * 0.8, fy + 6]], C.hi);
  }
  L(ctx, [[50 - bel * 0.55, top + 22], [50 - bel * 0.75, (top + bot) / 2]], "rgba(255,255,255,0.35)", 4);
}

/** Cloth: bolt, folded stack, rolled carpet, hank of yarn. */
function cloth(ctx: Ctx, C: Cols, p: GoodParams) {
  if (p.kind === "rug") {
    E(ctx, 50, 32, 27, 13, 0, C.lt);
    P(ctx, [[23, 32], [77, 32], [77, 70], [23, 70]], lg(ctx, 23, 0, 77, 0, [[0, C.dk], [0.45, C.base], [1, C.dk]]));
    E(ctx, 50, 32, 27, 13, 0, C.lt); E(ctx, 50, 32, 13, 6, 0, C.dk);
    for (let i = 0; i < 3; i++) {
      const y = 42 + i * 10;
      for (let k = 0; k < 5; k++) P(ctx, [[28 + k * 11, y], [33 + k * 11, y + 5], [28 + k * 11, y + 10], [23 + k * 11, y + 5]], k % 2 ? C.hi : C.lt);
    }
    for (let i = 0; i < 10; i++) L(ctx, [[25 + i * 5.6, 70], [25 + i * 5.6, 82]], C.lt, 2.4);
  } else if (p.kind === "roll") {
    E(ctx, 50, 36, 26, 12, 0, C.lt);
    P(ctx, [[24, 36], [76, 36], [76, 74], [24, 74]], lg(ctx, 24, 0, 76, 0, [[0, C.dk], [0.45, C.base], [1, C.dk]]));
    E(ctx, 50, 74, 26, 12, 0, C.base);
    E(ctx, 50, 36, 26, 12, 0, C.lt); E(ctx, 50, 36, 13, 6, 0, C.dk);
    for (let i = 0; i < 4; i++) L(ctx, [[30 + i * 13, 40], [30 + i * 13, 72]], "rgba(255,255,255,0.14)", 3);
  } else if (p.kind === "stack") {
    for (let i = 0; i < 3; i++) {
      const y = 44 + i * 15;
      P(ctx, [[22, y], [78, y], [74, y + 13], [26, y + 13]], i % 2 ? C.base : C.lt);
      L(ctx, [[26, y + 13], [74, y + 13]], C.dk, 2);
    }
    P(ctx, [[26, 32], [74, 32], [78, 44], [22, 44]], C.lt);
  } else if (p.kind === "hank") {
    for (let i = 0; i < 3; i++) { const x = 38 + i * 12; E(ctx, x, 54, 9, 30, 0, i === 1 ? C.lt : C.base); }
    P(ctx, [[30, 44], [70, 44], [70, 60], [30, 60]], C.dk);
    L(ctx, [[34, 46], [66, 46]], "rgba(255,255,255,0.25)", 3);
  } else { // bolt over a board
    P(ctx, [[20, 58], [80, 42], [80, 72], [20, 88]], lg(ctx, 20, 0, 80, 0, [[0, C.dk], [0.5, C.base], [1, C.lt]]));
    P(ctx, [[20, 58], [80, 42], [80, 50], [20, 66]], C.lt);
    for (let i = 0; i < 5; i++) L(ctx, [[24 + i * 13, 60 + (5 - i) * 3], [24 + i * 13, 88 - i * 3]], "rgba(0,0,0,0.12)", 3);
    if (p.pattern) for (let i = 0; i < 4; i++) L(ctx, [[22 + i * 15, 63 + (4 - i) * 3.6], [30 + i * 15, 61 + (4 - i) * 3.6]], C.hi, 2.5);
  }
}

/** Metal: ingot stack, coin pile, wire coil, worked ware. */
function metal(ctx: Ctx, C: Cols, p: GoodParams) {
  if (p.kind === "coins") {
    for (let i = 0; i < 5; i++) {
      const y = 74 - i * 7, x = 50 + (i % 2 ? 3 : -3);
      E(ctx, x, y, 24, 9, 0, i === 4 ? C.lt : C.base); E(ctx, x, y - 2, 24, 9, 0, C.lt); E(ctx, x, y - 2, 15, 5.5, 0, C.dk);
    }
    E(ctx, 26, 40, 15, 15, 0, vol(ctx, 26, 40, 15, C)); E(ctx, 26, 40, 9, 9, 0, C.dk);
  } else if (p.kind === "ware") {
    P(ctx, [[50, 14], [57, 32], [74, 34], [60, 48], [64, 68], [50, 58], [36, 68], [40, 48], [26, 34], [43, 32]], vol(ctx, 50, 44, 32, C));
    L(ctx, [[50, 20], [50, 52]], "rgba(255,255,255,0.35)", 3);
  } else if (p.kind === "sheet") {                 // Tin: rolled sheet + snips
    P(ctx, [[18, 52], [62, 36], [84, 44], [40, 62]], lg(ctx, 18, 0, 84, 0, [[0, C.lt], [0.5, C.base], [1, C.dk]]));
    P(ctx, [[18, 52], [40, 62], [40, 72], [18, 62]], C.dk);
    P(ctx, [[40, 62], [84, 44], [84, 54], [40, 72]], C.base);
    for (let i = 0; i < 4; i++) L(ctx, [[26 + i * 13, 54 - i * 2], [30 + i * 13, 66 - i * 2]], "rgba(255,255,255,0.28)", 1.6);
    E(ctx, 70, 72, 12, 5, -0.25, C.dk);
  } else if (p.kind === "pigs") {                  // Lead: squat pigs + a spill
    for (let i = 0; i < 2; i++) {
      const y = 74 - i * 15;
      P(ctx, [[24, y - 11], [70, y - 11], [76, y], [18, y]], lg(ctx, 18, 0, 76, 0, [[0, C.dk], [0.5, C.base], [1, C.lt]]));
      P(ctx, [[24, y - 11], [70, y - 11], [64, y - 17], [30, y - 17]], C.lt);
    }
    P(ctx, [[30, 42], [62, 42], [58, 34], [34, 34]], C.base);
    for (const [x, y, r] of [[36, 28, 4], [52, 26, 3.4]]) E(ctx, x, y, r, r * 0.7, 0, C.hi);
  } else if (p.kind === "plate") {                 // Silver: chased plate + goblet
    E(ctx, 44, 58, 30, 22, 0, rg(ctx, 44, 58, 4, 32, [[0, C.hi], [0.5, C.lt], [1, C.dk]]));
    E(ctx, 44, 56, 20, 14, 0, C.base);
    ctx.beginPath(); ctx.ellipse(44, 58, 26, 19, 0, 0, T2); ctx.strokeStyle = C.dk; ctx.lineWidth = 2; ctx.stroke();
    P(ctx, [[66, 26], [86, 26], [83, 42], [69, 42]], C.lt);
    P(ctx, [[74, 42], [78, 42], [78, 52], [74, 52]], C.dk);
    P(ctx, [[68, 52], [84, 52], [86, 57], [66, 57]], C.base);
    E(ctx, 34, 50, 9, 5, -0.5, "rgba(255,255,255,0.45)");
  } else if (p.kind === "ore") {
    P(ctx, [[24, 68], [34, 40], [54, 32], [72, 46], [76, 72], [50, 84]], lg(ctx, 24, 32, 76, 84, [[0, C.lt], [0.5, C.base], [1, C.dk]]));
    P(ctx, [[34, 40], [54, 32], [52, 52], [36, 58]], C.lt);
    for (const [a, b] of [[[42, 62], [58, 50]], [[48, 74], [66, 60]]]) L(ctx, [a, b], C.hi, 4);
  } else {
    const n = p.n || 3;
    for (let i = 0; i < n; i++) {
      const y = 76 - i * 16, w = 30 - i * 3;
      P(ctx, [[50 - w, y - 12], [50 + w, y - 12], [50 + w + 5, y], [50 - w - 5, y]], lg(ctx, 50 - w, 0, 50 + w, 0, [[0, C.dk], [0.45, C.base], [1, C.lt]]));
      P(ctx, [[50 - w, y - 12], [50 + w, y - 12], [50 + w * 0.8, y - 17], [50 - w * 0.8, y - 17]], C.lt);
    }
  }
}

/** A small cross-flare specular sparkle — the glint a cut gem actually throws
 *  under raking light, which a flat highlight ellipse can't fake. */
function sparkle(ctx: Ctx, x: number, y: number, r: number, c: string) {
  ctx.save(); ctx.globalCompositeOperation = "lighter";
  ctx.strokeStyle = c; ctx.lineWidth = 1.5; ctx.lineCap = "round";
  ctx.beginPath(); ctx.moveTo(x - r, y); ctx.lineTo(x + r, y); ctx.moveTo(x, y - r); ctx.lineTo(x, y + r); ctx.stroke();
  ctx.globalAlpha = 0.55; ctx.lineWidth = 0.7;
  ctx.beginPath(); ctx.moveTo(x - r * 0.68, y - r * 0.68); ctx.lineTo(x + r * 0.68, y + r * 0.68);
  ctx.moveTo(x - r * 0.68, y + r * 0.68); ctx.lineTo(x + r * 0.68, y - r * 0.68); ctx.stroke();
  ctx.restore();
}

/** Cut gemstone. `cut` changes the facet plan so every stone is its own picture. */
function gemstone(ctx: Ctx, C: Cols, p: GoodParams) {
  const cut = p.cut || "brilliant";
  const F = (pts: Pt[], f: Paint) => P(ctx, pts, f);
  if (cut === "brilliant") {
    F([[50, 16], [80, 42], [50, 86], [20, 42]], C.base);
    F([[50, 16], [80, 42], [50, 50]], C.lt); F([[50, 16], [20, 42], [50, 50]], C.hi);
    F([[20, 42], [50, 86], [50, 50]], C.dk); F([[80, 42], [50, 86], [50, 50]], C.base);
  } else if (cut === "emerald") {
    F([[30, 20], [70, 20], [80, 34], [80, 72], [70, 86], [30, 86], [20, 72], [20, 34]], C.base);
    F([[34, 26], [66, 26], [72, 36], [72, 70], [66, 80], [34, 80], [28, 70], [28, 36]], C.lt);
    F([[34, 26], [66, 26], [62, 40], [38, 40]], C.hi);
    F([[38, 66], [62, 66], [66, 80], [34, 80]], C.dk);
  } else if (cut === "cushion") {
    F([[50, 16], [76, 30], [84, 56], [62, 84], [38, 84], [16, 56], [24, 30]], C.base);
    F([[50, 16], [76, 30], [50, 52], [24, 30]], C.hi);
    F([[24, 30], [50, 52], [16, 56]], C.lt);
    F([[50, 52], [62, 84], [38, 84]], C.dk);
  } else if (cut === "pear") {
    F([[50, 14], [74, 50], [62, 84], [38, 84], [26, 50]], C.base);
    F([[50, 14], [74, 50], [50, 54], [26, 50]], C.hi);
    F([[26, 50], [50, 54], [38, 84]], C.lt);
    F([[50, 54], [74, 50], [62, 84], [38, 84]], C.dk);
    F([[50, 14], [62, 42], [50, 46], [38, 42]], "rgba(255,255,255,0.5)");
  } else if (cut === "marquise") {
    F([[50, 12], [74, 50], [50, 88], [26, 50]], C.base);
    F([[50, 12], [74, 50], [50, 50]], C.hi);
    F([[50, 12], [26, 50], [50, 50]], C.lt);
    F([[26, 50], [50, 88], [50, 50]], C.dk);
    L(ctx, [[50, 12], [50, 88]], C.hi, 2);
  } else if (cut === "trilliant") {
    F([[50, 16], [84, 74], [16, 74]], C.base);
    F([[50, 16], [67, 45], [33, 45]], C.hi);
    F([[33, 45], [67, 45], [84, 74], [16, 74]], C.lt);
    F([[50, 45], [84, 74], [16, 74]], C.dk);
    L(ctx, [[33, 45], [67, 45]], C.hi, 2);
  } else if (cut === "cabochon") {
    E(ctx, 50, 54, 30, 26, 0, vol(ctx, 50, 54, 30, C));
    E(ctx, 40, 42, 12, 7, -0.6, "rgba(255,255,255,0.55)");
  } else { // rough / uncut
    F([[26, 60], [36, 28], [58, 20], [76, 40], [72, 74], [44, 84]], lg(ctx, 26, 20, 76, 84, [[0, C.lt], [0.55, C.base], [1, C.dk]]));
    F([[36, 28], [58, 20], [56, 46], [38, 50]], C.hi);
  }
  E(ctx, 38, 34, 7, 4, -0.6, "rgba(255,255,255,0.6)");
  sparkle(ctx, 38, 34, 7, "#ffffff");
  sparkle(ctx, 64, 60, 3.4, "rgba(255,255,255,0.75)");
}

/** Fish body — species vary by depth, fins, tail and markings. */
function fish(ctx: Ctx, C: Cols, p: GoodParams) {
  const d = p.depth || 20, y = p.y || 52;
  P(ctx, [[24, y], [46, y - d, 72, y - d * 0.55], [84, y], [72, y + d * 0.55, 46, y + d], [24, y]],
    lg(ctx, 0, y - d, 0, y + d, [[0, C.lt], [0.5, C.base], [1, C.dk]]));
  P(ctx, [[26, y], [10, y - d * 0.9], [12, y], [10, y + d * 0.9]], C.dk);           // tail
  if (p.dorsal) P(ctx, [[48, y - d * 0.85], [60, y - d * 1.7], [68, y - d * 0.7]], C.dk);
  P(ctx, [[52, y + d * 0.5], [62, y + d * 1.15], [68, y + d * 0.35]], C.dk);        // pelvic
  E(ctx, 76, y - d * 0.18, 3.2, 3.2, 0, "#14202a"); E(ctx, 77, y - d * 0.3, 1.2, 1.2, 0, "#fff");
  if (p.stripes) for (let i = 0; i < 4; i++) L(ctx, [[40 + i * 10, y - d * 0.7], [40 + i * 10, y + d * 0.7]], "rgba(255,255,255,0.25)", 3);
  if (p.spots) for (const [sx, sy] of [[52, y - 6], [62, y + 3], [44, y + 5]]) E(ctx, sx, sy, 3, 3, 0, "rgba(20,30,40,0.35)");
  if (p.dried) { L(ctx, [[70, y - d * 0.6], [70, y - d * 1.9]], "#9a8a6a", 2.5); E(ctx, 70, y - d * 2, 3, 3, 0, "#c9b78e"); }
}

/** Cetacean. */
function whale(ctx: Ctx, C: Cols) {
  P(ctx, [[16, 58], [40, 26, 70, 40], [86, 52], [70, 66, 40, 74], [16, 58]], lg(ctx, 0, 26, 0, 74, [[0, C.lt], [0.55, C.base], [1, C.dk]]));
  P(ctx, [[84, 50], [96, 34], [92, 56]], C.dk);
  P(ctx, [[52, 42], [62, 26], [64, 44]], C.dk);
  P(ctx, [[44, 66], [54, 80], [62, 64]], C.dk);
  E(ctx, 30, 54, 2.6, 2.6, 0, "#14202a");
  for (let i = 0; i < 5; i++) L(ctx, [[22 + i * 6, 64], [24 + i * 6, 72]], "rgba(255,255,255,0.2)", 2.5);
  L(ctx, [[46, 34], [40, 18, 48, 10]], "rgba(200,225,240,0.7)", 4);
}

/** Land animal silhouettes. */
function beast(ctx: Ctx, C: Cols, p: GoodParams) {
  if (p.kind === "horse") {
    P(ctx, [[26, 54], [44, 40, 62, 48], [70, 52], [76, 44, 80, 30], [86, 26], [84, 38], [76, 52], [70, 64], [60, 68], [34, 66], [26, 54]],
      lg(ctx, 0, 26, 0, 80, [[0, C.lt], [0.55, C.base], [1, C.dk]]));
    for (const x of [34, 44, 60, 68]) P(ctx, [[x, 64], [x + 6, 64], [x + 5, 86], [x - 1, 86]], C.dk);
    P(ctx, [[26, 54], [12, 66], [22, 66]], C.dk);
    P(ctx, [[80, 28], [84, 18], [86, 28]], C.dk);
    L(ctx, [[70, 34], [62, 46, 56, 56]], C.dk, 5);
  } else if (p.kind === "sheep") {
    E(ctx, 48, 54, 28, 20, 0, C.lt);
    for (const [x, y, r] of [[28, 46, 11], [42, 38, 12], [60, 40, 12], [70, 52, 11], [34, 64, 10], [54, 68, 11]]) E(ctx, x, y, r, r, 0, r > 11 ? C.lt : C.base);
    E(ctx, 76, 44, 11, 9, 0.4, C.dk); E(ctx, 80, 42, 2.2, 2.2, 0, "#14202a");
    for (const x of [36, 50, 62]) P(ctx, [[x, 70], [x + 5, 70], [x + 4, 84], [x - 1, 84]], C.dk);
  } else { // hide / pelt stretched
    P(ctx, [[50, 14], [68, 22], [86, 20], [74, 44], [80, 72], [62, 88], [38, 88], [20, 72], [26, 44], [14, 20], [32, 22]],
      rg(ctx, 50, 50, 6, 44, [[0, C.lt], [0.6, C.base], [1, C.dk]]));
    if (p.spots) for (const [x, y, r] of [[42, 44, 7], [60, 54, 6], [48, 68, 5]]) E(ctx, x, y, r, r * 0.8, 0.3, C.dk);
    if (p.fur) for (let i = 0; i < 12; i++) {
      const a = i / 12 * T2;
      L(ctx, [[50 + Math.cos(a) * 30, 50 + Math.sin(a) * 32], [50 + Math.cos(a) * 36, 50 + Math.sin(a) * 38]], C.dk, 2.5);
    }
  }
}

/** Leaves / pods / bark rolls / threads — the aromatics family. */
function botanical(ctx: Ctx, C: Cols, p: GoodParams) {
  if (p.kind === "leaf") {
    P(ctx, [[26, 78], [18, 34, 52, 20], [84, 30], [74, 68, 26, 78]], lg(ctx, 26, 20, 84, 78, [[0, C.lt], [0.6, C.base], [1, C.dk]]));
    L(ctx, [[28, 76], [50, 50, 78, 34]], C.dk, 3);
    for (let i = 1; i < 5; i++) L(ctx, [[30 + i * 10, 72 - i * 8], [36 + i * 10, 58 - i * 6]], C.dk, 2);
    if (p.second) P(ctx, [[36, 84], [44, 60, 72, 58]], C.base, false);
  } else if (p.kind === "bark") {
    for (let i = 0; i < 3; i++) {
      const x = 32 + i * 18;
      P(ctx, [[x - 9, 18], [x + 9, 18], [x + 9, 84], [x - 9, 84]], lg(ctx, x - 9, 0, x + 9, 0, [[0, C.dk], [0.4, C.base], [1, C.lt]]));
      E(ctx, x, 18, 9, 3.4, 0, C.lt); E(ctx, x, 18, 5, 1.9, 0, C.dk);
    }
  } else if (p.kind === "buds") {
    for (const [x, y, a] of [[34, 60, -0.5], [52, 70, 0.2], [66, 52, 0.7], [46, 42, -0.2]]) {
      L(ctx, [[x, y], [x + Math.cos(a) * 16, y - 16]], C.dk, 4.5);
      E(ctx, x + Math.cos(a) * 18, y - 19, 6, 6.5, a, C.base);
      for (let i = 0; i < 4; i++) {
        const b = a + i * 1.4;
        L(ctx, [[x + Math.cos(a) * 18, y - 23], [x + Math.cos(a) * 18 + Math.cos(b) * 7, y - 26 + Math.sin(b) * 5]], C.lt, 2);
      }
    }
  } else if (p.kind === "corns") {
    for (const [x, y, r] of [[38, 44, 10], [60, 40, 9], [50, 60, 11], [30, 64, 8], [68, 62, 9], [50, 80, 7]]) {
      E(ctx, x, y, r, r, 0, vol(ctx, x, y, r, C));
      E(ctx, x - r * 0.3, y - r * 0.35, r * 0.25, r * 0.18, -0.6, "rgba(255,255,255,0.35)");
      L(ctx, [[x - r * 0.5, y + r * 0.3], [x + r * 0.4, y - r * 0.2]], C.dk, 1.6);
    }
  } else if (p.kind === "threads") {
    for (let i = 0; i < 9; i++) {
      const x = 26 + i * 6, s = (i % 2 ? 1 : -1);
      L(ctx, [[x, 74], [x + s * 8, 54, x - s * 4, 28]], i % 3 ? C.base : C.lt, 3.2);
    }
    E(ctx, 50, 80, 26, 7, 0, C.dk);
  } else if (p.kind === "flower") {
    for (let i = 0; i < (p.petals || 6); i++) {
      const a = i / (p.petals || 6) * T2;
      E(ctx, 50 + Math.cos(a) * 19, 52 + Math.sin(a) * 19, 13, 8, a, i % 2 ? C.base : C.lt);
    }
    E(ctx, 50, 52, 10, 10, 0, C.hi);
    L(ctx, [[50, 62], [50, 88]], C.stem || "#4e7a3a", 4);
  } else if (p.kind === "cane") {
    for (let i = 0; i < 4; i++) {
      const x = 30 + i * 13, t = i % 2;
      P(ctx, [[x - 5, 20 + t * 6], [x + 5, 20 + t * 6], [x + 5, 86], [x - 5, 86]], i % 2 ? C.base : C.lt);
      for (let k = 0; k < 5; k++) L(ctx, [[x - 5, 30 + t * 4 + k * 12], [x + 5, 30 + t * 4 + k * 12]], C.dk, 2.4, "butt");
    }
    P(ctx, [[34, 20], [18, 6, 26, 24]], C.leaf || "#6f9a44", false);
    L(ctx, [[24, 60], [76, 60]], "rgba(40,30,16,0.45)", 4);
  } else if (p.kind === "distaff") {
    L(ctx, [[52, 90], [50, 40]], "#7a5f3a", 5);
    for (let i = 0; i < 7; i++) {
      const s = (i - 3) * 3;
      L(ctx, [[50 + s * 0.4, 44], [46 + s, 26, 50 + s * 1.5, 10]], i % 2 ? C.base : C.lt, 3.4);
    }
    E(ctx, 50, 44, 16, 7, 0, C.dk);
    P(ctx, [[36, 60], [64, 60], [62, 74], [38, 74]], C.lt);
    for (let i = 0; i < 4; i++) L(ctx, [[39 + i * 7, 60], [39 + i * 7, 74]], C.dk, 1.8);
  } else { // sheaf of stalks
    for (let i = 0; i < 5; i++) {
      const x = 32 + i * 9;
      L(ctx, [[x - 4, 86], [x, 50, x + (i - 2) * 2, 20]], C.base, 3.4);
      E(ctx, x + (i - 2) * 2, 20, 3.5, 5, 0, C.lt);
    }
    L(ctx, [[30, 64], [70, 64]], C.dk, 5);
  }
}

/** Tree / timber / worked wood. */
function wood(ctx: Ctx, C: Cols, p: GoodParams) {
  if (p.kind === "logs") {
    for (const [x, y, r] of [[36, 64, 17], [66, 64, 17], [51, 36, 17]]) {
      E(ctx, x, y, r, r, 0, vol(ctx, x, y, r, C));
      for (let i = 1; i < 4; i++) { ctx.beginPath(); ctx.arc(x - 1, y - 1, r * i / 4.2, 0, T2); ctx.strokeStyle = C.dk; ctx.lineWidth = 1.6; ctx.stroke(); }
    }
  } else if (p.kind === "tree" && !p.palm) {
    P(ctx, [[44, 88], [46, 60], [54, 60], [56, 88]], C.dk);
    for (const [x, y, r] of [[50, 34, 26], [30, 48, 17], [70, 48, 17]]) E(ctx, x, y, r, r * 0.85, 0, vol(ctx, x, y, r, { base: C.base, lt: C.lt, dk: C.dk }));
  } else if (p.palm) {
    L(ctx, [[48, 90], [54, 60, 50, 34]], "#6d5232", 6);
    for (let i = 0; i < 6; i++) {
      const a = -Math.PI / 2 + (i - 2.5) * 0.5;
      L(ctx, [[50, 34], [50 + Math.cos(a) * 26, 34 + Math.sin(a) * 20, 50 + Math.cos(a) * 40, 34 + Math.sin(a) * 36]], i % 2 ? C.base : C.lt, 5);
    }
    for (const [x, y] of [[44, 42], [56, 44], [50, 48]]) E(ctx, x, y, 5, 6, 0, C.dk);
  } else if (p.kind === "plank") {
    P(ctx, [[16, 40], [84, 32], [84, 56], [16, 64]], lg(ctx, 16, 0, 84, 0, [[0, C.dk], [0.5, C.base], [1, C.lt]]));
    P(ctx, [[16, 64], [84, 56], [84, 66], [16, 74]], C.dk);
    for (let i = 0; i < 3; i++) L(ctx, [[22, 46 + i * 6], [78, 38 + i * 6]], "rgba(0,0,0,0.14)", 1.6);
  } else { // chair / furniture
    P(ctx, [[30, 34], [58, 30], [60, 60], [32, 64]], lg(ctx, 30, 0, 60, 0, [[0, C.dk], [0.6, C.base], [1, C.lt]]));
    P(ctx, [[32, 62], [74, 58], [76, 70], [34, 74]], C.base);
    for (const [x, y] of [[36, 74], [70, 70], [60, 62]]) P(ctx, [[x, y], [x + 5, y - 1], [x + 4, y + 16], [x - 1, y + 16]], C.dk);
    for (let i = 0; i < 3; i++) L(ctx, [[36 + i * 8, 34], [38 + i * 8, 60]], C.dk, 2);
  }
}

/** Paper, books, candles, writing. */
function craft(ctx: Ctx, C: Cols, p: GoodParams) {
  if (p.kind === "scroll") {
    P(ctx, [[24, 28], [76, 28], [76, 74], [24, 74]], lg(ctx, 0, 28, 0, 74, [[0, C.lt], [0.5, C.base], [1, C.dk]]));
    for (let i = 0; i < 4; i++) L(ctx, [[34, 40 + i * 9], [66, 40 + i * 9]], "rgba(60,44,26,0.4)", 2);
    E(ctx, 24, 51, 7, 25, 0, C.dk); E(ctx, 76, 51, 7, 25, 0, C.base);
  } else if (p.kind === "book") {
    P(ctx, [[22, 24], [74, 20], [78, 74], [26, 80]], C.dk);
    P(ctx, [[28, 28], [72, 24], [75, 70], [31, 76]], C.lt);
    for (let i = 0; i < 5; i++) L(ctx, [[34, 36 + i * 8], [68, 33 + i * 8]], "rgba(70,54,34,0.35)", 1.8);
    P(ctx, [[22, 24], [26, 80], [20, 74], [16, 20]], C.base);
    if (p.clasp) P(ctx, [[70, 42], [80, 40], [80, 52], [70, 54]], "#c9a24a");
  } else if (p.kind === "candle") {
    P(ctx, [[42, 32], [58, 32], [60, 84], [40, 84]], lg(ctx, 40, 0, 60, 0, [[0, C.dk], [0.45, C.lt], [1, C.dk]]));
    L(ctx, [[50, 32], [50, 24]], "#3a2c1c", 2.5);
    P(ctx, [[50, 8], [58, 22], [50, 28], [42, 22]], "#f0b24a");
    P(ctx, [[50, 14], [54, 22], [50, 26], [46, 22]], "#fff0c0");
    L(ctx, [[44, 42], [44, 78]], "rgba(255,255,255,0.35)", 3);
    P(ctx, [[32, 84], [68, 84], [72, 92], [28, 92]], "#8c7a56");
  } else if (p.kind === "satchel") {
    P(ctx, [[26, 42], [74, 42], [78, 80], [22, 80]], lg(ctx, 22, 0, 78, 0, [[0, C.dk], [0.45, C.base], [1, C.lt]]));
    P(ctx, [[24, 38], [76, 38], [78, 58], [22, 58]], C.lt);
    L(ctx, [[24, 58], [76, 58]], C.dk, 2.5);
    P(ctx, [[44, 54], [56, 54], [56, 66], [44, 66]], C.dk);
    E(ctx, 50, 60, 3.2, 3.2, 0, "#c9a24a");
    L(ctx, [[28, 42], [50, 18, 72, 42]], C.dk, 5);
    for (let i = 0; i < 2; i++) L(ctx, [[32 + i * 36, 44], [32 + i * 36, 56]], C.dk, 3);
  } else { // soap / bar
    P(ctx, [[24, 40], [76, 34], [80, 64], [28, 72]], lg(ctx, 24, 0, 80, 0, [[0, C.lt], [0.5, C.base], [1, C.dk]]));
    P(ctx, [[24, 40], [76, 34], [70, 44], [30, 50]], C.hi);
    E(ctx, 52, 52, 10, 7, -0.1, "rgba(255,255,255,0.25)");
    ctx.beginPath(); ctx.arc(52, 52, 9, 0, T2); ctx.strokeStyle = "rgba(255,255,255,0.35)"; ctx.lineWidth = 2; ctx.stroke();
  }
}

/** Marine curios: pearl, shell, coral, ambergris. */
function marine(ctx: Ctx, C: Cols, p: GoodParams) {
  if (p.kind === "pearl") {
    P(ctx, [[16, 60], [50, 32, 84, 60], [74, 76], [26, 76]], C.dk);
    P(ctx, [[20, 60], [50, 38, 80, 60], [70, 70], [30, 70]], C.base);
    for (let i = 0; i < 7; i++) L(ctx, [[50, 68], [22 + i * 9.4, 44]], "rgba(255,255,255,0.3)", 2);
    E(ctx, 50, 58, 12, 12, 0, rg(ctx, 50, 58, 2, 14, [[0, "#ffffff"], [0.6, "#e6eef4"], [1, "#b9c8d4"]]));
    E(ctx, 46, 54, 4, 3, -0.6, "#ffffff");
  } else if (p.kind === "conch") {
    P(ctx, [[26, 74], [20, 30, 58, 22], [86, 42], [66, 80], [26, 74]], lg(ctx, 20, 22, 86, 80, [[0, C.lt], [0.55, C.base], [1, C.dk]]));
    for (let i = 0; i < 5; i++) L(ctx, [[30 + i * 4, 72 - i * 3], [52 + i * 6, 30 + i * 6]], C.hi, 2.4);
    E(ctx, 62, 58, 12, 9, 0.4, C.dk);
  } else if (p.kind === "coral") {
    const br = (x: number, y: number, a: number, len: number, d: number) => {
      const ex = x + Math.cos(a) * len, ey = y + Math.sin(a) * len;
      L(ctx, [[x, y], [ex, ey]], d === 2 ? C.dk : C.base, 3.4 + d * 1.6);
      if (d > 0) { br(ex, ey, a - 0.62, len * 0.74, d - 1); br(ex, ey, a + 0.5, len * 0.74, d - 1); }
    };
    br(50, 88, -Math.PI / 2, 24, 2);
    for (const [x, y] of [[36, 44], [64, 46], [50, 30]]) E(ctx, x, y, 4, 4, 0, C.lt);
    E(ctx, 50, 90, 22, 6, 0, "rgba(30,44,54,0.5)");
  } else { // ambergris / lump
    P(ctx, [[22, 56], [30, 26, 66, 24], [82, 50], [70, 80], [34, 80]], rg(ctx, 46, 48, 6, 40, [[0, C.lt], [0.55, C.base], [1, C.dk]]));
    for (const [x, y, r] of [[40, 46, 6], [60, 58, 5], [52, 34, 4]]) E(ctx, x, y, r, r * 0.7, 0.4, C.dk);
    E(ctx, 38, 40, 8, 5, -0.5, "rgba(255,255,255,0.3)");
  }
}

/** Mineral heaps, salt crystals, clay. */
function mineral(ctx: Ctx, C: Cols, p: GoodParams) {
  if (p.kind === "crystals") {
    P(ctx, [[24, 84], [36, 44], [48, 84]], C.lt);
    P(ctx, [[40, 84], [56, 26], [72, 84]], C.base);
    P(ctx, [[62, 84], [74, 50], [86, 84]], C.lt);
    P(ctx, [[56, 26], [64, 52], [56, 58], [48, 52]], C.hi);
    E(ctx, 50, 86, 34, 7, 0, C.dk);
  } else if (p.kind === "heap") {
    P(ctx, [[18, 82], [50, 36], [82, 82]], lg(ctx, 18, 36, 82, 82, [[0, C.lt], [0.6, C.base], [1, C.dk]]));
    for (const [x, y, r] of [[38, 66, 5], [56, 58, 4], [48, 74, 5], [64, 74, 4]]) E(ctx, x, y, r, r * 0.8, 0, C.hi);
    E(ctx, 50, 84, 34, 7, 0, C.dk);
    if (p.scoop) { P(ctx, [[58, 44], [80, 32], [86, 44], [64, 56]], "#b9a887"); P(ctx, [[80, 34], [92, 26], [95, 31], [83, 39]], "#7a5f3a"); }
  } else if (p.kind === "loaf") {
    P(ctx, [[50, 16], [70, 80], [30, 80]], lg(ctx, 30, 0, 70, 0, [[0, C.lt], [0.45, C.hi], [1, C.dk]]));
    P(ctx, [[50, 16], [60, 48], [40, 48]], "rgba(255,255,255,0.5)");
    L(ctx, [[36, 64], [64, 64]], "rgba(120,100,70,0.35)", 3);
    E(ctx, 50, 80, 20, 6, 0, C.dk);
    P(ctx, [[24, 80], [76, 80], [80, 88], [20, 88]], "#8a7250");
  } else if (p.kind === "pan") {
    P(ctx, [[14, 44], [86, 44], [76, 80], [24, 80]], "#6f6350");
    P(ctx, [[20, 48], [80, 48], [72, 76], [28, 76]], lg(ctx, 0, 48, 0, 76, [[0, C.hi], [0.6, C.lt], [1, C.base]]));
    for (let i = 0; i < 5; i++) L(ctx, [[24 + i * 2, 52 + i * 5], [76 - i * 2, 52 + i * 5]], "rgba(150,140,120,0.45)", 2);
    P(ctx, [[44, 20], [52, 20], [56, 50], [48, 50]], "#7a5f3a");
    P(ctx, [[34, 18], [66, 18], [66, 24], [34, 24]], "#7a5f3a");
  } else if (p.kind === "block") {
    P(ctx, [[24, 40], [64, 28], [80, 40], [80, 72], [40, 84], [24, 72]], C.base);
    P(ctx, [[24, 40], [64, 28], [80, 40], [40, 52]], C.lt);
    P(ctx, [[40, 52], [80, 40], [80, 72], [40, 84]], C.dk);
    if (p.veins) for (const [a, b] of [[[46, 58], [70, 50]], [[44, 72], [66, 64]]]) L(ctx, [a, b], C.hi, 2.4);
  } else { // resin drop
    P(ctx, [[50, 18], [76, 52, 66, 74], [50, 84], [34, 74, 24, 52]], rg(ctx, 46, 48, 4, 40, [[0, C.hi], [0.45, C.base], [1, C.dk]]));
    E(ctx, 42, 44, 9, 13, -0.4, "rgba(255,255,255,0.4)");
    if (p.inclusion) E(ctx, 56, 58, 5, 4, 0.4, "rgba(60,36,10,0.55)");
  }
}

/** Fired-earth wares and glass. */
function ware(ctx: Ctx, C: Cols, p: GoodParams) {
  if (p.kind === "glass") {
    P(ctx, [[36, 26], [64, 26], [62, 40], [70, 64], [50, 80], [30, 64], [38, 40]],
      lg(ctx, 30, 0, 70, 0, [[0, "rgba(255,255,255,0.5)"], [0.35, C.lt], [0.7, C.base], [1, C.dk]]));
    E(ctx, 50, 26, 14, 5, 0, C.lt);
    L(ctx, [[40, 34], [36, 60]], "rgba(255,255,255,0.7)", 4);
  } else if (p.kind === "pot") {
    P(ctx, [[34, 24], [66, 24], [62, 34], [78, 56], [62, 80], [38, 80], [22, 56], [38, 34]],
      lg(ctx, 22, 0, 78, 0, [[0, C.dk], [0.4, C.base], [0.75, C.lt], [1, C.dk]]));
    L(ctx, [[28, 50], [72, 50]], C.hi, 3.5);
    for (let i = 0; i < 5; i++) E(ctx, 30 + i * 10, 60, 3, 3, 0, C.hi);
    E(ctx, 50, 24, 16, 5, 0, C.lt);
  } else if (p.kind === "vat") {
    P(ctx, [[24, 38], [76, 38], [70, 84], [30, 84]], lg(ctx, 24, 0, 76, 0, [[0, "#5c4a34"], [0.5, "#7a6144"], [1, "#4a3b28"]]));
    E(ctx, 50, 38, 26, 9, 0, C.base);
    E(ctx, 50, 38, 22, 7, 0, C.dk);
    for (const y of [52, 70]) L(ctx, [[26, y], [74, y]], "rgba(30,22,14,0.5)", 3.5, "butt");
    L(ctx, [[62, 14], [58, 30, 64, 40]], "#8a7a5c", 4);
    P(ctx, [[52, 18], [74, 14], [70, 32], [50, 34]], C.lt);
    for (let i = 0; i < 4; i++) L(ctx, [[54 + i * 5, 18], [52 + i * 5, 33]], C.base, 2);
  } else if (p.kind === "bell") {
    P(ctx, [[38, 28], [62, 28], [74, 70], [26, 70]], lg(ctx, 26, 0, 74, 0, [[0, C.dk], [0.42, C.lt], [0.75, C.base], [1, C.dk]]));
    E(ctx, 50, 70, 24, 8, 0, C.base);
    E(ctx, 50, 70, 24, 8, 0, lg(ctx, 26, 0, 74, 0, [[0, C.dk], [0.5, C.lt], [1, C.dk]]));
    L(ctx, [[50, 26], [50, 16]], C.dk, 5);
    ctx.beginPath(); ctx.arc(50, 14, 7, Math.PI, 0); ctx.strokeStyle = C.base; ctx.lineWidth = 5; ctx.stroke();
    E(ctx, 50, 80, 5, 7, 0, C.dk);
    L(ctx, [[34, 44], [30, 64]], "rgba(255,255,255,0.3)", 4);
  } else { // clay lump on a wheel
    P(ctx, [[30, 44], [70, 44], [64, 72], [36, 72]], lg(ctx, 30, 0, 70, 0, [[0, C.dk], [0.5, C.base], [1, C.lt]]));
    E(ctx, 50, 44, 20, 7, 0, C.lt); E(ctx, 50, 44, 11, 4, 0, C.dk);
    E(ctx, 50, 76, 32, 8, 0, C.dk);
  }
}

/** Statues, columns, carved goods, jewelry. */
function carved(ctx: Ctx, C: Cols, p: GoodParams) {
  if (p.kind === "column") {
    P(ctx, [[30, 22], [70, 22], [70, 30], [30, 30]], C.lt);
    P(ctx, [[36, 30], [64, 30], [64, 76], [36, 76]], lg(ctx, 36, 0, 64, 0, [[0, C.dk], [0.45, C.lt], [1, C.dk]]));
    for (const x of [42, 50, 58]) L(ctx, [[x, 32], [x, 74]], C.dk, 2);
    P(ctx, [[28, 76], [72, 76], [74, 86], [26, 86]], C.base);
  } else if (p.kind === "statue") {
    E(ctx, 50, 26, 10, 12, 0, C.lt);
    P(ctx, [[40, 38], [60, 38], [64, 72], [36, 72]], lg(ctx, 36, 0, 64, 0, [[0, C.dk], [0.5, C.lt], [1, C.dk]]));
    P(ctx, [[40, 40], [30, 58], [36, 60], [44, 46]], C.base);
    P(ctx, [[60, 40], [72, 54], [66, 58], [56, 46]], C.base);
    P(ctx, [[30, 72], [70, 72], [74, 86], [26, 86]], C.dk);
  } else if (p.kind === "tusk") {
    L(ctx, [[22, 74], [46, 58, 58, 32], [62, 18]], C.lt, 13);
    L(ctx, [[24, 76], [48, 60, 60, 34]], C.dk, 4);
    E(ctx, 22, 74, 7, 7, 0, C.base);
  } else if (p.kind === "ring") {
    ctx.beginPath(); ctx.arc(50, 58, 24, 0, T2); ctx.strokeStyle = C.base; ctx.lineWidth = 9; ctx.stroke();
    ctx.beginPath(); ctx.arc(50, 58, 24, Math.PI * 1.1, Math.PI * 1.75); ctx.strokeStyle = C.lt; ctx.lineWidth = 9; ctx.stroke();
    P(ctx, [[50, 10], [64, 26], [50, 40], [36, 26]], C.hi);
    P(ctx, [[50, 10], [64, 26], [50, 26]], "rgba(255,255,255,0.55)");
  } else if (p.kind === "figurine") {
    E(ctx, 50, 24, 9, 10, 0, C.lt);
    P(ctx, [[42, 34], [58, 34], [62, 66], [38, 66]], lg(ctx, 38, 0, 62, 0, [[0, C.dk], [0.45, C.lt], [1, C.base]]));
    P(ctx, [[42, 36], [28, 52], [33, 56], [46, 42]], C.base);
    P(ctx, [[58, 36], [70, 48], [66, 53], [54, 42]], C.base);
    P(ctx, [[38, 66], [62, 66], [66, 78], [34, 78]], C.base);
    P(ctx, [[30, 78], [70, 78], [74, 88], [26, 88]], C.dk);
    L(ctx, [[46, 40], [46, 62]], "rgba(255,255,255,0.3)", 2.5);
  } else { // disc / bi
    E(ctx, 50, 54, 32, 32, 0, rg(ctx, 50, 54, 6, 34, [[0, C.lt], [0.6, C.base], [1, C.dk]]));
    E(ctx, 50, 54, 11, 11, 0, "rgba(12,18,24,0.85)");
    ctx.beginPath(); ctx.arc(50, 54, 22, 0, T2); ctx.strokeStyle = C.hi; ctx.lineWidth = 2; ctx.stroke();
  }
}

/** Smoke/aroma column for censed goods. */
function aroma(ctx: Ctx, C: Cols, p: GoodParams) {
  P(ctx, [[30, 74], [70, 74], [64, 88], [36, 88]], C.dk);
  P(ctx, [[34, 66], [66, 66], [70, 76], [30, 76]], C.base);
  for (const dx of [-10, 10]) L(ctx, [[50 + dx, 64], [50 + dx - 12, 46, 50 + dx + 10, 30], [50 + dx - 6, 14]], C.lt, 4);
  if (p.grains) for (const [x, y, r] of [[44, 60, 5], [56, 58, 4], [50, 62, 4]]) E(ctx, x, y, r, r * 0.8, 0, C.hi);
}

/** Sack of a commodity, with the goods spilling out. */
function sack(ctx: Ctx, C: Cols, p: GoodParams) {
  P(ctx, [[28, 44], [72, 44], [80, 80], [20, 80]], lg(ctx, 20, 0, 80, 0, [[0, C.dk], [0.45, C.base], [1, C.lt]]));
  P(ctx, [[34, 30], [66, 30], [72, 46], [28, 46]], C.lt);
  L(ctx, [[34, 38], [66, 38]], C.dk, 4);
  for (const [x, y, r] of (p.spill || [[42, 26, 5], [54, 22, 5], [62, 28, 4]])) E(ctx, x, y, r, r * (p.round === false ? 0.6 : 1), 0.4, C.hi);
  L(ctx, [[36, 52], [36, 76]], "rgba(255,255,255,0.2)", 4);
}

type Family = (ctx: Ctx, C: Cols, p: GoodParams) => void;
const F: Record<string, Family> = {
  ear, panicle, fruit, vessel, cloth, metal, gemstone, fish, whale, beast,
  botanical, wood, craft, marine, mineral, ware, carved, aroma, sack,
};

// ── per-good recipes: family + its variation. No two goods repeat a set. ─────
export const RECIPES: Record<string, [string, GoodParams]> = {
  wheat: ["ear", { n: 5, kr: 8, kw: 5, awn: 0.5, leaf: 1 }],
  rice: ["ear", { n: 6, kr: 6.5, kw: 4, droop: 0.6, leaf: 1 }],
  barley: ["ear", { n: 6, kr: 7, kw: 4.5, awn: 1, leaf: 0 }],
  millet: ["panicle", { rows: 8, taper: 1 }],
  dates: ["wood", { kind: "tree", palm: 1 }],
  honey: ["vessel", { nk: 9, bel: 28, top: 20, lip: 1, fill: 0.62, handles: 0 }],

  wine: ["vessel", { nk: 6, bel: 22, top: 14, cork: 1, fill: 0.5, foot: 14 }],
  oliveoil: ["vessel", { nk: 8, bel: 30, top: 18, handles: 1, lip: 1 }],
  citrus: ["fruit", { n: 1, segments: 1, leaf: 1 }],
  beer: ["vessel", { nk: 20, bel: 24, top: 26, hoops: 1, fill: 0.72, foot: 20 }],
  mead: ["vessel", { nk: 14, bel: 26, top: 22, fill: 0.66, lip: 1, foot: 16 }],
  brandy: ["vessel", { nk: 5, bel: 26, top: 12, cork: 1, fill: 0.4, foot: 16 }],
  citrus_liqueur: ["vessel", { nk: 7, bel: 20, top: 16, stem: 1, fill: 0.55, bot: 76 }],

  sugar: ["botanical", { kind: "cane" }],
  refined_sugar: ["mineral", { kind: "loaf" }],
  tobacco: ["botanical", { kind: "leaf", second: 1 }],
  indigo: ["botanical", { kind: "flower", petals: 5 }],
  coffee: ["sack", { spill: [[40, 24, 5], [52, 20, 5], [62, 26, 5]] }],
  tea: ["botanical", { kind: "leaf" }],
  cacao: ["fruit", { n: 3, oblate: 1.25 }],

  spices: ["mineral", { kind: "heap", scoop: 1 }],
  cloves: ["botanical", { kind: "buds" }],
  pepper: ["botanical", { kind: "corns" }],
  cinnamon: ["botanical", { kind: "bark" }],
  frankincense: ["mineral", { kind: "resin" }],
  incense: ["aroma", { grains: 1 }],
  saffron: ["botanical", { kind: "threads" }],
  perfume: ["vessel", { nk: 5, bel: 24, top: 16, cork: 1, bot: 74, foot: 18 }],

  silk: ["cloth", { kind: "roll" }],
  cotton: ["fruit", { n: 5 }],
  flax: ["botanical", { kind: "distaff" }],
  wool_fleece: ["beast", { kind: "sheep" }],
  wool_llama: ["cloth", { kind: "hank" }],
  furs: ["beast", { kind: "hide", fur: 1 }],
  hides: ["beast", { kind: "hide" }],
  horses: ["beast", { kind: "horse" }],
  ivory: ["carved", { kind: "tusk" }],
  cloth: ["cloth", { kind: "bolt" }],
  linen: ["cloth", { kind: "stack" }],
  cotton_cloth: ["cloth", { kind: "bolt", pattern: 0 }],
  silk_brocade: ["cloth", { kind: "bolt", pattern: 1 }],
  carpets: ["cloth", { kind: "rug" }],
  leather_goods: ["craft", { kind: "satchel" }],

  timber: ["wood", { kind: "logs" }],
  hardwoods: ["wood", { kind: "tree" }],
  paper: ["craft", { kind: "scroll" }],
  clay: ["ware", { kind: "clay" }],
  ceramics: ["ware", { kind: "pot" }],
  glassware: ["ware", { kind: "glass" }],
  books: ["craft", { kind: "book", clasp: 1 }],
  furniture: ["wood", { kind: "chair" }],
  candles: ["craft", { kind: "candle" }],
  soap: ["craft", { kind: "soap" }],
  statuary: ["carved", { kind: "statue" }],
  ivory_carvings: ["carved", { kind: "figurine" }],

  salt: ["mineral", { kind: "crystals" }],
  bay_salt: ["mineral", { kind: "pan" }],
  iron: ["metal", { kind: "ore" }],
  copper: ["metal", { n: 3 }],
  tin: ["metal", { kind: "sheet" }],
  lead: ["metal", { kind: "pigs" }],
  gold: ["metal", { kind: "coins" }],
  silver: ["metal", { kind: "plate" }],
  gemstones: ["gemstone", { cut: "rough" }],
  ruby: ["gemstone", { cut: "cushion" }],
  sapphire: ["gemstone", { cut: "brilliant" }],
  emerald: ["gemstone", { cut: "emerald" }],
  diamond: ["gemstone", { cut: "pear" }],
  amethyst: ["gemstone", { cut: "trilliant" }],
  topaz: ["gemstone", { cut: "marquise" }],
  jade: ["carved", { kind: "disc" }],
  marble: ["mineral", { kind: "block", veins: 1 }],
  metalware: ["metal", { kind: "ware" }],
  bronzeware: ["ware", { kind: "bell" }],
  jewelry: ["carved", { kind: "ring" }],

  stockfish: ["fish", { depth: 14, dried: 1 }],
  herring: ["fish", { depth: 17, stripes: 1 }],
  salted_herring: ["vessel", { nk: 22, bel: 26, top: 24, hoops: 1, foot: 22 }],
  pearls: ["marine", { kind: "pearl" }],
  whaling: ["whale", {}],
  amber: ["mineral", { kind: "resin", inclusion: 1 }],
  dyes: ["ware", { kind: "vat" }],
  tyrian_purple: ["marine", { kind: "conch" }],
  coral: ["marine", { kind: "coral" }],
  ambergris: ["marine", { kind: "lump" }],
};

/** Draw one good's illustration into a 0..100 box at (0,0), scaled by `size`.
 *  A good with no recipe (a user-added custom) falls back to a mineral heap. */
export function drawGood(ctx: Ctx, name: string, size: number, color: string) {
  const rec = RECIPES[name] || ["mineral", { kind: "heap" }];
  const C: Cols = {
    base: color, lt: shade(color, 1.28), hi: shade(color, 1.55), dk: shade(color, 0.6),
    stem: "#6d5a32", leaf: "#5f8a3e",
  };
  ctx.save();
  ctx.scale(size / 100, size / 100);
  ctx.lineJoin = "round"; ctx.lineCap = "round";
  (F[rec[0]] || F.mineral)(ctx, C, rec[1] || {});
  ctx.restore();
}

function mkCanvas(w: number, h: number): HTMLCanvasElement {
  const c = document.createElement("canvas"); c.width = w; c.height = h; return c;
}

export interface PixelIconOpts { grid?: number; glow?: number }

/** Standalone PIXEL ICON: the good's artwork on a coarse grid, stamped with a
 *  one-pixel dark edge and a bevel, plus a bright rim-glow for dark goods so
 *  black subjects still read on a dark panel. */
export function drawIcon(
  ctx: Ctx, cx: number, cy: number, size: number, color: string, name: string, opts: PixelIconOpts = {},
) {
  const G = Math.max(20, Math.round(opts.grid || 46));   // art grid — every icon the same
  const pad = 3;
  const W = G + pad * 2;

  // 1. draw once, measure the subject, then redraw scaled so EVERY good fills
  //    the same share of the grid — that's what makes the set read as even.
  const probe = mkCanvas(W, W);
  const pc = probe.getContext("2d")!;
  pc.save(); pc.translate(pad, pad); drawGood(pc, name, G, color); pc.restore();
  let x0 = W, y0 = W, x1 = 0, y1 = 0;
  const d0 = pc.getImageData(0, 0, W, W).data;
  for (let y = 0; y < W; y++) for (let x = 0; x < W; x++) {
    if (d0[(y * W + x) * 4 + 3] > 24) { if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; }
  }
  const bw = Math.max(1, x1 - x0 + 1), bh = Math.max(1, y1 - y0 + 1);
  const target = G * 0.92;
  const k = Math.min(target / bw, target / bh, 1.9);

  const art = mkCanvas(W, W);
  const a = art.getContext("2d")!;
  a.save();
  a.translate(W / 2, W / 2);
  a.scale(k, k);
  a.translate(-(x0 + bw / 2), -(y0 + bh / 2));
  a.translate(pad, pad); drawGood(a, name, G, color);
  a.restore();

  // 1b. NORMALISE: measure the ink bbox and refit it so every icon carries the
  //     same visual weight in its cell (fit the long axis, floor the short one).
  {
    const d = a.getImageData(0, 0, W, W).data;
    let nx0 = W, ny0 = W, nx1 = -1, ny1 = -1;
    for (let y = 0; y < W; y++) for (let x = 0; x < W; x++) {
      if (d[(y * W + x) * 4 + 3] > 12) { if (x < nx0) nx0 = x; if (x > nx1) nx1 = x; if (y < ny0) ny0 = y; if (y > ny1) ny1 = y; }
    }
    if (nx1 >= nx0 && ny1 >= ny0) {
      const nbw = nx1 - nx0 + 1, nbh = ny1 - ny0 + 1, t2 = G * 0.82, floor = G * 0.44;
      let k2 = t2 / Math.max(nbw, nbh);
      if (Math.min(nbw, nbh) * k2 < floor) k2 = Math.min(k2 * 1.55, floor / Math.min(nbw, nbh));
      k2 = Math.max(0.6, Math.min(2.6, k2));
      const src = mkCanvas(W, W);
      src.getContext("2d")!.drawImage(art, 0, 0);
      a.clearRect(0, 0, W, W);
      a.save();
      a.translate(W / 2, W / 2);
      a.scale(k2, k2);
      a.translate(-(nx0 + nbw / 2), -(ny0 + nbh / 2));
      a.drawImage(src, 0, 0);
      a.restore();
    }
  }

  // 2. its silhouette, used for the dark edge and the bevel
  const sil = (fill: string) => {
    const c = mkCanvas(W, W);
    const x = c.getContext("2d")!; x.drawImage(art, 0, 0);
    x.globalCompositeOperation = "source-in"; x.fillStyle = fill; x.fillRect(0, 0, W, W); return c;
  };
  const dark = sil("#0d0b08"), light = sil("#ffffff");

  const out = mkCanvas(W, W);
  const o = out.getContext("2d")!;
  // dark edge: the silhouette stamped one grid-pixel out in eight directions
  for (const [dx, dy] of [[-1, 0], [1, 0], [0, -1], [0, 1], [-1, -1], [1, -1], [-1, 1], [1, 1]]) o.drawImage(dark, dx, dy);
  o.drawImage(dark, 0, 0);
  o.drawImage(art, 0, 0);
  // bevel: a ONE-pixel rim band, not a wash — the silhouette minus itself, offset
  const band = (src: HTMLCanvasElement, dx: number, dy: number) => {
    const c = mkCanvas(W, W);
    const x = c.getContext("2d")!; x.drawImage(src, 0, 0);
    x.globalCompositeOperation = "destination-out"; x.drawImage(art, dx, dy); return c;
  };
  o.globalCompositeOperation = "source-atop";
  o.globalAlpha = 0.75; o.drawImage(band(light, 1, 1), 0, 0);    // shiny top-left rim
  o.globalAlpha = 0.4; o.drawImage(band(dark, -1, -1), 0, 0);    // shaded bottom-right
  o.globalAlpha = 1; o.globalCompositeOperation = "source-over";

  const Lm = lum(color), glow = opts.glow ?? 1;
  const px = size / W;
  ctx.save();
  // authored at grid resolution and upscaled — smoothing OFF is what makes the edge crisp
  ctx.imageSmoothingEnabled = false;
  const dx0 = cx - W * px / 2, dy0 = cy - W * px / 2;
  // dark goods keep a white shine so they read on a dark panel
  if (glow && Lm < 0.42) {
    ctx.save(); ctx.globalAlpha = Math.min(0.7, (0.8 - Lm)) * glow;
    for (const [ox, oy] of [[-1, -1], [1, -1], [-1, 1], [1, 1]]) ctx.drawImage(light, dx0 + ox * px, dy0 + oy * px, W * px, W * px);
    ctx.restore();
  }
  ctx.drawImage(out, dx0, dy0, W * px, W * px);
  ctx.restore();
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

const RIMS: Record<string, { a: string; b: string; c: string; d: string }> = {
  gold: { a: "#f6e6b0", b: "#d8b24a", c: "#8a6f2c", d: "#5d4a1c" },
  steel: { a: "#f0f5fa", b: "#a9b8c6", c: "#63727f", d: "#3b4650" },
  dark: { a: "#5d6c7c", b: "#33404e", c: "#1b232c", d: "#0e141a" },
};

/** Mutes a good's tint towards a warm sepia — the desaturated, aged-print
 *  palette a hand-painted ledger icon uses instead of a bright flat colour. */
function vicMute(hex: string): string {
  const c = hx(hex), g = (c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11);
  const sepia = [112, 92, 64];
  const mix = (v: number, s: number, t: number) => v * (1 - t) + s * t;
  const r = [mix(mix(c[0], g, 0.42), sepia[0], 0.22), mix(mix(c[1], g, 0.42), sepia[1], 0.22), mix(mix(c[2], g, 0.42), sepia[2], 0.22)];
  return "#" + r.map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0")).join("");
}

/** Deterministic per-good noise, so the same good textures the same way twice. */
function seeded(name: string): (n: number) => number {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 131 + name.charCodeAt(i)) >>> 0;
  return () => { h = (h * 1103515245 + 12345) >>> 0; return ((h >>> 8) % 10000) / 10000; };
}

/** Draws the good, then lays a MATERIAL texture over just its own ink (masked
 *  with source-atop) — the difference between a flat icon and a painted one:
 *  grain speckle on organic goods, brushed streaks on metal, a woven check on
 *  cloth, a glass reflection streak on vessels and jars. */
function drawGoodMaterial(ctx: Ctx, name: string, s: number, color: string, family: string) {
  const pad = 4, W = Math.ceil(s) + pad * 2;
  const off = mkCanvas(W, W);
  const o = off.getContext("2d")!;
  o.translate(pad, pad);
  drawGood(o, name, s, color);
  o.save(); o.globalCompositeOperation = "source-atop";
  const rnd = seeded(name);
  if (family === "ear" || family === "panicle" || family === "fruit" || family === "botanical" || family === "beast") {
    for (let i = 0; i < 46; i++) {
      const x = rnd(i) * W, y = rnd(i + 1) * W;
      o.fillStyle = rnd(i + 2) > 0.52 ? "rgba(255,250,232,0.12)" : "rgba(36,24,10,0.11)";
      o.beginPath(); o.arc(x, y, 0.7 + rnd(i + 3) * 1.3, 0, T2); o.fill();
    }
  } else if (family === "metal" || family === "mineral" || family === "carved") {
    for (let i = 0; i < 11; i++) {
      const y = rnd(i) * W;
      o.strokeStyle = rnd(i + 1) > 0.5 ? "rgba(255,255,255,0.11)" : "rgba(0,0,0,0.13)";
      o.lineWidth = 0.7; o.beginPath(); o.moveTo(0, y); o.lineTo(W, y + (rnd(i + 2) - 0.5) * 7); o.stroke();
    }
  } else if (family === "cloth") {
    for (let x = 0; x < W; x += 3.4) for (let y = 0; y < W; y += 3.4) {
      o.fillStyle = ((x + y) / 3.4) % 2 ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.07)";
      o.fillRect(x, y, 1.6, 1.6);
    }
  } else if (family === "vessel" || family === "ware") {
    const g = o.createLinearGradient(0, 0, W * 0.4, 0);
    g.addColorStop(0, "rgba(255,255,255,0.24)"); g.addColorStop(0.5, "rgba(255,255,255,0.02)"); g.addColorStop(1, "rgba(255,255,255,0)");
    o.fillStyle = g; o.fillRect(0, 0, W, W);
  } else if (family === "wood" || family === "craft") {
    for (let i = 0; i < 6; i++) {
      const y = W * 0.15 + i * W * 0.13;
      o.strokeStyle = "rgba(0,0,0,0.09)"; o.lineWidth = 0.6;
      o.beginPath(); o.moveTo(0, y); o.bezierCurveTo(W * 0.3, y + 2, W * 0.7, y - 2, W, y); o.stroke();
    }
  }
  o.restore();
  ctx.drawImage(off, -pad, -pad);
}

/** A Victoria-II-style ledger icon: a hand-painted good inside a bevelled
 *  bronze frame on an aged paper card, muted and softly lit rather than the
 *  flat pixel-art treatment `drawIcon` uses. `size` is the drawn square. */
export function drawIconVictorian(ctx: Ctx, cx: number, cy: number, size: number, color: string, name: string) {
  const muted = vicMute(color);
  ctx.save();
  ctx.translate(cx - size / 2, cy - size / 2);
  // paper card
  const rr = (x: number, y: number, w: number, h: number, r: number) => {
    ctx.beginPath(); ctx.moveTo(x + r, y);
    ctx.arcTo(x + w, y, x + w, y + h, r); ctx.arcTo(x + w, y + h, x, y + h, r);
    ctx.arcTo(x, y + h, x, y, r); ctx.arcTo(x, y, x + w, y, r); ctx.closePath();
  };
  const pg = lg(ctx, 0, 0, size, size, [[0, "#e2cf9e"], [0.55, "#cdb27c"], [1, "#a9885a"]]);
  rr(0, 0, size, size, size * 0.07); ctx.fillStyle = pg; ctx.fill();
  // subtle paper grain
  ctx.save(); ctx.clip();
  for (let i = 0; i < 26; i++) {
    const gx = (i * 53) % size, gy = (i * 97) % size;
    ctx.fillStyle = i % 2 ? "rgba(70,50,24,0.05)" : "rgba(255,244,214,0.06)"; ctx.fillRect(gx, gy, size * 0.09, size * 0.02);
  }
  ctx.restore();
  // vignette
  ctx.save(); rr(0, 0, size, size, size * 0.07); ctx.clip();
  const vg = rg(ctx, size / 2, size / 2, size * 0.2, size * 0.72, [[0, "rgba(0,0,0,0)"], [1, "rgba(40,26,10,0.32)"]]);
  ctx.fillStyle = vg; ctx.fillRect(0, 0, size, size);
  ctx.restore();
  // the good, muted, softly lit, no hard pixel edge
  ctx.save();
  ctx.translate(size * 0.11, size * 0.11);
  ctx.shadowColor = "rgba(30,18,6,0.45)"; ctx.shadowBlur = size * 0.05; ctx.shadowOffsetY = size * 0.02;
  const fam = (RECIPES[name] || ["mineral"])[0];
  drawGoodMaterial(ctx, name, size * 0.78, muted, fam);
  ctx.restore();
  // bevelled bronze frame
  const rim = RIMS.gold, bw = Math.max(2, size * 0.05);
  ctx.lineJoin = "miter";
  rr(bw * 0.4, bw * 0.4, size - bw * 0.8, size - bw * 0.8, size * 0.06);
  ctx.strokeStyle = lg(ctx, 0, 0, size, size, [[0, rim.a], [0.5, rim.b], [1, rim.c]]);
  ctx.lineWidth = bw; ctx.stroke();
  rr(bw * 1.1, bw * 1.1, size - bw * 2.2, size - bw * 2.2, size * 0.045);
  ctx.strokeStyle = "rgba(20,12,4,0.5)"; ctx.lineWidth = Math.max(1, bw * 0.28); ctx.stroke();
  // corner rivets
  const rv = Math.max(1.4, size * 0.022);
  for (const [rx, ry] of [[bw * 0.9, bw * 0.9], [size - bw * 0.9, bw * 0.9], [bw * 0.9, size - bw * 0.9], [size - bw * 0.9, size - bw * 0.9]]) {
    ctx.beginPath(); ctx.arc(rx, ry, rv, 0, T2); ctx.fillStyle = rim.b; ctx.fill();
    ctx.beginPath(); ctx.arc(rx - rv * 0.3, ry - rv * 0.3, rv * 0.4, 0, T2); ctx.fillStyle = rim.a; ctx.fill();
  }
  ctx.restore();
}

export interface MedallionOpts { rim?: string; reeded?: boolean }

/** An enamel medallion: cast shadow, bevelled metal rim, domed enamel field,
 *  the good's illustration, and a glass highlight. */
export function drawMedallion(ctx: Ctx, cx: number, cy: number, R: number, color: string, name: string, opts: MedallionOpts = {}) {
  const rim = RIMS[opts.rim || "gold"] || RIMS.gold;
  ctx.save();
  ctx.translate(cx, cy);
  // cast shadow
  E(ctx, R * 0.06, R * 0.12, R, R, 0, rg(ctx, 0, R * 0.12, R * 0.2, R * 1.05, [[0, "rgba(0,0,0,0.55)"], [1, "rgba(0,0,0,0)"]]));
  // metal rim
  E(ctx, 0, 0, R, R, 0, lg(ctx, -R, -R, R, R, [[0, rim.a], [0.32, rim.b], [0.62, rim.c], [1, rim.d]]));
  ctx.beginPath(); ctx.arc(0, 0, R * 0.995, 0, T2); ctx.strokeStyle = "rgba(12,9,4,0.65)"; ctx.lineWidth = R * 0.05; ctx.stroke();
  // rim bevel + reeded edge
  ctx.beginPath(); ctx.arc(0, 0, R * 0.9, 0, T2); ctx.strokeStyle = "rgba(255,255,255,0.28)"; ctx.lineWidth = R * 0.035; ctx.stroke();
  if (opts.reeded !== false) {
    for (let i = 0; i < 48; i++) {
      const a = i / 48 * T2;
      L(ctx, [[Math.cos(a) * R * 0.99, Math.sin(a) * R * 0.99], [Math.cos(a) * R * 0.9, Math.sin(a) * R * 0.9]], "rgba(0,0,0,0.18)", R * 0.02);
    }
  }
  // enamel field
  const fr = R * 0.78;
  E(ctx, 0, 0, fr, fr, 0, rg(ctx, 0, 0, fr * 0.1, fr, [[0, shade(color, 1.3)], [0.55, color], [1, shade(color, 0.52)]]));
  ctx.beginPath(); ctx.arc(0, 0, fr, 0, T2); ctx.strokeStyle = "rgba(10,8,4,0.5)"; ctx.lineWidth = R * 0.035; ctx.stroke();
  // subject
  const s = fr * 1.5;
  ctx.save(); ctx.translate(-s / 2, -s / 2 - R * 0.02);
  ctx.shadowColor = "rgba(10,8,4,0.45)"; ctx.shadowBlur = R * 0.09; ctx.shadowOffsetY = R * 0.03;
  drawGood(ctx, name, s, color);
  ctx.restore();
  // glass highlight over the top-left of the field
  ctx.save();
  ctx.beginPath(); ctx.arc(0, 0, fr, 0, T2); ctx.clip();
  E(ctx, -fr * 0.3, -fr * 0.55, fr * 0.72, fr * 0.42, -0.5, "rgba(255,255,255,0.20)");
  ctx.restore();
  ctx.restore();
}
