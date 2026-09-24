// cityArt.ts — culture/climate-styled pixel-isometric SETTLEMENTS.
//
// Port of the settlement design handoff's `wf-city.js` (the canvas half only —
// the window/cards are React in `ui/campaign/CityView.tsx`). One deterministic
// generator per city, seeded from its NAME (FNV-1a → mulberry32), eight
// architectural FAMILIES, a low-resolution iso renderer that is upscaled with
// nearest-neighbour sampling, and a top-down PLAN view of the same generated
// city.
//
// Nothing here reads the sim. The caller (CityView) turns a `HubDetail` into a
// `CityCfg` — which buildings stand, whose quarters they sit in, what water the
// city is on — and this file only draws that. So a building the data does not
// hold is never drawn: the renderer places exactly `cfg.buildings`.
//
// The drawing primitives are kept close to the reference line for line, on
// purpose: the reference is the visual spec, and a faithful port is what makes a
// later diff against it readable.

// ── colour + seeded-random helpers ─────────────────────────────────────────────

/** mulberry32 — the design's deterministic generator. */
export function rng(seed: number): () => number {
  let s = (seed >>> 0) || 1;
  return () => {
    s = (s + 0x6D2B79F5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
/** FNV-1a string hash (32-bit). */
export function hstr(s: string): number {
  let h = 2166136261 >>> 0;
  for (let k = 0; k < s.length; k++) { h ^= s.charCodeAt(k); h = Math.imul(h, 16777619) >>> 0; }
  return h;
}
function hx(h: string): [number, number, number] {
  const s = h.replace("#", "");
  return [parseInt(s.slice(0, 2), 16), parseInt(s.slice(2, 4), 16), parseInt(s.slice(4, 6), 16)];
}
/** Shade a hex colour: f < 1 darkens toward black, f > 1 lightens toward white. */
function sh(hex: string, f: number): string {
  const c = hx(hex);
  const m = (v: number) => Math.max(0, Math.min(255, Math.round(f >= 1 ? v + (255 - v) * (f - 1) : v * f)));
  return "#" + c.map((v) => m(v).toString(16).padStart(2, "0")).join("");
}
function mix(a: string, b: string, t: number): string {
  const A = hx(a), B = hx(b);
  return "#" + A.map((v, k) => Math.round(v + (B[k] - v) * t).toString(16).padStart(2, "0")).join("");
}
const pick = <T,>(r: () => number, a: readonly T[]): T => a[Math.floor(r() * a.length) % a.length];

/** Normalise any colour the backend hands us (`#rgb`, `#rrggbb`, `rgb(r,g,b)`,
 *  an `[r,g,b]` triple) to `#rrggbb`, since every shading helper here parses
 *  hex. An unparseable colour falls back to the civic slate rather than NaN. */
export function toHex(c: string | [number, number, number] | undefined | null, fallback = "#7a8aa0"): string {
  if (!c) return fallback;
  if (Array.isArray(c)) {
    return "#" + c.map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0")).join("");
  }
  const s = c.trim();
  if (/^#[0-9a-fA-F]{6}$/.test(s)) return s.toLowerCase();
  if (/^#[0-9a-fA-F]{3}$/.test(s)) return "#" + s.slice(1).split("").map((ch) => ch + ch).join("").toLowerCase();
  const m = s.match(/^rgba?\(\s*(\d+)[,\s]+(\d+)[,\s]+(\d+)/i);
  if (m) return toHex([+m[1], +m[2], +m[3]], fallback);
  return fallback;
}

// ── architectural families ─────────────────────────────────────────────────────

export type StyleKey = "med" | "hansa" | "steppe" | "desert" | "east" | "trop" | "alpine" | "arctic";
type WallKind = "stone" | "brick" | "palisade" | "mud" | "rammed" | "none";
type HouseForm = "med" | "hansa" | "yurt" | "flat" | "east" | "stilt" | "chalet" | "turf" | "alpine";
type TreeKind = "cypress" | "olive" | "broad" | "conifer" | "poplar" | "palm" | "pine" | "jungle" | "snowpine";
export type ShipKind = "galley" | "carrack" | "cog" | "hulk" | "raft" | "dhow" | "junk" | "sampan" | "prahu" | "barge" | "knarr";
export type CaravanKind = "camel" | "horse" | "wagon";

export interface CityStyle {
  label: string;
  ground: string[]; fields: string[]; fieldP: number;
  trees: number; treeKinds: TreeKind[]; snow?: number;
  water: string; shore: string; road: string; street: string; plaza: string; quay: string;
  wall: WallKind; wallC: string;
  walls: string[]; roofs: string[];
  house: HouseForm; density: number;
  ships: ShipKind[]; stone: string; civRoof: string; ice?: boolean;
}

/** The eight families — palettes copied verbatim from the handoff. */
export const STYLES: Record<StyleKey, CityStyle> = {
  med: { label: "Mediterranean stone", ground: ["#8f9a5c", "#98a062", "#a2a268"], fields: ["#c2a95a", "#8a9a4a", "#a8945a", "#b89f60"], fieldP: .55, trees: .1, treeKinds: ["cypress", "olive", "olive"], water: "#2c6a8a", shore: "#d8c898", road: "#c2b08a", street: "#c9b994", plaza: "#d8caa6", quay: "#b8ab90", wall: "stone", wallC: "#c2b598", walls: ["#eadfc4", "#dcbf90", "#e8d2a4", "#cfa67c", "#f2e8d4", "#d8c4a0"], roofs: ["#b85a3a", "#a84c30", "#c46c44", "#b0603c"], house: "med", density: .9, ships: ["galley", "carrack", "galley"], stone: "#e0d6bc", civRoof: "#b85a3a" },
  hansa: { label: "Northern timber · Hanseatic", ground: ["#5e8a4a", "#648f4e", "#6a9452"], fields: ["#b8a052", "#7e9c48", "#a08c4a", "#8aa050"], fieldP: .6, trees: .2, treeKinds: ["broad", "broad", "conifer"], water: "#36586c", shore: "#b8b08a", road: "#9a9280", street: "#8e887c", plaza: "#a09a8c", quay: "#7e786e", wall: "brick", wallC: "#8e4a34", walls: ["#9c4a33", "#8a3f2b", "#b0654a", "#a85a3c", "#e2d6bf", "#d8cbb0"], roofs: ["#5a3a30", "#6c3c2e", "#474a4e", "#7a4434"], house: "hansa", density: .95, ships: ["cog", "cog", "hulk"], stone: "#9c4a33", civRoof: "#4f6f60" },
  steppe: { label: "Steppe · yurt-town", ground: ["#a6a066", "#b0a86e", "#9c9860"], fields: ["#b8a860"], fieldP: .1, trees: .02, treeKinds: ["poplar"], water: "#4e7c8a", shore: "#c8b88a", road: "#bba880", street: "#b8a57e", plaza: "#c8b690", quay: "#a8987a", wall: "palisade", wallC: "#7a5a3a", walls: ["#ece4d2", "#e4dac4", "#f2ecdc"], roofs: ["#d8cdb4", "#cfc2a6", "#e0d6c0"], house: "yurt", density: .62, ships: ["raft"], stone: "#e8e0cc", civRoof: "#c89a3a" },
  desert: { label: "Desert · oasis mudbrick", ground: ["#d8b67a", "#dcbc82", "#d2ae72"], fields: ["#7e9a46", "#8aa24e", "#6f8e40"], fieldP: 0, trees: .015, treeKinds: ["palm"], water: "#3a8e9e", shore: "#c8b27a", road: "#e2c896", street: "#e4cc9c", plaza: "#ecd8ae", quay: "#d0b888", wall: "mud", wallC: "#c8965e", walls: ["#d8ae74", "#cfa066", "#e0bd88", "#e9d3aa", "#d4a870"], roofs: ["#caa06a"], house: "flat", density: .92, ships: ["dhow"], stone: "#ecdcc0", civRoof: "#3a9e98" },
  east: { label: "East Asian", ground: ["#6c9850", "#729e56", "#68924c"], fields: ["#86b46a", "#6aa6a8", "#7cae62", "#5e9eaa"], fieldP: .72, trees: .12, treeKinds: ["pine", "broad"], water: "#3c7888", shore: "#a8a488", road: "#aaa48e", street: "#a39d8a", plaza: "#b8b29e", quay: "#8e8a7c", wall: "rammed", wallC: "#9a927e", walls: ["#e8e4d8", "#ddd6c4", "#cfc4aa"], roofs: ["#4a5560", "#3e4852", "#56606a"], house: "east", density: .9, ships: ["junk", "sampan", "junk"], stone: "#e0dac8", civRoof: "#3a4650" },
  trop: { label: "Tropical · stilt coast", ground: ["#4c8a3c", "#548f40", "#5c9844"], fields: ["#7eb04e", "#98aa48", "#6aa648"], fieldP: .35, trees: .38, treeKinds: ["palm", "jungle", "jungle", "palm"], water: "#23859c", shore: "#e4d49c", road: "#a88c60", street: "#ae9468", plaza: "#bea276", quay: "#8e7650", wall: "palisade", wallC: "#9a7a48", walls: ["#b8955a", "#a88a52", "#c4a066"], roofs: ["#c8a45a", "#b8924a", "#a88440", "#d0ae66"], house: "stilt", density: .75, ships: ["prahu", "prahu", "junk"], stone: "#9a8a6e", civRoof: "#8a6a3a" },
  alpine: { label: "Alpine · mountain", ground: ["#688a56", "#6e905a", "#62844e"], fields: ["#8aa656", "#9aac5e"], fieldP: .25, trees: .34, treeKinds: ["conifer", "conifer", "conifer", "broad"], snow: .16, water: "#4a7a98", shore: "#9a9a88", road: "#a09a8a", street: "#9a968a", plaza: "#aca89c", quay: "#8a887e", wall: "stone", wallC: "#8e8c86", walls: ["#9a968e", "#a8a49a"], roofs: ["#5a4a40", "#4e4038", "#e8eef2"], house: "chalet", density: .8, ships: ["barge"], stone: "#e4e0d6", civRoof: "#6a3a30" },
  arctic: { label: "Arctic · turf & ice", ground: ["#e6ecf0", "#dfe7ec", "#eef2f4"], fields: ["#c8d0c8"], fieldP: .05, trees: .06, treeKinds: ["snowpine"], water: "#2c4c62", shore: "#b8c4c8", road: "#b0b8bc", street: "#a8b0b4", plaza: "#bcc4c8", quay: "#8a949a", wall: "palisade", wallC: "#5a4a3e", walls: ["#6a5846", "#5e4e3e", "#76624c"], roofs: ["#f0f4f6", "#e8eef2"], house: "turf", density: .7, ships: ["knarr", "knarr"], stone: "#6a5846", civRoof: "#eef2f4", ice: true },
};

/** A human name for a family's wall, for the scene's "walls" chip. */
export const WALL_LABEL: Record<WallKind, string> = {
  stone: "Stone walls", brick: "Brick walls", palisade: "Palisade", mud: "Mudbrick walls",
  rammed: "Rammed-earth walls", none: "Unwalled",
};

// ── iso primitives (world x,y in tiles; z in px at TW=20) ─────────────────────

type Ctx = CanvasRenderingContext2D;
interface Geo { ctx: Ctx; TW: number; TH: number; OX: number; OY: number; zs: number }
type V3 = [number, number, number];

function mkG(ctx: Ctx, TW: number, OX: number, OY: number): Geo { return { ctx, TW, TH: TW / 2, OX, OY, zs: TW / 20 }; }
function P(G: Geo, x: number, y: number, z = 0): [number, number] {
  return [G.OX + (x - y) * G.TW / 2, G.OY + (x + y) * G.TH / 2 - z * G.zs];
}
function q(G: Geo, pts: V3[], fill: string) {
  const c = G.ctx; c.beginPath();
  pts.forEach((p, k) => { const [sx, sy] = P(G, p[0], p[1], p[2]); if (k) c.lineTo(sx, sy); else c.moveTo(sx, sy); });
  c.closePath(); c.fillStyle = fill; c.fill();
}
function box(G: Geo, x0: number, y0: number, x1: number, y1: number, z: number, h: number, cL: string, cR: string, cT: string | null) {
  q(G, [[x0, y1, z], [x1, y1, z], [x1, y1, z + h], [x0, y1, z + h]], cL);
  q(G, [[x1, y0, z], [x1, y1, z], [x1, y1, z + h], [x1, y0, z + h]], cR);
  if (cT) q(G, [[x0, y0, z + h], [x1, y0, z + h], [x1, y1, z + h], [x0, y1, z + h]], cT);
}
/** A lit box: left face full colour, right face ×0.72, top ×1.12 (or `top`; null = open top). */
function wbox(G: Geo, x0: number, y0: number, x1: number, y1: number, z: number, h: number, c: string, top?: string | null) {
  box(G, x0, y0, x1, y1, z, h, c, sh(c, .72), top === undefined ? sh(c, 1.12) : top);
}
function winsL(G: Geo, x0: number, x1: number, y: number, z0: number, h: number, col: string, step = 5, w = .045) {
  const n = Math.max(1, Math.round((x1 - x0) / .24)); const rows = Math.max(1, Math.floor((h - 2) / step));
  for (let r = 0; r < rows; r++) {
    const z = z0 + 2 + r * step;
    for (let k = 0; k < n; k++) { const x = x0 + (k + .5) * (x1 - x0) / n; q(G, [[x - w, y, z], [x + w, y, z], [x + w, y, z + 2.2], [x - w, y, z + 2.2]], col); }
  }
}
function winsR(G: Geo, y0: number, y1: number, x: number, z0: number, h: number, col: string, step = 5, w = .045) {
  const n = Math.max(1, Math.round((y1 - y0) / .24)); const rows = Math.max(1, Math.floor((h - 2) / step));
  for (let r = 0; r < rows; r++) {
    const z = z0 + 2 + r * step;
    for (let k = 0; k < n; k++) { const y = y0 + (k + .5) * (y1 - y0) / n; q(G, [[x, y - w, z], [x, y + w, z], [x, y + w, z + 2.2], [x, y - w, z + 2.2]], col); }
  }
}
function hip(G: Geo, x0: number, y0: number, x1: number, y1: number, z: number, rh: number, c: string, e = .07) {
  x0 -= e; y0 -= e; x1 += e; y1 += e; const lx = x1 - x0, ly = y1 - y0;
  if (lx >= ly) {
    const r = ly / 2, ym = (y0 + y1) / 2; const a: V3 = [x0 + r, ym, z + rh], b: V3 = [x1 - r, ym, z + rh];
    q(G, [[x0, y0, z], [x1, y0, z], b, a], sh(c, .7)); q(G, [[x0, y0, z], [x0, y1, z], a], sh(c, .8));
    q(G, [[x0, y1, z], [x1, y1, z], b, a], sh(c, 1.08)); q(G, [[x1, y0, z], [x1, y1, z], b], sh(c, .8));
  } else {
    const r = lx / 2, xm = (x0 + x1) / 2; const a: V3 = [xm, y0 + r, z + rh], b: V3 = [xm, y1 - r, z + rh];
    q(G, [[x0, y0, z], [x0, y1, z], b, a], sh(c, .72)); q(G, [[x0, y0, z], [x1, y0, z], a], sh(c, .72));
    q(G, [[x1, y0, z], [x1, y1, z], b, a], sh(c, .82)); q(G, [[x0, y1, z], [x1, y1, z], b], sh(c, 1.08));
  }
}
function tips(G: Geo, x0: number, y0: number, x1: number, y1: number, z: number, c: string, e = .14) {
  const cx = G.ctx; cx.fillStyle = sh(c, .9); const s = 2 * G.zs;
  for (const [x, y, dx] of [[x0 - e, y1 + e, -1], [x1 + e, y1 + e, 0], [x1 + e, y0 - e, 1]] as V3[]) {
    const [sx, sy] = P(G, x, y, z);
    cx.beginPath(); cx.moveTo(sx - s, sy); cx.lineTo(sx + dx * s * 1.2, sy - s * 1.4); cx.lineTo(sx + s, sy); cx.closePath(); cx.fill();
  }
}
function gable(G: Geo, x0: number, y0: number, x1: number, y1: number, z: number, rh: number, axis: "x" | "y", c: string, cEnd: string, e = .06) {
  if (axis === "x") {
    const ym = (y0 + y1) / 2; const X0 = x0 - e, X1 = x1 + e, Y0 = y0 - e, Y1 = y1 + e;
    q(G, [[X0, Y0, z], [X1, Y0, z], [X1, ym, z + rh], [X0, ym, z + rh]], sh(c, .7));
    q(G, [[x1, y0, z], [x1, y1, z], [x1, ym, z + rh]], cEnd);
    q(G, [[X0, Y1, z], [X1, Y1, z], [X1, ym, z + rh], [X0, ym, z + rh]], sh(c, 1.05));
  } else {
    const xm = (x0 + x1) / 2; const X0 = x0 - e, X1 = x1 + e, Y0 = y0 - e, Y1 = y1 + e;
    q(G, [[X0, Y0, z], [X0, Y1, z], [xm, Y1, z + rh], [xm, Y0, z + rh]], sh(c, .72));
    q(G, [[x0, y1, z], [x1, y1, z], [xm, y1, z + rh]], cEnd);
    q(G, [[X1, Y0, z], [X1, Y1, z], [xm, Y1, z + rh], [xm, Y0, z + rh]], sh(c, .88));
  }
}
function stepGable(G: Geo, x0: number, y0: number, x1: number, y1: number, z: number, rh: number, c: string, wall: string) {
  const xm = (x0 + x1) / 2;
  q(G, [[x0, y0, z], [x0, y1, z], [xm, y1, z + rh], [xm, y0, z + rh]], sh(c, .72));
  q(G, [[x1, y0, z], [x1, y1, z], [xm, y1, z + rh], [xm, y0, z + rh]], sh(c, .88));
  const n = 4; const pts: V3[] = [[x0, y1, z]];
  for (let k = 0; k < n; k++) { const zz = z + rh * (k + 1) / n + 1.2; const xa = x0 + (xm - x0) * k / n, xb = x0 + (xm - x0) * (k + 1) / n; pts.push([xa, y1, zz], [xb, y1, zz]); }
  pts.push([xm + (xm - x0) / n * .5, y1, z + rh + 1.2]);
  for (let k = n - 1; k >= 0; k--) { const zz = z + rh * (k + 1) / n + 1.2; const xa = x1 - (x1 - xm) * k / n, xb = x1 - (x1 - xm) * (k + 1) / n; pts.push([xb, y1, zz], [xa, y1, zz]); }
  pts.push([x1, y1, z]);
  q(G, pts, wall);
  q(G, [[xm - .05, y1, z + rh * .45], [xm + .05, y1, z + rh * .45], [xm + .05, y1, z + rh * .45 + 2], [xm - .05, y1, z + rh * .45 + 2]], "#2a2420");
}
function pyr(G: Geo, x0: number, y0: number, x1: number, y1: number, z: number, h: number, c: string) {
  const a: V3 = [(x0 + x1) / 2, (y0 + y1) / 2, z + h];
  q(G, [[x0, y0, z], [x1, y0, z], a], sh(c, .7)); q(G, [[x0, y0, z], [x0, y1, z], a], sh(c, .75));
  q(G, [[x0, y1, z], [x1, y1, z], a], sh(c, 1.08)); q(G, [[x1, y0, z], [x1, y1, z], a], sh(c, .82));
}
function flatTop(G: Geo, x0: number, y0: number, x1: number, y1: number, z: number, c: string) {
  q(G, [[x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]], sh(c, 1.1)); const d = .07;
  q(G, [[x0 + d, y0 + d, z], [x1 - d, y0 + d, z], [x1 - d, y1 - d, z], [x0 + d, y1 - d, z]], sh(c, .92));
}
function crenels(G: Geo, x0: number, y0: number, x1: number, y1: number, z: number, c: string) {
  const n = Math.max(2, Math.round((x1 - x0) / .2));
  for (let k = 0; k < n; k += 2) { const x = x0 + k * (x1 - x0) / n; wbox(G, x, y1 - .1, x + (x1 - x0) / n, y1, z, 2, c); }
  const m = Math.max(2, Math.round((y1 - y0) / .2));
  for (let k = 0; k < m; k += 2) { const y = y0 + k * (y1 - y0) / m; wbox(G, x1 - .1, y, x1, y + (y1 - y0) / m, z, 2, c); }
}
function ell(G: Geo, x: number, y: number, z: number, r: number): [number, number, number, number] {
  const [sx, sy] = P(G, x, y, z); const rx = r * G.TW * .72; return [sx, sy, rx, rx / 2];
}
function cyl(G: Geo, x: number, y: number, z: number, r: number, h: number, c: string, top?: string | null) {
  const X = G.ctx; const [sx, sy, rx, ry] = ell(G, x, y, z, r); const ty = sy - h * G.zs;
  const g = X.createLinearGradient(sx - rx, 0, sx + rx, 0);
  g.addColorStop(0, sh(c, 1.08)); g.addColorStop(.45, c); g.addColorStop(1, sh(c, .62));
  X.fillStyle = g; X.beginPath(); X.moveTo(sx - rx, ty); X.lineTo(sx - rx, sy); X.ellipse(sx, sy, rx, ry, 0, Math.PI, 0, true); X.lineTo(sx + rx, ty); X.closePath(); X.fill();
  if (top !== null) { X.fillStyle = top || sh(c, 1.12); X.beginPath(); X.ellipse(sx, ty, rx, ry, 0, 0, Math.PI * 2); X.fill(); }
}
function cone(G: Geo, x: number, y: number, z: number, r: number, h: number, c: string) {
  const X = G.ctx; const [sx, sy, rx, ry] = ell(G, x, y, z, r); const ty = sy - h * G.zs;
  const g = X.createLinearGradient(sx - rx, 0, sx + rx, 0);
  g.addColorStop(0, sh(c, 1.14)); g.addColorStop(.5, c); g.addColorStop(1, sh(c, .6));
  X.fillStyle = g; X.beginPath(); X.moveTo(sx, ty); X.lineTo(sx - rx, sy); X.ellipse(sx, sy, rx, ry, 0, Math.PI, 0, true); X.closePath(); X.fill();
}
function dome(G: Geo, x: number, y: number, z: number, r: number, c: string): number {
  const X = G.ctx; const [sx, sy, rx, ry] = ell(G, x, y, z, r);
  const g = X.createRadialGradient(sx - rx * .35, sy - rx * .55, rx * .1, sx, sy - rx * .2, rx * 1.15);
  g.addColorStop(0, sh(c, 1.4)); g.addColorStop(.55, c); g.addColorStop(1, sh(c, .58));
  X.fillStyle = g; X.beginPath(); X.moveTo(sx - rx, sy); X.ellipse(sx, sy, rx, ry, 0, Math.PI, 0, true); X.ellipse(sx, sy, rx, rx * .85, 0, 0, Math.PI, true); X.closePath(); X.fill();
  return (rx * .85) / G.zs;
}
function flag(G: Geo, x: number, y: number, z: number, col: string) {
  const X = G.ctx; const [sx, sy] = P(G, x, y, z); const s = G.zs;
  X.strokeStyle = "#2a2622"; X.lineWidth = Math.max(.8, s * .9);
  X.beginPath(); X.moveTo(sx, sy); X.lineTo(sx, sy - 8 * s); X.stroke();
  X.fillStyle = col; X.beginPath(); X.moveTo(sx, sy - 8 * s); X.lineTo(sx + 5 * s, sy - 6.8 * s); X.lineTo(sx, sy - 5.4 * s); X.closePath(); X.fill();
}
function smoke(G: Geo, x: number, y: number, z: number) {
  const X = G.ctx; const [sx, sy] = P(G, x, y, z);
  for (let k = 0; k < 4; k++) { X.fillStyle = `rgba(210,210,215,${.34 - k * .07})`; X.beginPath(); X.arc(sx + k * 1.6 * G.zs, sy - k * 2.6 * G.zs, (1.4 + k * .6) * G.zs, 0, Math.PI * 2); X.fill(); }
}
function scaffold(G: Geo, x0: number, y0: number, x1: number, y1: number, h: number) {
  const X = G.ctx; X.strokeStyle = "#d8b878"; X.lineWidth = Math.max(.7, .8 * G.zs);
  const L = (a: V3, b: V3) => { const p = P(G, ...a), r = P(G, ...b); X.beginPath(); X.moveTo(p[0], p[1]); X.lineTo(r[0], r[1]); X.stroke(); };
  for (const [x, y] of [[x0, y1], [x1, y1], [x1, y0]]) L([x, y, 0], [x, y, h]);
  for (let z = 4; z <= h; z += 4) { L([x0, y1, z], [x1, y1, z]); L([x1, y1, z], [x1, y0, z]); }
  L([x0, y1, 0], [x1, y1, h]);
}

// ── vegetation, ships, caravans, people (screen-space sprites) ────────────────

function tree(G: Geo, x: number, y: number, kind: TreeKind) {
  const X = G.ctx; const [sx, sy] = P(G, x, y, 0); const s = G.zs;
  X.fillStyle = "rgba(20,30,20,.22)"; X.beginPath(); X.ellipse(sx + 2 * s, sy + .5 * s, 3.4 * s, 1.5 * s, 0, 0, Math.PI * 2); X.fill();
  const trunk = (h: number, c?: string) => { X.fillStyle = c || "#5a4030"; X.fillRect(sx - .6 * s, sy - h * s, 1.2 * s, h * s); };
  const blob = (cx: number, cy: number, r: number, c: string) => { X.fillStyle = c; X.beginPath(); X.arc(sx + cx * s, sy + cy * s, r * s, 0, Math.PI * 2); X.fill(); };
  if (kind === "conifer" || kind === "pine" || kind === "snowpine") {
    trunk(3); const base = kind === "pine" ? "#3f6a44" : "#2f5a3a";
    for (let k = 0; k < 3; k++) {
      const w = (4.2 - k * 1.1) * s, y0 = sy - (2 + k * 3.2) * s;
      X.fillStyle = k % 2 ? sh(base, 1.12) : base; X.beginPath(); X.moveTo(sx - w, y0); X.lineTo(sx, y0 - 5 * s); X.lineTo(sx + w, y0); X.closePath(); X.fill();
      if (kind === "snowpine") { X.fillStyle = "#f2f6f8"; X.beginPath(); X.moveTo(sx - w * .5, y0 - 2.4 * s); X.lineTo(sx, y0 - 5 * s); X.lineTo(sx + w * .4, y0 - 2.6 * s); X.closePath(); X.fill(); }
    }
  } else if (kind === "cypress") {
    trunk(2); X.fillStyle = "#34563a"; X.beginPath(); X.ellipse(sx, sy - 6.5 * s, 1.9 * s, 5.5 * s, 0, 0, Math.PI * 2); X.fill();
    X.fillStyle = "#46704a"; X.beginPath(); X.ellipse(sx - .6 * s, sy - 7.5 * s, .9 * s, 3.8 * s, 0, 0, Math.PI * 2); X.fill();
  } else if (kind === "poplar") {
    trunk(2); X.fillStyle = "#7a9a3e"; X.beginPath(); X.ellipse(sx, sy - 6 * s, 1.8 * s, 4.8 * s, 0, 0, Math.PI * 2); X.fill();
  } else if (kind === "olive") {
    trunk(2.5, "#6a5a4a"); blob(-1.4, -4, 2.6, "#7a8a60"); blob(1.4, -4.4, 2.4, "#8a9a6c"); blob(0, -5.6, 2.2, "#9aa87a");
  } else if (kind === "palm") {
    X.strokeStyle = "#7a5e3e"; X.lineWidth = 1.2 * s; X.beginPath(); X.moveTo(sx, sy); X.quadraticCurveTo(sx + 2 * s, sy - 5 * s, sx + 1 * s, sy - 10 * s); X.stroke();
    X.strokeStyle = "#3f7a3a"; X.lineWidth = 1.3 * s;
    for (const [dx, dy] of [[-5, 1], [5, 1.5], [-3.5, -2], [4, -2], [0, -3]]) {
      X.beginPath(); X.moveTo(sx + 1 * s, sy - 10 * s); X.quadraticCurveTo(sx + (1 + dx * .6) * s, sy + (-10 + dy - 2) * s, sx + (1 + dx) * s, sy + (-10 + dy + 1.5) * s); X.stroke();
    }
  } else if (kind === "jungle") {
    trunk(2, "#4a3a28"); blob(-2, -4, 3.4, "#2f6a34"); blob(2, -4.5, 3.2, "#357a3a"); blob(0, -7, 3.2, "#3f8a44"); blob(-.8, -7.8, 1.4, "#5aa054");
  } else {
    trunk(3); blob(-1.6, -5, 2.8, "#4a7a3a"); blob(1.6, -5.2, 2.8, "#548640"); blob(0, -7.2, 2.8, "#5f9448"); blob(-.6, -8, 1.2, "#78aa58");
  }
}
function ship(G: Geo, x: number, y: number, type: ShipKind, dir: number) {
  const X = G.ctx; const [sx, sy] = P(G, x, y, 0); const s = G.zs; X.save(); X.translate(sx, sy); X.scale(s, s);
  X.strokeStyle = "rgba(235,245,250,.5)"; X.lineWidth = .8; X.beginPath(); X.moveTo(-8 * dir, -2); X.lineTo(-15 * dir, -5.5); X.stroke();
  const hull = (L: number, H: number, c?: string, stern?: number) => {
    X.save(); X.transform(1, .5 * dir, 0, 1, 0, 0); X.fillStyle = c || "#5a3a22"; X.beginPath();
    X.moveTo(-L, -(stern || 0)); X.lineTo(L, -(stern || 0) * .3); X.lineTo(L * .8, H); X.lineTo(-L * .8, H); X.closePath(); X.fill();
    X.fillStyle = "rgba(255,240,210,.25)"; X.fillRect(-L * .8, -.2, L * 1.6, .7); X.restore();
  };
  const mast = (mx: number, h: number) => { X.strokeStyle = "#3a2a1c"; X.lineWidth = .8; X.beginPath(); X.moveTo(mx, mx * .5 * dir); X.lineTo(mx, mx * .5 * dir - h); X.stroke(); };
  const sq = (mx: number, y0: number, w: number, h: number, c: string, stripe?: string) => {
    X.fillStyle = c; X.fillRect(mx - w / 2, mx * .5 * dir + y0, w, h);
    if (stripe) { X.fillStyle = stripe; X.fillRect(mx - w / 2, mx * .5 * dir + y0 + h * .4, w, h * .22); }
  };
  const lat = (mx: number, h: number, w: number, c: string) => {
    X.fillStyle = c; X.beginPath(); X.moveTo(mx, mx * .5 * dir - h); X.lineTo(mx + w * dir, mx * .5 * dir - 1.5); X.lineTo(mx - w * .6 * dir, mx * .5 * dir - 1); X.closePath(); X.fill();
  };
  if (type === "cog" || type === "hulk") { const L = type === "hulk" ? 8 : 6.5; hull(L, 3, "#5a3a22", 1.5); mast(0, 11); sq(0, -10.5, 7, 7, "#efe6d0", "#a8382e"); }
  else if (type === "carrack") { hull(8.5, 3.2, "#4e321e", 2.5); mast(-3.5, 9); mast(1, 13); mast(5, 8); sq(1, -12.5, 7, 6, "#efe6d0"); sq(-3.5, -8.5, 5, 4.5, "#e8dcc4"); lat(5, 8, 3, "#efe6d0"); }
  else if (type === "galley") {
    hull(10, 1.8, "#6a3a26"); X.strokeStyle = "#3a2a1c"; X.lineWidth = .6;
    for (let k = -7; k <= 7; k += 2) { X.beginPath(); X.moveTo(k, k * .5 * dir + 1.6); X.lineTo(k - 1.4 * dir, k * .5 * dir + 4); X.stroke(); }
    mast(1, 11); lat(1, 11, 6, "#f0e8d4");
  } else if (type === "junk") {
    hull(7.5, 3, "#5a3a26", 2.6); mast(-2.5, 10); mast(2.5, 13);
    for (const [mx, h] of [[-2.5, 9], [2.5, 12]]) {
      X.fillStyle = "#b8683a"; X.fillRect(mx - 3, mx * .5 * dir - h, 6, h - 2); X.strokeStyle = "#7a3e22"; X.lineWidth = .5;
      for (let k = 1; k < h - 2; k += 2) { X.beginPath(); X.moveTo(mx - 3, mx * .5 * dir - h + k); X.lineTo(mx + 3, mx * .5 * dir - h + k); X.stroke(); }
    }
  } else if (type === "sampan") { hull(4.5, 1.8, "#6a5238"); X.fillStyle = "#6a5a3a"; X.beginPath(); X.ellipse(0, -1, 2.6, 2, 0, Math.PI, 0); X.fill(); }
  else if (type === "dhow") { hull(7, 2.6, "#7a5230", 1.8); mast(0, 12); lat(0, 12, 7, "#f2ead6"); }
  else if (type === "prahu") {
    hull(6.5, 1.6, "#6a4a2a"); X.strokeStyle = "#4a3420"; X.lineWidth = .7; X.beginPath(); X.moveTo(-3, -1.5 + 2); X.lineTo(-3, 4.5); X.moveTo(3, 1.5 + 2); X.lineTo(3, 7); X.stroke();
    X.fillStyle = "#6a4a2a"; X.fillRect(-6, 4 + (-1.5), 12, 1); mast(0, 10);
    X.fillStyle = "#d8b870"; X.beginPath(); X.moveTo(-1, -1); X.quadraticCurveTo(-6, -7, -2, -12); X.lineTo(4, -8); X.quadraticCurveTo(1, -5, -1, -1); X.fill();
  } else if (type === "knarr") { hull(6.5, 2.6, "#4e3a2a", 2); mast(0, 10); sq(0, -10, 7, 6.5, "#efe6d0", "#a8382e"); }
  else { hull(5, 1.4, "#6a5238"); X.fillStyle = "#b8905a"; X.fillRect(-3, -2.5, 2.5, 2); X.fillRect(.5, -2.2, 2.5, 2); }
  X.restore();
}
function caravan(G: Geo, x: number, y: number, kind: CaravanKind, n: number, dir: number) {
  const X = G.ctx; const s = G.zs;
  for (let k = 0; k < n; k++) {
    const [sx, sy] = P(G, x + k * .34 * dir, y, 0); X.save(); X.translate(sx, sy); X.scale(s, s);
    if (kind === "camel") {
      X.fillStyle = "#b08a58"; X.fillRect(-2, -4, 4, 2); X.fillRect(-1, -5, 2, 1); X.fillRect(2, -6, 1, 3); X.fillRect(2, -6.5, 2, 1);
      X.fillStyle = "#6a5238"; X.fillRect(-2, -2, .8, 2); X.fillRect(1.2, -2, .8, 2);
      if (k % 2 === 0) { X.fillStyle = "#a8382e"; X.fillRect(-1.6, -5, 1.6, 1); }
    } else if (kind === "horse") {
      X.fillStyle = "#6a4428"; X.fillRect(-2, -3.5, 4, 1.8); X.fillRect(1.6, -5, 1.2, 2);
      X.fillStyle = "#3a2618"; X.fillRect(-2, -1.7, .8, 1.7); X.fillRect(1.2, -1.7, .8, 1.7);
      if (k % 3 === 0) { X.fillStyle = "#d8c8a0"; X.fillRect(-1, -5.2, 1.4, 1.8); }
    } else {
      X.fillStyle = "#7a5a3a"; X.fillRect(-2.5, -3.5, 5, 2.5); X.fillStyle = "#e8dcc0"; X.beginPath(); X.ellipse(0, -3.5, 2.6, 1.8, 0, Math.PI, 0); X.fill();
      X.fillStyle = "#3a2618"; X.fillRect(-2, -1, 1.2, 1.2); X.fillRect(1, -1, 1.2, 1.2);
    }
    X.restore();
  }
}
function person(G: Geo, x: number, y: number, col: string, skin: string) {
  const X = G.ctx; const [sx, sy] = P(G, x, y, 0); const s = G.zs;
  X.fillStyle = col; X.fillRect(sx - .5 * s, sy - 2.4 * s, 1.1 * s, 2.4 * s);
  X.fillStyle = skin; X.fillRect(sx - .5 * s, sy - 3.4 * s, 1.1 * s, 1 * s);
}

// ── houses (one tile) ─────────────────────────────────────────────────────────

function house(G: Geo, i: number, j: number, S: CityStyle, r: () => number): number {
  const f = S.house; const wall = pick(r, S.walls), roof = pick(r, S.roofs); const dk = "#2c2620", lit = "#f0c060";
  const winC = () => r() < .25 ? lit : dk;
  const ox = (r() - .5) * .08, oy = (r() - .5) * .08; const x0 = i + .14 + ox, y0 = j + .14 + oy, x1 = i + .86 + ox, y1 = j + .86 + oy;
  if (f === "med") {
    const h = 7 + Math.floor(r() * 6); wbox(G, x0, y0, x1, y1, 0, h, wall); winsL(G, x0, x1, y1, 0, h, winC(), 5); winsR(G, y0, y1, x1, 0, h, dk, 5);
    if (r() < .18) { flatTop(G, x0, y0, x1, y1, h, wall); if (r() < .5) tree(G, x0 + .25, y0 + .3, "olive"); } else hip(G, x0, y0, x1, y1, h, 3, roof, .06);
    return h + 4;
  }
  if (f === "hansa") {
    const h = 10 + Math.floor(r() * 6); const X0 = i + .2, X1 = i + .8; wbox(G, X0, y0, X1, y1, 0, h, wall); winsL(G, X0, X1, y1, 0, h, winC(), 4); winsR(G, y0, y1, X1, 0, h, dk, 4);
    if (r() < .65) stepGable(G, X0, y0, X1, y1, h, 8, roof, wall); else gable(G, X0, y0, X1, y1, h, 8, "y", roof, wall, .05);
    return h + 9;
  }
  if (f === "yurt") {
    if (r() < .18) { wbox(G, x0, y0, x1, y1, 0, 5, "#8a6a48"); gable(G, x0, y0, x1, y1, 5, 3, "x", "#6a5238", sh("#8a6a48", .72)); return 8; }
    const cx = i + .5, cy = j + .5, rr = .3 + r() * .06; cyl(G, cx, cy, 0, rr, 4, wall, null);
    q(G, [[cx - .06, cy + rr * .96, 0], [cx + .06, cy + rr * .96, 0], [cx + .06, cy + rr * .96, 3], [cx - .06, cy + rr * .96, 3]], "#b8423a");
    cone(G, cx, cy, 4, rr + .03, 4, roof); const [sx, sy] = P(G, cx, cy, 8); G.ctx.fillStyle = "#5a4a3a"; G.ctx.fillRect(sx - .6 * G.zs, sy - .4 * G.zs, 1.2 * G.zs, .8 * G.zs);
    return 8;
  }
  if (f === "flat") {
    const h = 6 + Math.floor(r() * 6); wbox(G, x0, y0, x1, y1, 0, h, wall, null); winsL(G, x0, x1, y1, 0, h, dk, 5, .035); flatTop(G, x0, y0, x1, y1, h, wall);
    const v = r();
    if (v < .2) dome(G, (x0 + x1) / 2, (y0 + y1) / 2, h, .22, sh(wall, 1.12));
    else if (v < .32) wbox(G, x1 - .2, y0, x1, y0 + .2, h, 6, wall);
    else if (v < .5) q(G, [[x0 + .1, y0 + .1, h + 2.5], [x0 + .45, y0 + .1, h + 2.5], [x0 + .45, y0 + .45, h + 2.5], [x0 + .1, y0 + .45, h + 2.5]], pick(r, ["#b8423a", "#3a7a9a", "#d8a040", "#6a8a4a"]));
    return h + 3;
  }
  if (f === "east") {
    const h = 6; wbox(G, x0, y0, x1, y1, 0, h, wall); q(G, [[x0, y1, 0], [x1, y1, 0], [x1, y1, 1.4], [x0, y1, 1.4]], "#6a4a34");
    winsL(G, x0, x1, y1, 0, h, "#6a3a2a", 8, .08); hip(G, x0, y0, x1, y1, h, 4, roof, .14); tips(G, x0, y0, x1, y1, h, roof);
    return h + 5;
  }
  if (f === "stilt") {
    const X = G.ctx; X.strokeStyle = "#5a4028"; X.lineWidth = Math.max(.8, G.zs);
    for (const [x, y] of [[x0, y1], [x1, y1], [x1, y0]]) { const a = P(G, x, y, 0), b = P(G, x, y, 4); X.beginPath(); X.moveTo(a[0], a[1]); X.lineTo(b[0], b[1]); X.stroke(); }
    wbox(G, x0, y0, x1, y1, 4, 5, wall); winsL(G, x0, x1, y1, 4, 5, "#3a2a1a", 6, .08); hip(G, x0, y0, x1, y1, 9, 7, roof, .16);
    return 16;
  }
  if (f === "chalet") {
    wbox(G, x0, y0, x1, y1, 0, 4, "#9a968e"); wbox(G, x0, y0, x1, y1, 4, 5, "#7a4e2e"); winsL(G, x0, x1, y1, 4, 5, winC(), 6, .06);
    const snow = r() < .55; gable(G, x0, y0, x1, y1, 9, 4, "x", snow ? "#e8eef2" : roof, sh("#7a4e2e", .72), .14);
    return 13;
  }
  if (f === "turf") {
    wbox(G, x0, y0 + .08, x1, y1 - .08, 0, 3, wall); gable(G, x0, y0 + .08, x1, y1 - .08, 3, 4, "x", pick(r, S.roofs), sh(wall, .72), .1);
    q(G, [[x0, y1 - .02, 3], [x1, y1 - .02, 3], [x1, y1 - .02, 3.8], [x0, y1 - .02, 3.8]], "#6f8a5a");
    if (r() < .4) smoke(G, x1 - .2, (y0 + y1) / 2, 8);
    return 8;
  }
  return 6;
}

// ── landmarks (2×2 footprint at a,b) ─────────────────────────────────────────

/** Every landmark kind the renderer knows how to draw. Anything else falls to a
 *  generic civic block. */
export type LandmarkKind =
  | "Cathedral" | "Temple" | "Palace" | "Citadel" | "Guildhall" | "Council Hall" | "Bank" | "Mint"
  | "Granary" | "Warehouse" | "Fondaco" | "Workshop" | "Harbor" | "Shipyard" | string;
export type SlotStatus = "op" | "build" | "site" | "lock";

interface PlacedLandmark { a: number; b: number; kind: LandmarkKind; status: SlotStatus; color: string; num: number; ref: number }

function roofFor(G: Geo, S: CityStyle, x0: number, y0: number, x1: number, y1: number, z: number, rh: number, col: string | null, wall: string) {
  const f = S.house; const c = col || S.civRoof;
  if (f === "med") hip(G, x0, y0, x1, y1, z, rh * .55, c, .06);
  else if (f === "hansa") gable(G, x0, y0, x1, y1, z, rh * 1.2, "x", c, sh(wall, .72), .05);
  else if (f === "flat") flatTop(G, x0, y0, x1, y1, z, wall);
  else if (f === "east") { hip(G, x0, y0, x1, y1, z, rh * .7, c, .16); tips(G, x0, y0, x1, y1, z, c, .16); }
  else if (f === "stilt") hip(G, x0, y0, x1, y1, z, rh * 1.2, "#c8a45a", .18);
  else if (f === "chalet") gable(G, x0, y0, x1, y1, z, rh * .6, "x", "#e8eef2", sh(wall, .72), .14);
  else if (f === "turf") gable(G, x0, y0, x1, y1, z, rh * .8, "x", "#eef2f4", sh(wall, .72), .1);
  else hip(G, x0, y0, x1, y1, z, rh * .5, c, .08);
}
function religious(G: Geo, a: number, b: number, S: CityStyle): number {
  const f = S.house, st = S.stone, dk = "#2c2620";
  if (f === "med") {
    wbox(G, a + 1.45, b + .1, a + 1.85, b + .5, 0, 30, st); winsR(G, b + .1, b + .5, a + 1.85, 20, 8, dk, 5, .06); pyr(G, a + 1.43, b + .08, a + 1.87, b + .52, 30, 6, S.civRoof);
    wbox(G, a + .15, b + .6, a + 1.85, b + 1.4, 0, 13, st); winsL(G, a + .25, a + 1.75, b + 1.4, 0, 13, dk, 6, .06); hip(G, a + .15, b + .6, a + 1.85, b + 1.4, 13, 4, S.civRoof, .05);
    wbox(G, a + .65, b + .2, a + 1.35, b + 1.8, 0, 13, st); hip(G, a + .65, b + .2, a + 1.35, b + 1.8, 13, 4, S.civRoof, .05);
    cyl(G, a + 1, b + 1, 16, .3, 5, st); const dz = dome(G, a + 1, b + 1, 21, .32, "#c8b48a"); cyl(G, a + 1, b + 1, 21 + dz, .07, 3, st);
    return 32;
  }
  if (f === "hansa" || f === "alpine") {
    const wall = f === "hansa" ? "#9c4a33" : "#eceae2";
    wbox(G, a + .15, b + .7, a + .7, b + 1.3, 0, 26, wall); winsL(G, a + .2, a + .65, b + 1.3, 14, 10, dk, 5, .06);
    if (f === "hansa") pyr(G, a + .13, b + .68, a + .72, b + 1.32, 26, 20, "#5f8a78");
    else { const dz = dome(G, a + .42, b + 1, 26, .26, "#4f6f58"); cone(G, a + .42, b + 1, 26 + dz, .06, 6, "#d4a83a"); }
    wbox(G, a + .7, b + .5, a + 1.9, b + 1.5, 0, f === "hansa" ? 15 : 11, wall); winsL(G, a + .8, a + 1.8, b + 1.5, 0, 15, dk, 7, .05);
    gable(G, a + .7, b + .5, a + 1.9, b + 1.5, f === "hansa" ? 15 : 11, f === "hansa" ? 11 : 7, "x", f === "hansa" ? "#4a3a34" : "#7a3a30", sh(wall, .72), .05);
    return 46;
  }
  if (f === "yurt") {
    wbox(G, a + .25, b + .25, a + 1.75, b + 1.75, 0, 4, "#e8e2d2"); wbox(G, a + .45, b + .45, a + 1.55, b + 1.55, 4, 3, "#f0ebe0");
    cyl(G, a + 1, b + 1, 7, .4, 3, "#f2eee4"); const dz = dome(G, a + 1, b + 1, 10, .44, "#f4f0e6"); cone(G, a + 1, b + 1, 10 + dz, .12, 12, "#d4a83a");
    return 30;
  }
  if (f === "flat") {
    cyl(G, a + 1.7, b + .3, 0, .13, 32, "#eadbc0"); cyl(G, a + 1.7, b + .3, 24, .19, 1.5, "#d8c8a8"); dome(G, a + 1.7, b + .3, 32, .14, "#3a9e98");
    wbox(G, a + .15, b + .15, a + 1.85, b + 1.85, 0, 10, "#ecdcc0", null); winsL(G, a + .25, a + 1.75, b + 1.85, 0, 10, "#5a4a38", 9, .08); winsR(G, b + .25, b + 1.75, a + 1.85, 0, 10, "#4a3a2a", 9, .08);
    flatTop(G, a + .15, b + .15, a + 1.85, b + 1.85, 10, "#ecdcc0"); cyl(G, a + 1, b + 1, 10, .42, 3, "#e8d8bc"); dome(G, a + 1, b + 1, 13, .5, "#3a9e98");
    return 40;
  }
  if (f === "east") {
    wbox(G, a + .2, b + .2, a + 1.8, b + 1.8, 0, 2, "#b8b0a0");
    for (let k = 0; k < 5; k++) {
      const w = .55 - .08 * k, z0 = 2 + k * 7;
      wbox(G, a + 1 - w, b + 1 - w, a + 1 + w, b + 1 + w, z0, 4.5, "#a8382e"); hip(G, a + 1 - w, b + 1 - w, a + 1 + w, b + 1 + w, z0 + 4.5, 2.6, S.civRoof, .16); tips(G, a + 1 - w, b + 1 - w, a + 1 + w, b + 1 + w, z0 + 4.5, S.civRoof, .16);
    }
    cone(G, a + 1, b + 1, 37, .06, 7, "#d4a83a");
    return 44;
  }
  if (f === "stilt") {
    const c = "#8e7c64";
    wbox(G, a + .1, b + .1, a + 1.9, b + 1.9, 0, 5, c); wbox(G, a + .35, b + .35, a + 1.65, b + 1.65, 5, 5, sh(c, 1.06)); wbox(G, a + .58, b + .58, a + 1.42, b + 1.42, 10, 5, sh(c, 1.12));
    wbox(G, a + .76, b + .76, a + 1.24, b + 1.24, 15, 6, c); q(G, [[a + .92, b + 1.24, 15], [a + 1.08, b + 1.24, 15], [a + 1.08, b + 1.24, 19], [a + .92, b + 1.24, 19]], "#2a2018");
    cone(G, a + 1, b + 1, 21, .3, 12, c); for (const [x, y] of [[.3, 1.7], [1.7, 1.7], [1.7, .3]]) cone(G, a + x, b + y, 5, .1, 5, c);
    return 36;
  }
  if (f === "chalet") return religious(G, a, b, { ...S, house: "alpine" });
  if (f === "turf") {
    const w = "#4a3528"; wbox(G, a + .35, b + .35, a + 1.65, b + 1.65, 0, 8, w); gable(G, a + .35, b + .35, a + 1.65, b + 1.65, 8, 7, "x", "#3a2a20", sh(w, .72), .1);
    wbox(G, a + .7, b + .7, a + 1.3, b + 1.3, 13, 5, w); gable(G, a + .7, b + .7, a + 1.3, b + 1.3, 18, 6, "y", "#3a2a20", sh(w, .9), .08); pyr(G, a + .9, b + .9, a + 1.1, b + 1.1, 24, 7, "#3a2a20");
    return 32;
  }
  return 20;
}

function landmark(G: Geo, L: { a: number; b: number; kind: LandmarkKind; status: SlotStatus; color: string }, S: CityStyle): number {
  const a = L.a, b = L.b, k = L.kind, f = S.house, dk = "#2c2620";
  const wall = f === "hansa" ? "#9c4a33" : f === "flat" ? "#e4cc9c" : f === "east" ? "#e8e4d8" : f === "stilt" ? "#a07e48" : f === "turf" ? "#5e4e3e" : f === "chalet" ? "#9a968e" : f === "yurt" ? "#8a6a48" : "#eadfc6";
  if (L.status === "build") {
    wbox(G, a + .25, b + .25, a + 1.75, b + 1.75, 0, 8, wall); scaffold(G, a + .18, b + .18, a + 1.82, b + 1.82, 14);
    q(G, [[a + 1.9, b + 1.9, 0], [a + 2, b + 1.9, 0], [a + 2, b + 2, 0], [a + 1.9, b + 2, 0]], "#c8a870");
    return 16;
  }
  if (k === "Cathedral" || k === "Temple") return religious(G, a, b, S);
  if (k === "Palace") {
    if (f === "yurt") {
      cyl(G, a + 1, b + 1, 0, .85, 7, "#f4eee0", null); q(G, [[a + .94, b + 1.85, 0], [a + 1.06, b + 1.85, 0], [a + 1.06, b + 1.85, 5], [a + .94, b + 1.85, 5]], "#b8423a");
      cone(G, a + 1, b + 1, 7, .9, 9, "#c89a3a"); cyl(G, a + 1, b + 1, 15, .18, 2, "#b8423a"); return 20;
    }
    if (f === "east") {
      wbox(G, a + .05, b + .05, a + 1.95, b + 1.95, 0, 2.5, "#b8b0a0"); wbox(G, a + .25, b + .35, a + 1.75, b + 1.65, 2.5, 7, "#a8382e");
      winsL(G, a + .3, a + 1.7, b + 1.65, 2.5, 7, "#e8d8b8", 9, .06); hip(G, a + .25, b + .35, a + 1.75, b + 1.65, 9.5, 4, S.civRoof, .18); tips(G, a + .25, b + .35, a + 1.75, b + 1.65, 9.5, S.civRoof, .18);
      wbox(G, a + .6, b + .7, a + 1.4, b + 1.3, 12, 4, "#a8382e"); hip(G, a + .6, b + .7, a + 1.4, b + 1.3, 16, 3, S.civRoof, .14); tips(G, a + .6, b + .7, a + 1.4, b + 1.3, 16, S.civRoof);
      return 22;
    }
    if (f === "stilt") {
      const X = G.ctx; X.strokeStyle = "#5a4028"; X.lineWidth = G.zs;
      for (let x = .15; x <= 1.86; x += .34) { const p = P(G, a + x, b + 1.6, 0), r2 = P(G, a + x, b + 1.6, 5); X.beginPath(); X.moveTo(p[0], p[1]); X.lineTo(r2[0], r2[1]); X.stroke(); }
      wbox(G, a + .1, b + .4, a + 1.9, b + 1.6, 5, 6, "#a07e48"); winsL(G, a + .2, a + 1.8, b + 1.6, 5, 6, "#3a2a1a", 7, .07); hip(G, a + .1, b + .4, a + 1.9, b + 1.6, 11, 10, "#c09a50", .2);
      return 24;
    }
    if (f === "flat") {
      wbox(G, a + .1, b + .1, a + 1.9, b + 1.9, 0, 12, wall, null); winsL(G, a + .2, a + 1.8, b + 1.9, 0, 12, "#5a4a38", 5, .05); flatTop(G, a + .1, b + .1, a + 1.9, b + 1.9, 12, wall);
      for (const [x, y] of [[.1, 1.9], [1.9, 1.9], [1.9, .1]]) { wbox(G, a + x - .16, b + y - .16, a + x + .16, b + y + .16, 0, 17, sh(wall, .96)); crenels(G, a + x - .16, b + y - .16, a + x + .16, b + y + .16, 17, wall); }
      dome(G, a + .8, b + .8, 12, .3, "#3a9e98"); dome(G, a + 1.3, b + 1.2, 12, .22, "#ecdcc0");
      return 24;
    }
    if (f === "hansa") {
      wbox(G, a + .1, b + .3, a + 1.9, b + 1.7, 0, 14, wall); winsL(G, a + .2, a + 1.8, b + 1.7, 0, 14, "#f0c060", 5); winsR(G, b + .4, b + 1.6, a + 1.9, 0, 14, dk, 5);
      for (let s = 0; s < 3; s++) { const x0 = a + .1 + s * .6; stepGable(G, x0, b + .3, x0 + .6, b + 1.7, 14, 9, "#5a3a30", wall); }
      return 26;
    }
    wbox(G, a + .1, b + .3, a + 1.9, b + 1.7, 0, 14, wall); winsL(G, a + .2, a + 1.8, b + 1.7, 0, 14, dk, 5); winsR(G, b + .4, b + 1.6, a + 1.9, 0, 14, dk, 5);
    roofFor(G, S, a + .1, b + .3, a + 1.9, b + 1.7, 14, 6, S.civRoof, wall);
    if (f === "med") {
      wbox(G, a + .1, b + 1.3, a + .5, b + 1.7, 0, 18, wall); crenels(G, a + .1, b + 1.3, a + .5, b + 1.7, 18, wall);
      wbox(G, a + 1.5, b + 1.3, a + 1.9, b + 1.7, 0, 18, wall); crenels(G, a + 1.5, b + 1.3, a + 1.9, b + 1.7, 18, wall);
    }
    return 24;
  }
  if (k === "Citadel") {
    const c = f === "flat" ? "#c8965e" : f === "yurt" || f === "stilt" || f === "turf" ? "#7a5a3a" : (f === "hansa" ? "#8e4a34" : "#a8a292");
    wbox(G, a + .4, b + .4, a + 1.6, b + 1.6, 0, 24, c); winsL(G, a + .5, a + 1.5, b + 1.6, 8, 16, dk, 6, .04);
    if (f === "east") { hip(G, a + .4, b + .4, a + 1.6, b + 1.6, 24, 5, S.civRoof, .16); tips(G, a + .4, b + .4, a + 1.6, b + 1.6, 24, S.civRoof); }
    else crenels(G, a + .4, b + .4, a + 1.6, b + 1.6, 24, c);
    for (const [x, y] of [[.35, 1.65], [1.65, 1.65], [1.65, .35]]) {
      cyl(G, a + x, b + y, 0, .22, 19, sh(c, 1.04));
      if (f === "hansa" || f === "chalet") cone(G, a + x, b + y, 19, .26, 9, "#5a3a30");
      else if (f === "turf" || f === "yurt" || f === "stilt") cone(G, a + x, b + y, 19, .26, 6, "#6a5238");
      else cyl(G, a + x, b + y, 19, .26, 2, sh(c, 1.1));
    }
    return 30;
  }
  if (k === "Guildhall" || k === "Council Hall") {
    const th = k === "Guildhall" ? 26 : 30;
    wbox(G, a + 1.35, b + .15, a + 1.8, b + .6, 0, th, wall); winsR(G, b + .15, b + .6, a + 1.8, 16, 12, dk, 5, .06);
    if (f === "east") hip(G, a + 1.35, b + .15, a + 1.8, b + .6, th, 4, S.civRoof, .12);
    else if (f === "flat" || f === "med") crenels(G, a + 1.35, b + .15, a + 1.8, b + .6, th, wall);
    else pyr(G, a + 1.33, b + .13, a + 1.82, b + .62, th, 9, S.civRoof);
    wbox(G, a + .15, b + .5, a + 1.85, b + 1.7, 0, 13, wall); winsL(G, a + .25, a + 1.75, b + 1.7, 0, 13, k === "Council Hall" ? "#f0c060" : dk, 5); winsR(G, b + .6, b + 1.6, a + 1.85, 0, 13, dk, 5);
    if (f === "hansa") stepGable(G, a + .15, b + .5, a + 1.85, b + 1.7, 13, 11, "#5a3a30", wall);
    else roofFor(G, S, a + .15, b + .5, a + 1.85, b + 1.7, 13, 7, null, wall);
    return 34;
  }
  if (k === "Bank") {
    const c = f === "hansa" ? "#e2d6bf" : f === "flat" ? "#ecdcc0" : "#e8e2d2";
    wbox(G, a + .2, b + .3, a + 1.8, b + 1.6, 0, 12, c);
    for (let x = .3; x < 1.75; x += .2) q(G, [[a + x, b + 1.72, 0], [a + x + .08, b + 1.72, 0], [a + x + .08, b + 1.72, 11], [a + x, b + 1.72, 11]], "#f6f2e8");
    q(G, [[a + .2, b + 1.6, 0], [a + 1.8, b + 1.6, 0], [a + 1.8, b + 1.6, 1], [a + .2, b + 1.6, 1]], "#6a6254");
    if (f === "med" || f === "hansa" || f === "chalet") {
      q(G, [[a + .2, b + 1.72, 12], [a + 1.8, b + 1.72, 12], [a + 1, b + 1.72, 16]], "#f2ece0");
      hip(G, a + .2, b + .3, a + 1.8, b + 1.6, 12, 3, f === "med" ? S.civRoof : "#6a7278", .1);
    } else roofFor(G, S, a + .2, b + .3, a + 1.8, b + 1.6, 12, 5, null, c);
    return 20;
  }
  if (k === "Mint") {
    wbox(G, a + 1.45, b + .35, a + 1.7, b + .6, 0, 20, "#7a6a5a"); smoke(G, a + 1.57, b + .47, 21);
    wbox(G, a + .3, b + .4, a + 1.7, b + 1.6, 0, 10, wall); winsL(G, a + .4, a + 1.6, b + 1.6, 0, 10, "#f0a040", 5); roofFor(G, S, a + .3, b + .4, a + 1.7, b + 1.6, 10, 6, null, wall);
    return 22;
  }
  if (k === "Granary") {
    if (f === "flat") { for (const [x, y] of [[.55, .55], [1.45, .6], [.6, 1.45], [1.4, 1.4]]) { cyl(G, a + x, b + y, 0, .32, 8, "#d0a066", null); dome(G, a + x, b + y, 8, .32, "#d8aa70"); } return 16; }
    if (f === "yurt") { cyl(G, a + 1, b + 1, 0, .6, 6, "#d8cdb4", null); cone(G, a + 1, b + 1, 6, .64, 6, "#b8a882"); return 12; }
    const tw = "#a8804e"; const X = G.ctx; X.fillStyle = "#8a847a";
    for (const [x, y] of [[.3, 1.6], [1.7, 1.6], [1.7, .4], [1, 1.6]]) { const p = P(G, a + x, b + y, 0); X.fillRect(p[0] - G.zs, p[1] - 3 * G.zs, 2 * G.zs, 3 * G.zs); }
    wbox(G, a + .2, b + .35, a + 1.8, b + 1.65, 3, 9, tw); q(G, [[a + .9, b + 1.65, 5], [a + 1.1, b + 1.65, 5], [a + 1.1, b + 1.65, 10], [a + .9, b + 1.65, 10]], "#3a2a1a");
    if (f === "east") { hip(G, a + .2, b + .35, a + 1.8, b + 1.65, 12, 5, S.civRoof, .14); tips(G, a + .2, b + .35, a + 1.8, b + 1.65, 12, S.civRoof); }
    else if (f === "stilt") hip(G, a + .2, b + .35, a + 1.8, b + 1.65, 12, 9, "#c8a45a", .18);
    else gable(G, a + .2, b + .35, a + 1.8, b + 1.65, 12, 7, "x", f === "turf" || f === "chalet" ? "#eef2f4" : "#8a5a3a", sh(tw, .72), .08);
    return 22;
  }
  if (k === "Warehouse" || k === "Fondaco") {
    if (f === "flat" || f === "yurt") {
      const c = f === "flat" ? "#dcbc86" : "#b09a78";
      wbox(G, a + .1, b + .1, a + 1.9, b + .4, 0, 7, c); wbox(G, a + .1, b + .4, a + .4, b + 1.6, 0, 7, c);
      q(G, [[a + .4, b + .4, 0], [a + 1.6, b + .4, 0], [a + 1.6, b + 1.6, 0], [a + .4, b + 1.6, 0]], sh(c, .9));
      caravan(G, a + .8, b + 1.1, f === "flat" ? "camel" : "horse", 2, 1);
      wbox(G, a + 1.6, b + .4, a + 1.9, b + 1.6, 0, 7, c); wbox(G, a + .1, b + 1.6, a + 1.9, b + 1.9, 0, 7, c); winsL(G, a + .2, a + 1.8, b + 1.9, 0, 7, "#4a3a28", 9, .09);
      for (const [x, y] of [[.1, 1.9], [1.9, 1.9], [1.9, .1]]) wbox(G, a + x - .15, b + y - .15, a + x + .12, b + y + .12, 0, 10, sh(c, 1.04));
      if (k === "Fondaco") flag(G, a + 1.9, b + 1.9, 10, L.color);
      return 12;
    }
    const c = k === "Fondaco" ? (f === "hansa" ? "#b0654a" : "#d8c4a0") : (f === "hansa" ? "#8a3f2b" : f === "east" ? "#cfc4aa" : f === "stilt" ? "#a88a52" : f === "turf" ? "#5e4e3e" : "#c8b898");
    wbox(G, a + .1, b + .3, a + 1.9, b + 1.7, 0, 12, c); winsL(G, a + .2, a + 1.8, b + 1.7, 0, 12, dk, 4, .05); winsR(G, b + .4, b + 1.6, a + 1.9, 0, 12, dk, 4, .05);
    if (k === "Fondaco") for (let x = .3; x < 1.8; x += .3) q(G, [[a + x, b + 1.7, 0], [a + x + .16, b + 1.7, 0], [a + x + .16, b + 1.7, 4], [a + x, b + 1.7, 4]], "#4a3a2c");
    if (f === "hansa") stepGable(G, a + .1, b + .3, a + 1.9, b + 1.7, 12, 12, "#5a3a30", c); else roofFor(G, S, a + .1, b + .3, a + 1.9, b + 1.7, 12, 7, null, c);
    const X = G.ctx; const p = P(G, a + 1.9, b + 1, 16), r2 = P(G, a + 2.3, b + 1, 16), r3 = P(G, a + 2.3, b + 1, 6);
    X.strokeStyle = "#3a2a1c"; X.lineWidth = G.zs; X.beginPath(); X.moveTo(p[0], p[1]); X.lineTo(r2[0], r2[1]); X.lineTo(r3[0], r3[1]); X.stroke();
    return 22;
  }
  if (k === "Workshop") {
    wbox(G, a + 1.4, b + .4, a + 1.62, b + .62, 0, 18, "#6a5a4a"); smoke(G, a + 1.5, b + .5, 19);
    wbox(G, a + .2, b + .45, a + 1.8, b + 1.6, 0, 9, wall); winsL(G, a + .3, a + 1.7, b + 1.6, 0, 9, "#f0a040", 5); roofFor(G, S, a + .2, b + .45, a + 1.8, b + 1.6, 9, 6, null, wall);
    for (let s = 0; s < 3; s++) wbox(G, a + .2 + s * .22, b + 1.7, a + .38 + s * .22, b + 1.88, 0, 2.5, ["#8a6a44", "#a8382e", "#6a7a4a"][s]);
    return 18;
  }
  if (k === "Harbor") {
    const pl = "#8a6a44"; q(G, [[a, b, 1], [a + 2, b, 1], [a + 2, b + 2, 1], [a, b + 2, 1]], sh(pl, 1.05));
    const X = G.ctx; X.strokeStyle = sh(pl, .8); X.lineWidth = .5;
    for (let s = .2; s < 2; s += .2) { const p = P(G, a + s, b, 1), r2 = P(G, a + s, b + 2, 1); X.beginPath(); X.moveTo(p[0], p[1]); X.lineTo(r2[0], r2[1]); X.stroke(); }
    box(G, a + .8, b + 2, a + 1.2, b + 3.4, 0, 1, sh(pl, .9), sh(pl, .7), sh(pl, 1.08));
    for (let s = 0; s < 5; s++) wbox(G, a + .2 + (s % 3) * .28, b + .3 + Math.floor(s / 3) * .3, a + .42 + (s % 3) * .28, b + .52 + Math.floor(s / 3) * .3, 1, 2.6, ["#8a6a44", "#a8382e", "#c8a060", "#6a7a4a", "#5a86a8"][s]);
    const p = P(G, a + 1.6, b + .6, 1), r2 = P(G, a + 1.6, b + .6, 17), r3 = P(G, a + 2.2, b + 1.6, 15);
    X.strokeStyle = "#3a2a1c"; X.lineWidth = 1.1 * G.zs; X.beginPath(); X.moveTo(p[0], p[1]); X.lineTo(r2[0], r2[1]); X.lineTo(r3[0], r3[1]); X.stroke();
    return 18;
  }
  if (k === "Shipyard") {
    const pl = "#8a6a44"; q(G, [[a, b, 1], [a + 2, b, 1], [a + 2, b + 2.6, 0], [a, b + 2.6, 0]], sh(pl, .95));
    const X = G.ctx; X.strokeStyle = "#c8a870"; X.lineWidth = G.zs;
    for (let s = 0; s < 6; s++) { const x = a + .4 + s * .22; const p = P(G, x, b + 1.4, 1), r2 = P(G, x, b + 1.4, 8); X.beginPath(); X.moveTo(p[0] - 2 * G.zs, p[1]); X.quadraticCurveTo(p[0] - 3 * G.zs, r2[1] + 2, p[0], r2[1]); X.stroke(); }
    const p = P(G, a + .4, b + 1.4, 8), r2 = P(G, a + 1.55, b + 1.4, 8); X.beginPath(); X.moveTo(p[0], p[1]); X.lineTo(r2[0], r2[1]); X.stroke();
    wbox(G, a + .2, b + .2, a + 1.8, b + .7, 0, 9, "#8a6a48"); gable(G, a + .2, b + .2, a + 1.8, b + .7, 9, 5, "x", "#5a4030", sh("#8a6a48", .72), .08);
    return 14;
  }
  wbox(G, a + .3, b + .3, a + 1.7, b + 1.7, 0, 10, wall); roofFor(G, S, a + .3, b + .3, a + 1.7, b + 1.7, 10, 6, null, wall);
  return 16;
}

// ── walls ─────────────────────────────────────────────────────────────────────

interface Tile { t: string; v: number; c: string; ward?: number; tree?: TreeKind; gate?: boolean; tower?: boolean }

function wallTile(G: Geo, t: Tile, i: number, j: number, S: CityStyle, c: number, R: number) {
  const k = S.wall, col = S.wallC; const hw = k === "palisade" ? 7 : k === "mud" || k === "rammed" ? 11 : 10; const onJ = Math.abs(j - c) === R;
  const topper = (x0: number, y0: number, x1: number, y1: number, z: number) => {
    if (k === "brick" || S.house === "chalet") cone(G, (x0 + x1) / 2, (y0 + y1) / 2, z, .44, 9, k === "brick" ? "#5a3a30" : "#6a3a30");
    else if (k === "rammed") { hip(G, x0, y0, x1, y1, z, 4, "#3e4852", .14); tips(G, x0, y0, x1, y1, z, "#3e4852"); }
    else if (k === "palisade") hip(G, x0, y0, x1, y1, z, 4, S.house === "turf" ? "#eef2f4" : S.house === "stilt" ? "#c8a45a" : "#6a5238", .1);
    else crenels(G, x0, y0, x1, y1, z, col);
  };
  if (t.tower) {
    const x0 = i + .12, y0 = j + .12, x1 = i + .88, y1 = j + .88;
    if (k === "brick" || (k === "stone" && S.house === "chalet")) { cyl(G, i + .5, j + .5, 0, .4, hw + 8, col, null); topper(x0, y0, x1, y1, hw + 8); }
    else { wbox(G, x0, y0, x1, y1, 0, hw + 7, col); winsL(G, x0 + .1, x1 - .1, y1, hw, 6, "#2a2420", 6, .05); topper(x0, y0, x1, y1, hw + 7); }
    return;
  }
  if (t.gate) {
    const x0 = i + .05, y0 = j + .05, x1 = i + .95, y1 = j + .95; wbox(G, x0, y0, x1, y1, 0, hw + 5, col);
    if (onJ) q(G, [[x1, y0 + .3, 0], [x1, y1 - .3, 0], [x1, y1 - .3, 6], [x1, y0 + .3, 6]], "#2a2420");
    else q(G, [[x0 + .3, y1, 0], [x1 - .3, y1, 0], [x1 - .3, y1, 6], [x0 + .3, y1, 6]], "#2a2420");
    topper(x0, y0, x1, y1, hw + 5);
    return;
  }
  if (k === "palisade") {
    const X = G.ctx; const n = 6; X.lineWidth = Math.max(1, 1.5 * G.zs);
    for (let s = 0; s < n; s++) {
      const u = (s + .5) / n; const [x, y] = onJ ? [i + u, j + .5] : [i + .5, j + u];
      const p = P(G, x, y, 0), r2 = P(G, x, y, hw); X.strokeStyle = s % 2 ? sh(col, .8) : col;
      X.beginPath(); X.moveTo(p[0], p[1]); X.lineTo(r2[0], r2[1]); X.stroke();
      X.fillStyle = sh(col, 1.15); X.fillRect(r2[0] - .5 * G.zs, r2[1] - 1.5 * G.zs, 1 * G.zs, 1.5 * G.zs);
    }
    return;
  }
  if (onJ) { wbox(G, i, j + .32, i + 1, j + .68, 0, hw, col); crenels(G, i, j + .32, i + 1, j + .68, hw, col); }
  else { wbox(G, i + .32, j, i + .68, j + 1, 0, hw, col); crenels(G, i + .32, j, i + .68, j + 1, hw, col); }
  if (k === "rammed") {
    if (onJ) q(G, [[i, j + .68, hw], [i + 1, j + .68, hw], [i + 1, j + .68, hw + 1], [i, j + .68, hw + 1]], "#3e4852");
    else q(G, [[i + .68, j, hw], [i + .68, j + 1, hw], [i + .68, j + 1, hw + 1], [i + .68, j, hw + 1]], "#3e4852");
  }
}

// ── the generator ──────────────────────────────────────────────────────────────

export type WaterKind = "sea" | "river" | "lake" | "oasis" | "none";

/** One landmark the caller wants placed. `ref` is the caller's own index for it
 *  (so a hover can map a drawn landmark back to its data row). */
export interface CityBuilding { kind: LandmarkKind; status: SlotStatus; color: string; ref: number }
export interface CityDistrict { name: string; color: string }

/** Everything the generator needs — deliberately a small, sim-agnostic record. */
export interface CityCfg {
  /** Seed. The same name always yields the same city. */
  name: string;
  style: StyleKey;
  water: WaterKind;
  walled: boolean;
  /** 0..4 population bucket — thins the house fill of a small town. */
  popBucket: number;
  buildings: CityBuilding[];
  /** Up to four quarters, NW/NE/SW/SE; their colours wash the ground. */
  districts: CityDistrict[];
  caravan: CaravanKind | null;
  /** How many hulls to draw on the water (the caller scales this by real vessels). */
  ships: number;
}

export interface GeneratedCity {
  N: number; R: number; c: number;
  T: Tile[][];
  L: PlacedLandmark[];
  S: CityStyle;
  W: WaterKind;
  hasQuay: boolean;
  walled: boolean;
}

const SLOT_ORDER: [number, number][] = [[1, 1], [-3, 1], [1, -3], [-3, -3], [4, 1], [1, 4], [-6, 1], [4, -3], [-3, 4], [-6, -3], [4, 4], [1, -6], [-3, -6], [4, -6], [-6, 4], [-6, -6]];
/** Share of inner-city tiles built up, by population bucket — a hamlet inside a
 *  full wall circuit reads as yards and gardens, not a packed city. */
const BUCKET_DENSITY = [0.45, 0.6, 0.75, 0.9, 1.0];

export function genCity(cfg: CityCfg, N = 36, R = 7): GeneratedCity {
  const S = STYLES[cfg.style]; const r = rng(hstr(cfg.name)); const c = Math.floor(N / 2);
  const T: Tile[][] = [];
  for (let j = 0; j < N; j++) { T[j] = []; for (let i = 0; i < N; i++) T[j][i] = { t: "g", v: r(), c: pick(r, S.ground) }; }
  const at = (i: number, j: number): Tile | null => (i >= 0 && j >= 0 && i < N && j < N) ? T[j][i] : null;
  const ring = (i: number, j: number) => Math.max(Math.abs(i - c), Math.abs(j - c));
  const inX = (i: number) => i >= c - R - 1 && i <= c + R + 1;
  const W = cfg.water; const shoreJ = c + R + 3;
  if (W === "sea" || W === "lake") {
    for (let i = 0; i < N; i++) {
      const e = shoreJ + (inX(i) ? 0 : Math.max(-1, Math.round(Math.sin(i * .45 + 1) * 1.6 + (i > c ? 1 : -.5))));
      for (let j = Math.max(0, e); j < N; j++) { at(i, j)!.t = "w"; }
      const s = at(i, e - 1); if (s && s.t === "g" && !inX(i)) s.t = "s";
    }
  }
  if (W === "river") {
    const w = cfg.style === "east" ? 3 : 2;
    for (let i = 0; i < N; i++) {
      const e = shoreJ + (inX(i) ? 0 : Math.round(Math.sin(i * .32) * 1.5));
      for (let d = 0; d < w; d++) { const t = at(i, e + d); if (t) t.t = "w"; }
      const s = at(i, e + w); if (s && s.t === "g") s.t = "s";
    }
  }
  if (W === "oasis") {
    for (let j = 0; j < N; j++) for (let i = 0; i < N; i++) {
      const dx = (i - (c + R + 4.5)) / 2.6, dy = (j - (c - 1)) / 3.2; const d = dx * dx + dy * dy; const t = at(i, j)!;
      if (d < 1) t.t = "w";
      else if (d < 2.1) { t.t = "f"; t.c = pick(r, S.fields); if (r() < .5) t.tree = "palm"; }
    }
  }
  const hasQuay = W === "sea" || W === "lake" || W === "river";
  for (let i = 0; i < N; i++) for (let j = 0; j < N; j++) {
    const t = at(i, j)!; if (t.t !== "g") continue;
    if (i === c && (j < c - R || j > c + R)) t.t = "r";
    if (j === c && (i < c - R || i > c + R)) t.t = "r";
  }
  if (hasQuay) for (let i = c - R; i <= c + R; i++) for (const j of [c + R + 1, c + R + 2]) { const t = at(i, j); if (t && t.t !== "w") t.t = "q"; }
  for (let j = 0; j < N; j++) for (let i = 0; i < N; i++) {
    const t = at(i, j)!, d = ring(i, j); if (t.t !== "g") continue;
    if (d >= R + 2 && d <= R + 8) {
      const bh = hstr(`${Math.floor(i / 3)},${Math.floor(j / 2)},${cfg.name}`);
      if ((bh % 100) / 100 < S.fieldP) { t.t = "f"; t.c = S.fields[bh % S.fields.length]; continue; }
    }
    if (d > R && r() < S.trees * (d > R + 3 ? 1 : .35)) t.tree = pick(r, S.treeKinds);
    if (S.snow && r() < S.snow) t.c = mix(t.c, "#eef2f4", .7);
  }
  const walled = cfg.walled && S.wall !== "none";
  for (let j = c - R; j <= c + R; j++) for (let i = c - R; i <= c + R; i++) {
    const t = at(i, j)!, d = ring(i, j); t.ward = (i < c ? 0 : 1) + (j < c ? 0 : 2);
    if (d === R && walled) {
      t.t = "W"; const corner = Math.abs(i - c) === R && Math.abs(j - c) === R;
      t.gate = (i === c || j === c) && !corner;
      t.tower = !t.gate && (corner || (Math.abs(j - c) === R && (i - c) % 4 === 0) || (Math.abs(i - c) === R && (j - c) % 4 === 0));
      continue;
    }
    if (d >= R) continue;
    t.t = (i === c || j === c || i === c + 3 || j === c + 3 || i === c - 4 || j === c - 4) ? "st" : "g";
    if ((i === c - 1 || i === c) && (j === c - 1 || j === c)) t.t = "pl";
  }
  // Landmarks — numbered in the caller's order over op/build rows (the slot card
  // shows the same numbers), placed Citadel-first at fixed slot anchors.
  const L: PlacedLandmark[] = []; let si = 0;
  const blds = cfg.buildings.filter((b) => b.status === "op" || b.status === "build")
    .map((b, idx) => ({ ...b, num: idx + 1 }));
  const sorted = [...blds].sort((x, y) => (x.kind === "Citadel" ? -1 : 0) - (y.kind === "Citadel" ? -1 : 0));
  const shoreAnch: [number, number][] = [[c - 3, c + R + 1], [c + 2, c + R + 1], [c - 6, c + R + 1]]; let shi = 0;
  for (const b of sorted) {
    let a: number, bb: number;
    if (b.kind === "Harbor" || b.kind === "Shipyard") {
      if (!hasQuay || shi >= shoreAnch.length) continue;
      [a, bb] = shoreAnch[shi++];
    } else if (b.kind === "Citadel") { a = c - 6; bb = c - 6; }
    else {
      let s: [number, number] | undefined;
      do { s = SLOT_ORDER[si++]; } while (s && s[0] === -6 && s[1] === -6);
      if (!s) continue;
      a = c + s[0]; bb = c + s[1];
    }
    for (let y = bb; y < bb + 2; y++) for (let x = a; x < a + 2; x++) { const t = at(x, y); if (t) t.t = "L"; }
    L.push({ a, b: bb, kind: b.kind, status: b.status, color: toHex(b.color), num: b.num, ref: b.ref });
  }
  const dens = S.density * (BUCKET_DENSITY[Math.max(0, Math.min(4, cfg.popBucket))] ?? 1);
  for (let j = c - R + 1; j < c + R; j++) for (let i = c - R + 1; i < c + R; i++) { const t = at(i, j)!; if (t.t === "g") t.t = r() < dens ? "h" : "y"; }
  const suburb = cfg.popBucket >= 2 ? 1 : 0.4;
  for (let j = 0; j < N; j++) for (let i = 0; i < N; i++) {
    const t = at(i, j)!, d = ring(i, j); if (t.t !== "g" || t.tree) continue;
    if (S.house === "yurt") { if (d > R && d <= R + 5 && r() < .3 * suburb) t.t = "h"; }
    else if (d > R && d <= R + 3 && (Math.abs(i - c) <= 1 || Math.abs(j - c) <= 1) && r() < .55 * suburb) t.t = "h";
  }
  return { N, R, c, T, L, S, W, hasQuay, walled };
}

// ── the iso renderer ──────────────────────────────────────────────────────────

/** A pack sprite provider (`public/city-sprites/<stem>.png`): return a ready
 *  image for a building type, or null to draw the procedural landmark. */
export type SpriteLookup = (kind: string) => HTMLImageElement | null;
/** The pack's uniform sprite cell (shared with scripts/gen_city_sprites.mjs):
 *  128×220, footprint width 100 whose bottom vertex sits at sprite y = 185. */
const SP_VB_W = 128, SP_VB_H = 220, SP_FOOT_W = 100, SP_BASE_Y = 185;
const SPRITE_PEAK: Record<string, number> = {
  Guildhall: 86, Workshop: 102, Granary: 88, Warehouse: 107, Shipyard: 122, Fondaco: 96, Cathedral: 20,
  Temple: 96, Citadel: 42, Palace: 92, "Council Hall": 42, Mint: 104, Bank: 102, Harbor: 74,
};

/** Blit a pack sprite over a 2×2 footprint; returns the roof-top z (for the flag). */
function blitSprite(G: Geo, img: HTMLImageElement, a: number, b: number, kind: string): number {
  const s = (2 * G.TW) / SP_FOOT_W;
  const [bx, by] = P(G, a + 2, b + 2, 0); // bottom vertex of the footprint
  const dx = bx - (SP_VB_W / 2) * s, dy = by - SP_BASE_Y * s;
  G.ctx.drawImage(img, dx, dy, SP_VB_W * s, SP_VB_H * s);
  const peakY = dy + (SPRITE_PEAK[kind] ?? 96) * s;
  const [, cy] = P(G, a + 1, b + 1, 0);
  return Math.max(0, (cy - peakY) / G.zs);
}

export interface IsoGeom { lw: number; lh: number; TW: number; TH: number; OX: number; OY: number }
export interface IsoRender { canvas: HTMLCanvasElement; geom: IsoGeom; city: GeneratedCity }

export interface IsoOpts { W: number; H: number; scale?: number; TW?: number; wash?: number; sprites?: SpriteLookup }

/** Render the iso scene into a fresh LOW-RES offscreen canvas (W/scale × H/scale).
 *  The caller blits it upscaled with smoothing off — the pixel look. */
export function renderIso(city: GeneratedCity, cfg: CityCfg, o: IsoOpts): IsoRender {
  const { W, H, scale = 2, TW = 20 } = o;
  const { T, L, S, c, N } = city; const r = rng(hstr(cfg.name + "iso"));
  const at = (i: number, j: number) => (i >= 0 && j >= 0 && i < N && j < N) ? T[j][i] : null;
  const lw = Math.round(W / scale), lh = Math.round(H / scale);
  const off = document.createElement("canvas"); off.width = lw; off.height = lh;
  const ctx = off.getContext("2d")!;
  const TH = TW / 2; const OX = lw / 2; const OY = Math.round(lh * .54 - (2 * c + 1) * TH / 2); const G = mkG(ctx, TW, OX, OY);
  ctx.fillStyle = S.ground[0]; ctx.fillRect(0, 0, lw, lh);
  const vis = (i: number, j: number) => { const [sx, sy] = P(G, i + .5, j + .5, 0); return sx > -TW && sx < lw + TW && sy > -TW && sy < lh + TW * 2; };
  const wash = o.wash ?? .16; const dcol = cfg.districts.map((d) => toHex(d.color));
  for (let j = 0; j < N; j++) for (let i = 0; i < N; i++) {
    if (!vis(i, j)) continue; const t = T[j][i]; let col: string;
    switch (t.t) {
      case "w": col = mix(S.water, "#0a1a2a", (t.v - .5) * .12 + .02); break;
      case "s": col = S.shore; break;
      case "r": col = S.road; break;
      case "q": col = S.quay; break;
      case "st": col = S.street; break;
      case "pl": col = S.plaza; break;
      case "f": col = t.c; break;
      case "W": col = S.street; break;
      case "h": case "y": case "L": {
        const dc = t.ward !== undefined ? dcol[t.ward] : undefined;
        col = mix(S.street, dc || S.street, wash);
        if (t.t === "y") col = mix(S.ground[1], dc || S.ground[1], wash * .6);
        break;
      }
      default: col = t.c;
    }
    const a = P(G, i, j, 0), b = P(G, i + 1, j, 0), d = P(G, i + 1, j + 1, 0), e = P(G, i, j + 1, 0);
    ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.lineTo(d[0], d[1]); ctx.lineTo(e[0], e[1]); ctx.closePath();
    ctx.fillStyle = col; ctx.fill(); ctx.strokeStyle = col; ctx.lineWidth = .7; ctx.stroke();
    if (t.t === "f") { ctx.strokeStyle = sh(col, .84); ctx.lineWidth = .6; for (let k = 1; k < 4; k++) { const p = P(G, i, j + k / 4, 0), p2 = P(G, i + 1, j + k / 4, 0); ctx.beginPath(); ctx.moveTo(p[0], p[1]); ctx.lineTo(p2[0], p2[1]); ctx.stroke(); } }
    if (t.t === "w" && t.v < .35) { const [sx, sy] = P(G, i + .5, j + .5, 0); const ice = S.ice && t.v < .12; ctx.fillStyle = ice ? "#e8f0f4" : "rgba(230,245,250,.35)"; ctx.fillRect(Math.round(sx - 2 + t.v * 6), Math.round(sy), ice ? 5 : 3, 1); }
    if ((t.t === "st" || t.t === "pl" || t.t === "q") && t.v < .5) { const [sx, sy] = P(G, i + t.v, j + .5, 0); ctx.fillStyle = sh(col, .9); ctx.fillRect(Math.round(sx), Math.round(sy), 2, 1); }
  }
  const objs: [number, () => void][] = []; const add = (d: number, f: () => void) => objs.push([d, f]);
  const shadows: [number, number, number, number, number][] = [];
  for (let j = 0; j < N; j++) for (let i = 0; i < N; i++) {
    if (!vis(i, j)) continue; const t = T[j][i]; const d = i + j + 1;
    if (t.t === "h") { const rr = rng(hstr(cfg.name + i + "," + j)); add(d, () => house(G, i, j, S, rr)); shadows.push([i + .14, j + .14, i + .86, j + .86, 10]); }
    else if (t.t === "W") add(d, () => wallTile(G, t, i, j, S, c, city.R));
    else if (t.tree) { const tk = t.tree; add(d, () => tree(G, i + .5, j + .5, tk)); }
    else if (t.t === "y" && t.v < .4) add(d, () => tree(G, i + .5, j + .5, pick(() => t.v * 2.4, S.treeKinds)));
    else if (t.t === "pl") add(d, () => {
      for (let k = 0; k < 2; k++) {
        const x = i + .25 + k * .45, y = j + .3 + (t.v * .3); const aw = ["#b8423a", "#d8a040", "#3a7a9a", "#6a8a4a"][(i + j + k) % 4];
        wbox(G, x, y, x + .26, y + .26, 0, 2.4, "#8a6a44"); q(G, [[x - .04, y - .04, 4], [x + .3, y - .04, 4], [x + .3, y + .3, 3.2], [x - .04, y + .3, 3.2]], aw);
      }
    });
    if ((t.t === "st" || t.t === "pl" || t.t === "q" || t.t === "r") && t.v > .45) {
      const rr = rng(hstr(cfg.name + "p" + i + j)); const n = t.t === "pl" ? 3 : 1 + Math.floor(rr() * 2);
      add(d + .01, () => { for (let k = 0; k < n; k++) person(G, i + .2 + rr() * .6, j + .2 + rr() * .6, pick(rr, ["#6a4a8a", "#a8382e", "#3a5a8a", "#c8a060", "#4a6a3a", "#8a6a44", "#e8e0d0"]), "#d8a880"); });
    }
  }
  for (const l of L) {
    shadows.push([l.a + .2, l.b + .2, l.a + 1.8, l.b + 1.8, 26]);
    add(l.a + l.b + 2, () => {
      const img = l.status === "op" && o.sprites ? o.sprites(l.kind) : null;
      const top = img ? blitSprite(G, img, l.a, l.b, l.kind) : landmark(G, l, S);
      if (l.status !== "build" && l.kind !== "Harbor" && l.kind !== "Shipyard") flag(G, l.a + 1, l.b + 1, top, l.color);
    });
  }
  const shipCap = city.W === "sea" ? 9 : city.W === "river" ? 6 : city.W === "lake" ? 3 : 0;
  const shipsN = Math.min(shipCap, Math.max(0, Math.round(cfg.ships)));
  let placed = 0;
  for (let k = 0; k < 200 && placed < shipsN; k++) {
    const i = Math.floor(r() * N), j = c + city.R + 3 + Math.floor(r() * (city.W === "river" ? 2 : 7)); const t = at(i, j);
    if (!t || t.t !== "w" || !vis(i, j)) continue; if (Math.abs(i - c) > city.R + 8) continue;
    const ty = pick(r, S.ships); const dir = r() < .5 ? 1 : -1; add(i + j + 1.5, () => ship(G, i + .5, j + .5, ty, dir)); placed++;
  }
  if (cfg.caravan) {
    const kind = cfg.caravan;
    add(c + city.R + 2 + c, () => caravan(G, c + city.R + 2.2, c + .5, kind, 5, 1));
    add(c + c - city.R - 2, () => caravan(G, c - city.R - 4, c + .5, kind, 4, 1));
  }
  for (const [x0, y0, x1, y1, h] of shadows) { const d = h / 16, e = h / 40; q(G, [[x0, y0, 0], [x1 + d, y0 + e, 0], [x1 + d, y1 + e, 0], [x0 + d, y1 + e, 0], [x0, y1, 0]], "rgba(16,22,30,.2)"); }
  objs.sort((A, B) => A[0] - B[0]); for (const [, f] of objs) f();
  const vg = ctx.createRadialGradient(lw / 2, lh / 2, lh * .35, lw / 2, lh / 2, lw * .62);
  vg.addColorStop(0, "rgba(0,0,0,0)"); vg.addColorStop(1, "rgba(8,12,20,.38)"); ctx.fillStyle = vg; ctx.fillRect(0, 0, lw, lh);
  return { canvas: off, geom: { lw, lh, TW, TH, OX, OY }, city };
}

/** Blit a low-res iso render onto a visible canvas at W×H, pixelated, backing at
 *  1× (no DPR multiply — the pixel look needs no retina detail). */
export function presentIso(cv: HTMLCanvasElement, src: HTMLCanvasElement, W: number, H: number) {
  if (cv.width !== W) cv.width = W;
  if (cv.height !== H) cv.height = H;
  cv.style.imageRendering = "pixelated";
  const cx = cv.getContext("2d"); if (!cx) return;
  cx.imageSmoothingEnabled = false; cx.clearRect(0, 0, W, H); cx.drawImage(src, 0, 0, W, H);
}

/** Inverse-iso hit test: which landmark is under a point given in the LOW-RES
 *  canvas's own pixel space. Tries a few heights so a tall roof still hits its
 *  own footprint; the front-most (largest a+b) hit wins, like the draw order. */
export function isoLandmarkAt(r: IsoRender, lx: number, ly: number): PlacedLandmark | null {
  const { TW, TH, OX, OY } = r.geom; const zs = TW / 20;
  let best: PlacedLandmark | null = null;
  for (const z of [0, 6, 12, 18, 26]) {
    const u = (lx - OX) / (TW / 2), v = (ly + z * zs - OY) / (TH / 2);
    const x = (u + v) / 2, y = (v - u) / 2;
    for (const l of r.city.L) {
      if (x >= l.a && x < l.a + 2 && y >= l.b && y < l.b + 2) {
        if (!best || l.a + l.b > best.a + best.b) best = l;
      }
    }
    if (best) return best;
  }
  return null;
}

// ── plan view ─────────────────────────────────────────────────────────────────

export interface PlanGeom { ox: number; oy: number; cs: number }

/** Draw the top-down plan of the same generated city onto `cv` (2× backing,
 *  smooth). Parchment ground, district wash, numbered landmark squares. */
export function renderPlan(cv: HTMLCanvasElement, city: GeneratedCity, cfg: CityCfg, W: number, H: number): PlanGeom {
  const { N, T, L, S, c, R } = city; const k = 2;
  cv.width = W * k; cv.height = H * k; cv.style.imageRendering = "auto";
  const X = cv.getContext("2d")!; X.setTransform(k, 0, 0, k, 0, 0);
  const cs = Math.floor(H / (2 * R + 9)); const ox = W / 2 - (c + .5) * cs, oy = H / 2 - (c + .5) * cs;
  const paper = "#efe4c8"; const ink = "#3a2e22";
  X.fillStyle = paper; X.fillRect(0, 0, W, H); const dcol = cfg.districts.map((d) => toHex(d.color));
  for (let j = 0; j < N; j++) for (let i = 0; i < N; i++) {
    const x = ox + i * cs, y = oy + j * cs; if (x < -cs || y < -cs || x > W || y > H) continue;
    const t = T[j][i]; let col = mix(t.c || S.ground[0], paper, .55);
    if (t.t === "w") col = mix(S.water, "#b8d4dc", .45);
    else if (t.t === "s") col = mix(S.shore, paper, .4);
    else if (t.t === "f") col = mix(t.c, paper, .42);
    else if (t.t === "r" || t.t === "st" || t.t === "pl" || t.t === "q" || t.t === "W") col = "#f6eedb";
    else if (t.t === "h" || t.t === "y" || t.t === "L") col = mix((t.ward !== undefined ? dcol[t.ward] : undefined) || "#d8c8a8", paper, .62);
    X.fillStyle = col; X.fillRect(x, y, cs + .5, cs + .5);
    if (t.t === "f") { X.strokeStyle = mix(t.c, ink, .25); X.globalAlpha = .35; X.lineWidth = .6; for (let s = 3; s < cs; s += 4) { X.beginPath(); X.moveTo(x, y + s); X.lineTo(x + cs, y + s); X.stroke(); } X.globalAlpha = 1; }
    if (t.t === "h") {
      X.fillStyle = mix(S.roofs[Math.floor(t.v * S.roofs.length) % S.roofs.length], ink, .15);
      if (S.house === "yurt") { X.beginPath(); X.arc(x + cs / 2, y + cs / 2, cs * .32, 0, Math.PI * 2); X.fill(); }
      else X.fillRect(x + cs * .16, y + cs * .16, cs * .68, cs * .68);
    }
    if (t.tree || (t.t === "y" && t.v < .4)) { X.fillStyle = "#6a8a4a"; X.beginPath(); X.arc(x + cs / 2, y + cs / 2, cs * .26, 0, Math.PI * 2); X.fill(); }
    if (t.t === "pl" && t.v < .6) { X.fillStyle = "#c8a060"; X.fillRect(x + cs * .3, y + cs * .3, cs * .4, cs * .4); }
  }
  if (city.walled) {
    X.strokeStyle = ink; X.lineWidth = Math.max(3, cs * .3); if (S.wall === "palisade") X.setLineDash([2, 2]);
    X.strokeRect(ox + (c - R + .5) * cs, oy + (c - R + .5) * cs, (2 * R) * cs, (2 * R) * cs); X.setLineDash([]);
    for (let j = c - R; j <= c + R; j++) for (let i = c - R; i <= c + R; i++) {
      const t = T[j][i]; if (t.t !== "W") continue; const x = ox + i * cs, y = oy + j * cs;
      if (t.gate) { X.fillStyle = "#f6eedb"; X.fillRect(x + cs * .2, y + cs * .2, cs * .6, cs * .6); X.strokeStyle = ink; X.lineWidth = 1.2; X.strokeRect(x + cs * .1, y + cs * .1, cs * .8, cs * .8); }
      else if (t.tower) {
        X.fillStyle = ink;
        if (S.wall === "brick") { X.beginPath(); X.arc(x + cs / 2, y + cs / 2, cs * .42, 0, Math.PI * 2); X.fill(); }
        else X.fillRect(x + cs * .08, y + cs * .08, cs * .84, cs * .84);
      }
    }
  }
  X.strokeStyle = mix(ink, paper, .5); X.lineWidth = 1; X.setLineDash([4, 3]); X.beginPath();
  X.moveTo(ox + c * cs, oy + (c - R + 1) * cs); X.lineTo(ox + c * cs, oy + (c + R) * cs);
  X.moveTo(ox + (c - R + 1) * cs, oy + c * cs); X.lineTo(ox + (c + R) * cs, oy + c * cs); X.stroke(); X.setLineDash([]);
  X.textAlign = "center"; X.textBaseline = "middle";
  for (const l of L) {
    const x = ox + l.a * cs, y = oy + l.b * cs;
    X.fillStyle = l.status === "build" ? mix(l.color, paper, .55) : l.color; X.fillRect(x + 2, y + 2, cs * 2 - 4, cs * 2 - 4);
    X.strokeStyle = ink; X.lineWidth = 1.5; if (l.status === "build") X.setLineDash([3, 2]); X.strokeRect(x + 2, y + 2, cs * 2 - 4, cs * 2 - 4); X.setLineDash([]);
    X.fillStyle = "#fff8ea"; X.beginPath(); X.arc(x + cs, y + cs, cs * .46, 0, Math.PI * 2); X.fill();
    X.fillStyle = ink; X.font = `700 ${Math.round(cs * .6)}px ui-monospace,Menlo,monospace`; X.fillText(String(l.num), x + cs, y + cs + .5);
  }
  const bx = W - 64, by = H - 58; X.fillStyle = ink; X.font = "700 11px Georgia,serif"; X.fillText("N", bx, by - 22);
  X.beginPath(); X.moveTo(bx, by - 16); X.lineTo(bx + 5, by); X.lineTo(bx, by - 4); X.lineTo(bx - 5, by); X.closePath(); X.fill();
  X.fillRect(24, H - 24, cs * 5, 3); X.font = "10px Georgia,serif"; X.textAlign = "left"; X.fillText("250 paces", 24, H - 34);
  return { ox, oy, cs };
}

/** Plan-view hit test in the plan's CSS pixel space. */
export function planLandmarkAt(city: GeneratedCity, g: PlanGeom, x: number, y: number): PlacedLandmark | null {
  const i = (x - g.ox) / g.cs, j = (y - g.oy) / g.cs;
  for (const l of city.L) if (i >= l.a && i < l.a + 2 && j >= l.b && j < l.b + 2) return l;
  return null;
}

// ── standalone icons (building slots, fleet registry) — cached ────────────────

const iconCache = new Map<string, HTMLCanvasElement>();
const ICON_CACHE_MAX = 400;
function remember(key: string, c: HTMLCanvasElement) {
  if (iconCache.size >= ICON_CACHE_MAX) { const first = iconCache.keys().next().value; if (first !== undefined) iconCache.delete(first); }
  iconCache.set(key, c);
}

/** One landmark in its family's style on a 3×3 patch of ground, `size`² px. */
export function landmarkIcon(kind: LandmarkKind, style: StyleKey, status: SlotStatus, color: string, size: number): HTMLCanvasElement {
  const key = `lm|${kind}|${style}|${status}|${color}|${size}`;
  const hit = iconCache.get(key); if (hit) return hit;
  const S = STYLES[style]; const off = document.createElement("canvas"); off.width = size; off.height = size;
  const ctx = off.getContext("2d")!; const G = mkG(ctx, Math.round(size * .28), size / 2, 0); G.OY = size * .92 - 3 * G.TH;
  const g = S.ground[0];
  for (let j = 0; j < 3; j++) for (let i = 0; i < 3; i++) {
    const col = (i === 0 || j === 0 || i === 2 || j === 2) ? mix(g, "#000000", .08) : S.street;
    const a = P(G, i, j, 0), b = P(G, i + 1, j, 0), d = P(G, i + 1, j + 1, 0), e = P(G, i, j + 1, 0);
    ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.lineTo(d[0], d[1]); ctx.lineTo(e[0], e[1]); ctx.closePath();
    ctx.fillStyle = col; ctx.fill(); ctx.strokeStyle = col; ctx.lineWidth = .6; ctx.stroke();
  }
  const col = toHex(color);
  if (status === "lock") ctx.globalAlpha = .35;
  const top = landmark(G, { a: .5, b: .5, kind, status: status === "build" ? "build" : "op", color: col }, S);
  if (status !== "lock" && status !== "build" && kind !== "Harbor" && kind !== "Shipyard") flag(G, 1.5, 1.5, top, col);
  ctx.globalAlpha = 1;
  remember(key, off);
  return off;
}

/** A hull or a caravan string on its own patch of water/road, `size`² px. */
export function vesselIcon(type: ShipKind | CaravanKind, style: StyleKey, size: number): HTMLCanvasElement {
  const key = `vs|${type}|${style}|${size}`;
  const hit = iconCache.get(key); if (hit) return hit;
  const S = STYLES[style]; const off = document.createElement("canvas"); off.width = size; off.height = size;
  const ctx = off.getContext("2d")!; const G = mkG(ctx, Math.round(size * .5), size / 2, 0); G.OY = size * .72 - G.TH;
  if (type === "camel" || type === "horse" || type === "wagon") { ctx.fillStyle = S.road; ctx.fillRect(0, size * .66, size, size * .2); caravan(G, .3 - .34, .5, type, 2, 1); }
  else { ctx.fillStyle = S.water; ctx.fillRect(0, size * .62, size, size * .38); ship(G, .5, .5, type, 1); }
  remember(key, off);
  return off;
}
