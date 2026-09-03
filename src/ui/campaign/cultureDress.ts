// Cultural DRESS PLATES — a portrait bust and a small full figure per people,
// drawn in the same pixel treatment as the goods set (coarse author grid, hard
// dark edge, one-pixel bevel, nearest-neighbour upscale).
//
// `DRESS_KITS` is index-aligned to `cultureFigure.ts`'s KITS and therefore to
// `sim/cultures.rs` — kit 7 is Indic in all three. It is a FLATTENED kit: the
// SVG figure system picks one option per individual out of an array of hairs /
// robes / trims, while a dress plate states the people's dress once, so each
// entry carries a single colour per axis.
//
// This is deliberately NOT a replacement for `cultureFigure.ts`. That system
// draws an INDIVIDUAL — a named house head, with a sex the sim actually models
// (matrilineal succession, widow regents) and a per-person seed. These plates
// draw a PEOPLE. Two different questions; the sex axis does not exist here and
// inventing one would be a redesign, not a port.

import { pixelize, shade } from "@canvas/goodArt";

export type Occasion = "everyday" | "national" | "ceremonial";
/** The three registers every people has, whatever its kit's provenance. */
export const REGISTERS: Occasion[] = ["everyday", "national", "ceremonial"];

export interface DressKit {
  id: number;
  name: string;
  region: string;
  skin: string; hair: string;
  robe: string; trim: string; cloth2: string;
  beard: number;
  garment: string;
  /** Headwear design index (defaults to `id`). */
  hat?: number;
  /** Neckline design index (defaults to `id`). */
  neck?: number;
  veil?: boolean;
  derived?: boolean;
  creole?: [string, string];
}

export const DRESS_KITS: DressKit[] = [
  { id: 0, name: "Roman", region: "Latin littoral", skin: "#e8bd90", hair: "#33241a", robe: "#ece6d6", trim: "#9c2b2b", cloth2: "#b8863b", beard: 0, garment: "toga" },
  { id: 1, name: "Hellene", region: "Aegean poleis", skin: "#e0b488", hair: "#2c1e14", robe: "#eef1f4", trim: "#2f6fb0", cloth2: "#c8a33a", beard: 1, garment: "himation" },
  { id: 2, name: "Punic", region: "Tyrian colonies", skin: "#d4a576", hair: "#241a12", robe: "#6a2f7a", trim: "#d8b23a", cloth2: "#b8623a", beard: 1, garment: "robe" },
  { id: 3, name: "Persian", region: "Iranian plateau", skin: "#d8ac78", hair: "#201712", robe: "#2f5aa0", trim: "#d8b23a", cloth2: "#8a3b3b", beard: 1, garment: "kaftan" },
  { id: 4, name: "Norse", region: "Northern seas", skin: "#f0cba3", hair: "#c08a3a", robe: "#6b4a2e", trim: "#9aa4ad", cloth2: "#4a5a3a", beard: 1, garment: "tunic" },
  { id: 5, name: "Celtic", region: "Western isles", skin: "#f4d6b8", hair: "#a8431e", robe: "#4a6a3a", trim: "#b8863b", cloth2: "#8a3b2a", beard: 1, garment: "tunic" },
  { id: 6, name: "Arab", region: "Desert emirates", skin: "#cf9e6a", hair: "#1c140e", robe: "#eae4d6", trim: "#7a5a2a", cloth2: "#b83a2a", beard: 1, garment: "thobe" },
  { id: 7, name: "Indic", region: "Monsoon coast", skin: "#9c6a44", hair: "#14100c", robe: "#c0392b", trim: "#e0b13a", cloth2: "#d8763a", beard: 0, garment: "sari" },
  { id: 8, name: "Sinitic", region: "River provinces", skin: "#e8bd90", hair: "#14100c", robe: "#b83a3a", trim: "#d8b23a", cloth2: "#2e6a5a", beard: 0, garment: "crossrobe" },
  { id: 9, name: "Slavic", region: "Forest rivers", skin: "#f4d6b8", hair: "#7a5a2e", robe: "#ece6d6", trim: "#b83a2a", cloth2: "#2f6fb0", beard: 1, garment: "tunic" },
  { id: 10, name: "Nahua", region: "Highland basin", skin: "#b07f52", hair: "#14100c", robe: "#d8763a", trim: "#2ea0a0", cloth2: "#b83a2a", beard: 0, garment: "wrap" },
  { id: 11, name: "Turkic", region: "Steppe khanates", skin: "#cf9e6a", hair: "#1c140e", robe: "#3a6a8a", trim: "#d8b23a", cloth2: "#8a3b3a", beard: 1, garment: "kaftan" },
  { id: 12, name: "Nilotic", region: "Upper river", skin: "#6f472c", hair: "#14100c", robe: "#eae4d6", trim: "#c9a227", cloth2: "#2a5fa0", beard: 0, garment: "wrap" },
  { id: 13, name: "Amazigh", region: "Atlas & sands", skin: "#c28f5c", hair: "#1c140e", robe: "#2a5fa0", trim: "#c8ced6", cloth2: "#8a3b3a", beard: 0, garment: "robe" },
  { id: 14, name: "Yamato", region: "Eastern isles", skin: "#e0b488", hair: "#14100c", robe: "#31405a", trim: "#d8b23a", cloth2: "#b83a3a", beard: 0, garment: "kimono" },
  { id: 15, name: "Mongol", region: "Cold steppe", skin: "#d8ac78", hair: "#1c140e", robe: "#8a5a2e", trim: "#c9a227", cloth2: "#3a6a8a", beard: 1, garment: "deel" },
  { id: 16, name: "Quechua", region: "Cordillera", skin: "#9c6a44", hair: "#14100c", robe: "#b83a2a", trim: "#e0b13a", cloth2: "#2ea0a0", beard: 0, garment: "poncho" },
  { id: 17, name: "Mande", region: "Sahel cities", skin: "#6f472c", hair: "#14100c", robe: "#d8b23a", trim: "#b83a2a", cloth2: "#2e7d5a", beard: 0, garment: "boubou" },
];

// ── procedural and creole peoples ──────────────────────────────────────────
// The eighteen above are PRESETS, not the whole world. A kit is just a record of
// choices — a headwear design, a neckline, a garment cut, three dye colours, a
// skin and hair ramp — so any culture the sim invents can have one.

const GARMENTS = ["robe", "kaftan", "tunic", "thobe", "kimono", "deel", "boubou", "wrap", "poncho", "crossrobe", "toga", "himation", "sari"];
// Dyes a pre-industrial city can actually strike, by rough cost of the dyestuff.
const DYES: Record<string, string[]> = {
  common: ["#8a6a3a", "#6b7a4a", "#9c6a44", "#7a6a5a", "#a8813a", "#5a6a72", "#8a4a3a", "#6a5a7a"],
  fine: ["#2f5aa0", "#b83a3a", "#2e7d5a", "#d8763a", "#3a6a8a", "#8a3b6a", "#4a6a3a", "#b8623a"],
  costly: ["#6a2f7a", "#c0392b", "#d8b23a", "#2ea0a0", "#e0b13a", "#c9a227"],
};
// Skin follows where a people LIVES, not which culture it is — a latitude ramp.
const SKINS = ["#f4d6b8", "#e8bd90", "#e0b488", "#d8ac78", "#cf9e6a", "#c28f5c", "#b07f52", "#9c6a44", "#6f472c"];
const HAIRS = ["#c08a3a", "#a8431e", "#7a5a2e", "#33241a", "#241a12", "#1c140e", "#14100c"];

/** The sim's hash, so a derived kit is stable for a given seed and culture. */
function hash01(seed: number, a: number, b: number): number {
  let h = (seed ^ 0x9e3779b9) >>> 0;
  for (const v of [a >>> 0, b >>> 0]) {
    h = (h ^ v) >>> 0; h = Math.imul(h, 0x85ebca6b) >>> 0;
    h = ((h >>> 13) ^ h) >>> 0; h = Math.imul(h, 0xc2b2ae35) >>> 0;
  }
  return (((h >>> 16) ^ h) >>> 0) / 4294967296;
}
const strSeed = (s: string | number): number => {
  let h = 2166136261;
  const t = String(s);
  for (let i = 0; i < t.length; i++) { h ^= t.charCodeAt(i); h = Math.imul(h, 16777619); }
  return h >>> 0;
};

export interface DeriveOpts { seed?: number; climate?: number; wealth?: number; region?: string }

/**
 * A kit for a culture that has no preset — a worldgen hearth past the list, or
 * a people whose kit index came back -1. Deterministic in (name, seed).
 * `climate` 0..1 runs cold→hot and drives the skin ramp and how much cloth the
 * garment uses; `wealth` 0..1 opens the costlier dyes.
 */
export function deriveKit(name: string, opts: DeriveOpts = {}): DressKit {
  const seed = opts.seed ?? 0, s = strSeed(name);
  const r = (n: number) => hash01(seed, s, n);
  const climate = opts.climate ?? r(1);
  const wealth = opts.wealth ?? r(2);
  // hot places wrap and drape; cold places tailor and layer
  const pool = climate > 0.66 ? ["robe", "thobe", "wrap", "boubou", "sari", "himation"]
    : climate < 0.33 ? ["deel", "kaftan", "tunic", "crossrobe", "poncho"]
    : GARMENTS;
  const tier = wealth > 0.72 ? DYES.costly : wealth > 0.38 ? DYES.fine : DYES.common;
  const pick = (arr: string[], n: number) => arr[Math.floor(r(n) * arr.length) % arr.length];
  const skinIdx = Math.min(SKINS.length - 1, Math.floor(climate * SKINS.length));
  const robe = pick(tier, 3);
  let trim = pick(DYES.costly, 4);
  if (trim === robe) trim = DYES.costly[(DYES.costly.indexOf(trim) + 2) % DYES.costly.length];
  return {
    id: -1, derived: true, name, region: opts.region || "",
    skin: SKINS[skinIdx], hair: pick(HAIRS, 5),
    robe, trim, cloth2: pick(DYES.fine, 6),
    beard: r(7) < 0.55 ? 1 : 0,
    garment: pick(pool, 8),
    hat: Math.floor(r(9) * 18),
    neck: Math.floor(r(10) * 18),
    veil: climate > 0.7 && r(11) < 0.34,
  };
}

const mix = (a: string, b: string, t: number): string => {
  const p = (h: string) => {
    let s = h.replace("#", "");
    if (s.length === 3) s = s.split("").map((c) => c + c).join("");
    return [0, 2, 4].map((i) => parseInt(s.slice(i, i + 2), 16));
  };
  const A = p(a), B = p(b);
  return "#" + A.map((v, i) => Math.round(v + (B[i] - v) * t).toString(16).padStart(2, "0")).join("");
};

/**
 * A creole people's kit: not an average of its parents but a COMPOSITE — it
 * keeps the majority parent's garment cut and neckline, takes the minority
 * parent's headwear, and strikes its cloth in a dye between the two. That is
 * why a creole reads as recognisably descended from both without looking like
 * either, and why its plate needs no new artwork.
 */
export function creoleKit(
  name: string, parentA: KitSpec, parentB: KitSpec, opts: { lean?: number; region?: string } = {},
): DressKit {
  const A = resolveKit(parentA), B = resolveKit(parentB);
  const t = opts.lean ?? 0.5;                 // 0 = all majority, 1 = all minority
  return {
    id: -1, derived: true, creole: [A.name, B.name], name,
    region: opts.region || "Creole · " + A.name + " × " + B.name,
    skin: mix(A.skin, B.skin, t), hair: t > 0.5 ? B.hair : A.hair,
    robe: mix(A.robe, B.robe, t), trim: B.trim, cloth2: mix(A.cloth2, B.cloth2, 1 - t),
    beard: t > 0.5 ? B.beard : A.beard,
    garment: A.garment,                        // the cut descends from the majority
    hat: B.hat ?? B.id,                        // the hat from the minority
    neck: A.neck ?? A.id,
    veil: A.veil || B.veil,
  };
}

export type KitSpec = number | DressKit | string | null | undefined;

/** Accept a preset index, a kit object, or a name to derive from. */
export function resolveKit(spec: KitSpec, opts?: DeriveOpts): DressKit {
  if (spec && typeof spec === "object") return spec;
  if (typeof spec === "number") {
    if (spec >= 0 && spec < DRESS_KITS.length) return DRESS_KITS[spec];
    return deriveKit("kit" + spec, { seed: spec, ...opts });
  }
  return deriveKit(String(spec ?? "unknown"), opts);
}

// ── primitives, all in author space ────────────────────────────────────────
type Ctx = CanvasRenderingContext2D;
const T2 = Math.PI * 2;
const E = (c: Ctx, x: number, y: number, rx: number, ry: number, f: string) => {
  c.beginPath(); c.ellipse(x, y, rx, ry, 0, 0, T2); c.fillStyle = f; c.fill();
};
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

/** The canonical head frame every plate is drawn against. */
const HX = 50, HY = 44, HRX = 19, HRY = 22;

interface Pal {
  skin: string; skinD: string; skinL: string; hair: string; hairL: string;
  robe: string; robeL: string; robeD: string; trim: string; trimL: string;
  cloth2: string; cloth2D: string; rich: boolean;
}

function pal(K: DressKit, occ: Occasion): Pal {
  const dull = occ === "everyday" ? 0.86 : 1;
  const robe = shade(K.robe, dull);
  return {
    skin: K.skin, skinD: shade(K.skin, 0.8), skinL: shade(K.skin, 1.12),
    hair: K.hair, hairL: shade(K.hair, 1.35),
    robe, robeL: shade(K.robe, dull * 1.18), robeD: shade(K.robe, dull * 0.72),
    trim: occ === "everyday" ? shade(K.trim, 0.8) : K.trim,
    trimL: shade(K.trim, 1.3),
    cloth2: shade(K.cloth2, dull), cloth2D: shade(K.cloth2, dull * 0.72),
    rich: occ === "ceremonial",
  };
}

// ── headwear: one distinct silhouette per people ───────────────────────────
function headwear(c: Ctx, id: number, p: Pal, occ: Occasion) {
  const top = HY - HRY, cx = HX;
  switch (id) {
    case 0: // Roman — laurel wreath over a short cap of hair
      for (let i = 0; i < 5; i++) {
        const t = i / 4, a = -Math.PI * (0.14 + t * 0.34);
        for (const s of [-1, 1]) {
          const x = cx + s * Math.cos(a) * (HRX + 2), y = HY + Math.sin(a) * (HRY + 1);
          P(c, [[x, y - 3], [x + s * 5, y - 6], [x + s * 2, y + 1]], i % 2 ? p.trim : shade(p.trim, 1.25));
        }
      }
      L(c, [[cx - HRX - 1, top + 9], [cx, top + 3], [cx + HRX + 1, top + 9]], p.cloth2, 3);
      break;
    case 1: // Hellene — fillet band with trailing ends
      L(c, [[cx - HRX - 1, top + 8], [cx, top + 2], [cx + HRX + 1, top + 8]], p.trim, 3.4);
      L(c, [[cx + HRX - 1, top + 8], [cx + HRX + 4, top + 20]], p.trim, 2.2);
      for (const s of [-1, 1]) E(c, cx + s * 13, top + 4, 5, 4, p.hairL);
      break;
    case 2: // Punic — tall conical cap with brim and tassel
      P(c, [[cx - 13, top + 4], [cx + 13, top + 4], [cx + 4, top - 22], [cx - 4, top - 22]], p.cloth2);
      P(c, [[cx - 4, top - 22], [cx + 4, top - 22], [cx + 3, top - 14], [cx - 3, top - 14]], p.trim);
      R(c, cx - 16, top + 2, 32, 5, p.trim);
      L(c, [[cx + 4, top - 20], [cx + 12, top - 12]], p.trim, 2);
      break;
    case 3: // Persian — fluted kyrbasia with a band and side flaps
      P(c, [[cx - 14, top + 6], [cx + 14, top + 6], [cx + 12, top - 16], [cx - 12, top - 16]], p.robe);
      for (let i = -2; i <= 2; i++) L(c, [[cx + i * 5, top - 14], [cx + i * 5.4, top + 4]], p.robeD, 1.4);
      R(c, cx - 15, top + 3, 30, 5, p.trim);
      for (const s of [-1, 1]) P(c, [[cx + s * 14, top + 6], [cx + s * 18, top + 10], [cx + s * 16, top + 24], [cx + s * 12, top + 20]], p.robeD);
      break;
    case 4: // Norse — fur-brimmed cap, braids
      P(c, [[cx - 16, top + 8], [cx + 16, top + 8], [cx + 12, top - 10], [cx - 12, top - 10]], p.robeD);
      R(c, cx - 18, top + 5, 36, 7, shade(p.trim, 1.1));
      for (let i = -3; i <= 3; i++) E(c, cx + i * 5.2, top + 8.5, 3.2, 3.6, i % 2 ? p.trim : shade(p.trim, 1.22));
      for (const s of [-1, 1]) {
        L(c, [[cx + s * (HRX - 1), HY + 6], [cx + s * (HRX + 3), HY + 26]], p.hair, 5);
        for (let i = 0; i < 3; i++) E(c, cx + s * (HRX + 1 + i * 0.6), HY + 12 + i * 6, 2.6, 2.2, p.hairL);
      }
      break;
    case 5: // Celtic — lime-washed hair swept back in spikes
      for (let i = -3; i <= 3; i++) P(c, [[cx + i * 5 - 3, top + 8], [cx + i * 5 + 3, top + 8], [cx + i * 5 + 1 + i * 1.6, top - 16]], i % 2 ? p.hairL : shade(p.hair, 1.6));
      E(c, cx, top + 10, HRX, 7, shade(p.hair, 1.45));
      break;
    case 6: // Arab — keffiyeh falling to the shoulders under a double cord
      P(c, [[cx - HRX - 5, HY + 4], [cx - HRX - 4, top - 2], [cx, top - 7], [cx + HRX + 4, top - 2], [cx + HRX + 5, HY + 4],
        [cx + HRX + 6, HY + 32], [cx + HRX - 1, HY + 32], [cx + HRX, HY + 2], [cx, top + 2], [cx - HRX, HY + 2], [cx - HRX + 1, HY + 32], [cx - HRX - 6, HY + 32]], p.robeL);
      L(c, [[cx - HRX - 3, top + 5], [cx, top - 2], [cx + HRX + 3, top + 5]], p.cloth2, 2.6);
      L(c, [[cx - HRX - 3, top + 10], [cx, top + 3], [cx + HRX + 3, top + 10]], p.cloth2, 2.6);
      break;
    case 7: // Indic — wound turban, jewel and plume
      P(c, [[cx - 17, top + 8], [cx + 17, top + 8], [cx + 14, top - 14], [cx - 14, top - 14]], p.trim);
      for (let i = 0; i < 4; i++) L(c, [[cx - 16 + i, top + 5 - i * 5], [cx + 16 - i, top + 2 - i * 5]], i % 2 ? shade(p.trim, 0.82) : shade(p.trim, 1.16), 4.6);
      E(c, cx - 1, top - 6, 3.4, 3.4, p.cloth2);
      L(c, [[cx + 8, top - 10], [cx + 15, top - 26]], p.robe, 2.6);
      break;
    case 8: // Sinitic — futou, the winged official's cap
      P(c, [[cx - 13, top + 6], [cx + 13, top + 6], [cx + 11, top - 12], [cx - 11, top - 12]], "#20242c");
      E(c, cx, top - 12, 11, 5, "#2b3038");
      for (const s of [-1, 1]) P(c, [[cx + s * 12, top - 4], [cx + s * 26, top - 8], [cx + s * 26, top - 1], [cx + s * 12, top + 2]], "#20242c");
      R(c, cx - 13, top + 3, 26, 4, p.trim);
      break;
    case 9: // Slavic — tall hat over a fur brim
      P(c, [[cx - 14, top + 4], [cx + 14, top + 4], [cx + 12, top - 20], [cx - 12, top - 20]], p.cloth2);
      E(c, cx, top - 20, 12, 4, shade(p.cloth2, 1.2));
      R(c, cx - 17, top + 1, 34, 8, shade(p.trim, 1.05));
      for (let i = -3; i <= 3; i++) E(c, cx + i * 5, top + 5, 3, 3.4, i % 2 ? p.trim : shade(p.trim, 1.25));
      break;
    case 10: // Nahua — feather fan, headband, jade earplug
      for (let i = -4; i <= 4; i++) {
        const a = i * 0.17;
        L(c, [[cx + i * 2.4, top + 4], [cx + Math.sin(a) * 30, top - 24 + Math.abs(i) * 2.4]], i % 2 ? p.trim : p.cloth2, 3.4);
      }
      R(c, cx - HRX, top + 6, HRX * 2, 6, p.robe);
      for (let i = -2; i <= 2; i++) E(c, cx + i * 6, top + 9, 2.2, 2.2, p.trim);
      E(c, cx + HRX - 1, HY + 6, 3.6, 3.6, p.trim);
      break;
    case 11: // Turkic — turban wound over a pointed kalpak
      P(c, [[cx - 11, top - 2], [cx + 11, top - 2], [cx, top - 22]], p.cloth2);
      for (let i = 0; i < 3; i++) { const y = top + 6 - i * 5; E(c, cx, y, 17 - i * 1.5, 4.4, i % 2 ? shade(p.robeL, 1.05) : p.robeL); }
      E(c, cx + 12, top - 4, 3, 3, p.trim);
      break;
    case 12: // Nilotic — shaved crown, beaded band, long ear ring
      R(c, cx - HRX, top + 7, HRX * 2, 5, p.cloth2);
      for (let i = -3; i <= 3; i++) E(c, cx + i * 5.2, top + 9.5, 2.4, 2.4, i % 2 ? p.trim : shade(p.trim, 1.3));
      L(c, [[cx - 3, top + 4], [cx, top - 1], [cx + 3, top + 4]], p.trim, 2.2);
      for (const s of [-1, 1]) { c.beginPath(); c.arc(cx + s * (HRX - 1), HY + 12, 5, 0, T2); c.strokeStyle = p.trim; c.lineWidth = 1.8; c.stroke(); }
      break;
    case 13: // Amazigh — indigo tagelmust, wrapped over the face
      P(c, [[cx - HRX - 4, HY + 2], [cx - HRX - 3, top - 2], [cx, top - 8], [cx + HRX + 3, top - 2], [cx + HRX + 4, HY + 2],
        [cx + HRX + 5, HY + 30], [cx - HRX - 5, HY + 30]], p.robe);
      for (let i = 0; i < 3; i++) L(c, [[cx - HRX - 2, top + 6 + i * 6], [cx + HRX + 2, top + 2 + i * 6]], p.robeD, 1.6);
      L(c, [[cx - HRX - 2, HY + 3], [cx + HRX + 2, HY + 3]], p.robeD, 1.6);
      E(c, cx, HY + 12, HRX - 1, 12, p.robe);        // face veil to the bridge
      L(c, [[cx - 12, HY + 6], [cx + 12, HY + 6]], p.trim, 1.4);
      break;
    case 14: // Yamato — small lacquered eboshi, tipped back
      c.save(); c.translate(cx, top + 4); c.rotate(-0.16);
      P(c, [[-10, 2], [10, 2], [7, -17], [-7, -17]], "#1b1f26");
      E(c, 0, -17, 7, 3, "#262b33");
      c.restore();
      L(c, [[cx - 12, top + 6], [cx + 12, top + 6]], p.trim, 2);
      L(c, [[cx + 2, top - 12], [cx + 12, top - 18]], p.hair, 4);
      break;
    case 15: // Mongol — fur-brimmed conical hat with earflaps
      P(c, [[cx - 13, top - 2], [cx + 13, top - 2], [cx, top - 24]], p.robe);
      L(c, [[cx, top - 24], [cx + 6, top - 30]], p.trim, 2.2);
      R(c, cx - 17, top - 4, 34, 8, shade(p.cloth2, 1.05));
      for (let i = -3; i <= 3; i++) E(c, cx + i * 5, top, 3, 3.4, i % 2 ? p.cloth2 : shade(p.cloth2, 1.25));
      for (const s of [-1, 1]) P(c, [[cx + s * 15, top + 2], [cx + s * 19, top + 6], [cx + s * 17, HY + 12], [cx + s * 12, HY + 8]], shade(p.cloth2, 0.86));
      break;
    case 16: // Quechua — knit chullo, patterned bands, earflaps
      P(c, [[cx - 15, top + 8], [cx + 15, top + 8], [cx + 11, top - 14], [cx - 11, top - 14]], p.robe);
      for (let i = 0; i < 4; i++) { const y = top - 12 + i * 5.4; R(c, cx - 15 + i * 0.7, y, 30 - i * 1.4, 2.6, i % 2 ? p.trim : p.cloth2); }
      E(c, cx, top - 15, 3.4, 3.4, p.trim);
      for (const s of [-1, 1]) {
        P(c, [[cx + s * 14, top + 8], [cx + s * 18, top + 12], [cx + s * 16, HY + 16], [cx + s * 11, HY + 12]], p.robe);
        L(c, [[cx + s * 15, HY + 14], [cx + s * 15, HY + 22]], p.cloth2, 1.8);
      }
      break;
    default: // 17 Mande — embroidered kufi
      P(c, [[cx - 15, top + 8], [cx + 15, top + 8], [cx + 13, top - 6], [cx - 13, top - 6]], p.cloth2);
      E(c, cx, top - 6, 13, 4, shade(p.cloth2, 1.18));
      for (let i = -2; i <= 2; i++) P(c, [[cx + i * 6 - 3, top + 4], [cx + i * 6, top - 2], [cx + i * 6 + 3, top + 4]], p.trim);
      break;
  }
  if (occ === "ceremonial") for (const s of [-1, 1]) E(c, HX + s * (HRX + 2), HY + 16, 2.6, 2.6, p.trim);
}

/** Whether the headwear design hides the hairline (skip the fringe if so). */
const COVERED = new Set([2, 3, 4, 6, 7, 8, 9, 11, 13, 15, 16, 17]);

/** Head, hair, face and headwear in the canonical frame. */
function headBlock(c: Ctx, K: DressKit, p: Pal, occ: Occasion) {
  const hat = K.hat ?? K.id, veiled = K.veil ?? (hat === 13);
  for (const s of [-1, 1]) E(c, HX + s * (HRX - 1), HY + 3, 3.4, 4.2, p.skinD);
  if (!COVERED.has(hat)) {
    P(c, [[HX - HRX, HY - 6], [HX - HRX - 3, HY + 30], [HX + HRX + 3, HY + 30], [HX + HRX, HY - 6]], p.hair);
  }
  E(c, HX, HY, HRX, HRY, p.skin);
  P(c, [[HX + HRX - 7, HY - HRY + 4], [HX + HRX, HY - 2], [HX + HRX - 2, HY + HRY - 6], [HX + 6, HY + HRY - 1]], p.skinD);
  if (!COVERED.has(hat)) {
    P(c, [[HX - HRX - 1, HY - 6], [HX - HRX + 2, HY - HRY - 3], [HX, HY - HRY - 4], [HX + HRX - 2, HY - HRY - 3], [HX + HRX + 1, HY - 6],
      [HX + 7, HY - HRY + 7], [HX, HY - HRY + 8], [HX - 7, HY - HRY + 7]], p.hair);
  } else {
    P(c, [[HX - HRX + 1, HY - 8], [HX + HRX - 1, HY - 8], [HX + 8, HY - HRY + 8], [HX - 8, HY - HRY + 8]], p.hair);
  }
  if (!veiled) {
    for (const s of [-1, 1]) L(c, [[HX + s * 3, HY - 5], [HX + s * 10, HY - 4]], shade(p.hair, 1.1), 1.8);
    for (const s of [-1, 1]) E(c, HX + s * 6.5, HY + 1, 2, 2.4, "#2a211c");
    for (const s of [-1, 1]) E(c, HX + s * 5.6, HY + 0.2, 0.8, 0.9, "#f2ece4");
    L(c, [[HX, HY + 3], [HX, HY + 8]], p.skinD, 1.6);
    L(c, [[HX - 4, HY + 13], [HX, HY + 14.6], [HX + 4, HY + 13]], shade(p.skin, 0.66), 1.8);
    if (K.beard && occ !== "everyday") {
      P(c, [[HX - HRX + 2, HY + 4], [HX - HRX + 3, HY + 20], [HX, HY + 27], [HX + HRX - 3, HY + 20], [HX + HRX - 2, HY + 4],
        [HX + 8, HY + 15], [HX - 8, HY + 15]], p.hair);
      L(c, [[HX - 5, HY + 9], [HX + 5, HY + 9]], shade(p.hair, 1.2), 1.4);
    }
  }
  headwear(c, hat, p, occ);
}

// ── the neckline: one per people, drawn over the shoulders ─────────────────
function collar(c: Ctx, id: number, p: Pal, occ: Occasion, sy: number, halfTop: number, halfBot: number, by: number) {
  const cx = HX;
  const band = (y: number, h: number, col: string) => P(c, [
    [cx - halfTop - (y - sy) * 0.34, y], [cx + halfTop + (y - sy) * 0.34, y],
    [cx + halfTop + (y + h - sy) * 0.34, y + h], [cx - halfTop - (y + h - sy) * 0.34, y + h],
  ], col);
  switch (id) {
    case 0: P(c, [[cx - halfTop + 1, sy], [cx + 8, by], [cx + 20, by], [cx - halfTop + 12, sy - 1]], p.robeL);
      L(c, [[cx - halfTop + 3, sy + 1], [cx + 14, by]], p.trim, 2.6); break;
    case 1: P(c, [[cx + halfTop - 1, sy], [cx - 10, by], [cx - 22, by], [cx + halfTop - 13, sy - 1]], p.robeL);
      E(c, cx + halfTop - 5, sy + 3, 3.4, 3.4, p.trim); break;
    case 2: P(c, [[cx - 12, sy], [cx, sy + 16], [cx + 12, sy], [cx + 9, sy - 3], [cx - 9, sy - 3]], p.robeD);
      L(c, [[cx - 12, sy], [cx, sy + 16], [cx + 12, sy]], p.trim, 2.4); break;
    case 3: R(c, cx - 3.5, sy - 2, 7, by - sy + 2, p.cloth2);
      L(c, [[cx, sy], [cx, by]], p.trim, 1.6);
      for (let i = 0; i < 4; i++) L(c, [[cx - 9, sy + 5 + i * 7], [cx + 9, sy + 5 + i * 7]], p.trim, 1.8); break;
    case 4: P(c, [[cx - halfTop, sy - 1], [cx - halfTop - 5, by], [cx - halfTop + 12, by], [cx - halfTop + 8, sy - 2]], p.cloth2);
      E(c, cx - halfTop + 4, sy + 4, 4.4, 4.4, p.trim); break;
    case 5: c.beginPath(); c.arc(cx, sy + 1, 10, Math.PI * 0.12, Math.PI * 0.88); c.strokeStyle = p.trim; c.lineWidth = 3.4; c.stroke();
      for (const s of [-1, 1]) E(c, cx + s * 9.9, sy + 2.2, 2.4, 2.4, shade(p.trim, 1.3)); break;
    case 6: c.beginPath(); c.arc(cx, sy - 2, 10, 0.1, Math.PI - 0.1); c.strokeStyle = p.trim; c.lineWidth = 3; c.stroke();
      L(c, [[cx, sy + 6], [cx, by]], p.trim, 2);
      for (let i = 0; i < 3; i++) E(c, cx, sy + 10 + i * 7, 1.8, 1.8, p.trim); break;
    case 7: P(c, [[cx - halfTop + 2, sy], [cx + 12, by], [cx + 24, by], [cx - halfTop + 14, sy]], p.cloth2);
      L(c, [[cx - halfTop + 12, sy], [cx + 22, by]], p.trim, 2.4);
      c.beginPath(); c.arc(cx, sy + 2, 9, 0.15, Math.PI - 0.15); c.strokeStyle = p.trim; c.lineWidth = 2.4; c.stroke(); break;
    case 8: P(c, [[cx - 13, sy - 2], [cx + 2, sy + 4], [cx + 4, by], [cx - 16, by]], p.robeL);
      P(c, [[cx + 13, sy - 2], [cx - 2, sy + 4], [cx, by], [cx + 16, by]], p.robeD);
      L(c, [[cx + 13, sy - 2], [cx - 2, sy + 5]], p.trim, 2.4); break;
    case 9: band(sy + 4, 7, p.trim);
      for (let i = -3; i <= 3; i++) P(c, [[cx + i * 8 - 3, sy + 10], [cx + i * 8, sy + 5], [cx + i * 8 + 3, sy + 10]], p.cloth2); break;
    case 10: P(c, [[cx - halfTop, sy - 1], [cx + halfTop, sy - 1], [cx + halfTop - 4, sy + 9], [cx - halfTop + 4, sy + 9]], p.trim);
      for (let i = -3; i <= 3; i++) P(c, [[cx + i * 8, sy + 10], [cx + i * 8 + 4, sy + 16], [cx + i * 8, sy + 22], [cx + i * 8 - 4, sy + 16]], p.cloth2); break;
    case 11: P(c, [[cx - 12, sy - 2], [cx + 4, sy + 6], [cx + 4, by], [cx - 14, by]], p.robeL);
      for (let i = 0; i < 4; i++) { const y = sy + 6 + i * 7; L(c, [[cx - 10, y], [cx + 2, y]], p.trim, 2); E(c, cx + 2, y, 1.7, 1.7, p.trimL); } break;
    case 12: for (let i = 0; i < 3; i++) { c.beginPath(); c.arc(cx, sy - 4, 11 + i * 5, 0.08, Math.PI - 0.08); c.strokeStyle = i % 2 ? p.trim : p.cloth2; c.lineWidth = 3.4; c.stroke(); }
      break;
    case 13: P(c, [[cx - 10, sy], [cx, sy + 12], [cx + 10, sy]], p.robeD);
      E(c, cx, sy + 5, 4.4, 4.4, p.trim); L(c, [[cx, sy + 9], [cx, sy + 18]], p.trim, 1.8); break;
    case 14: P(c, [[cx - 14, sy - 3], [cx + 3, sy + 6], [cx + 5, by], [cx - 17, by]], "#e8e2d4");
      P(c, [[cx + 14, sy - 3], [cx - 3, sy + 6], [cx - 1, by], [cx + 17, by]], p.cloth2);
      P(c, [[cx + 12, sy - 1], [cx - 2, sy + 7], [cx - 1, sy + 13], [cx + 14, sy + 4]], p.robeL);
      band(by - 7, 7, p.trim); break;
    case 15: P(c, [[cx - 13, sy - 2], [cx + 6, sy + 5], [cx + 6, by], [cx - 15, by]], p.robeL);
      L(c, [[cx - 13, sy - 1], [cx + 6, sy + 6]], p.trim, 2.6);
      band(by - 8, 8, p.cloth2); break;
    case 16: for (let i = 0; i < 4; i++) band(sy + 2 + i * 6, 4, i % 2 ? p.trim : p.cloth2);
      P(c, [[cx - 8, sy - 3], [cx + 8, sy - 3], [cx + 5, sy + 5], [cx - 5, sy + 5]], p.skinD); break;
    default: P(c, [[cx - 15, sy - 3], [cx + 15, sy - 3], [cx + 11, sy + 8], [cx - 11, sy + 8]], p.robeD);
      for (let i = 0; i < 3; i++) { c.beginPath(); c.arc(cx, sy + 4, 8 + i * 4, 0.2, Math.PI - 0.2); c.strokeStyle = i % 2 ? p.trim : p.cloth2; c.lineWidth = 2.2; c.stroke(); }
      break;
  }
  void halfBot;
  if (occ === "ceremonial") band(by - 4, 4, p.trimL);
}

// ── the bust: 100 × 100 author box ─────────────────────────────────────────
function bustArt(c: Ctx, K: DressKit, occ: Occasion) {
  const p = pal(K, occ);
  const sy = 74, by = 100, halfTop = 26, halfBot = 46;
  R(c, HX - 8, HY + HRY - 6, 16, 14, p.skinD);
  P(c, [[HX - halfTop, sy], [HX + halfTop, sy], [HX + halfBot, by], [HX - halfBot, by]], p.robe);
  P(c, [[HX + 6, sy + 1], [HX + halfTop, sy], [HX + halfBot, by], [HX + 14, by]], p.robeD);
  collar(c, K.neck ?? K.id, p, occ, sy, halfTop, halfBot, by);
  headBlock(c, K, p, occ);
}

// ── the figure: 100 × 210 author box ───────────────────────────────────────
function figureArt(c: Ctx, K: DressKit, occ: Occasion) {
  const p = pal(K, occ);
  const cx = 50, shY = 62, waist = 118, foot = 202;
  const kind = K.garment;
  const wide = kind === "robe" || kind === "kaftan" || kind === "thobe" || kind === "boubou" || kind === "kimono" || kind === "deel";
  const legs = kind === "tunic" || kind === "poncho" || kind === "wrap";
  const half = kind === "boubou" ? 38 : kind === "thobe" ? 27 : kind === "poncho" ? 36 : kind === "wrap" ? 26 : 34;
  const hem = kind === "tunic" ? 134 : kind === "poncho" ? 146 : kind === "wrap" ? 156 : 186;

  if (occ === "ceremonial") P(c, [[cx - 27, shY], [cx + 27, shY], [cx + 41, 190], [cx - 41, 190]], p.cloth2D);

  // legs and feet — bare below a short garment, trousered under a tunic
  const legTop = legs ? hem - 6 : 170;
  for (const s of [-1, 1]) {
    R(c, cx + (s < 0 ? -16 : 3), legTop, 13, foot - 10 - legTop, legs ? p.cloth2 : p.skinD);
    R(c, cx + (s < 0 ? -19 : 3), foot - 11, 16, 11, "#33251a");
    R(c, cx + (s < 0 ? -19 : 3), foot - 11, 16, 2.5, "#4a3823");
  }

  // sleeves, drawn wider than the torso so the arm reads as an arm
  const aw = wide ? 16 : 10;
  for (const s of [-1, 1]) {
    P(c, [[cx + s * 22, shY - 2], [cx + s * (22 + aw), shY + 10], [cx + s * (20 + aw), waist + 16], [cx + s * 19, waist + 10]], s < 0 ? p.robe : p.robeD);
    if (wide) L(c, [[cx + s * (21 + aw), waist + 4], [cx + s * 20, waist + 14]], p.trim, 2.2);
    E(c, cx + s * (19 + aw * 0.35), waist + 22, 6, 6, p.skin);
  }

  // the garment itself
  P(c, [[cx - 24, shY], [cx + 24, shY], [cx + half, hem], [cx - half, hem]], p.robe);
  P(c, [[cx + 5, shY], [cx + 24, shY], [cx + half, hem], [cx + half * 0.32, hem]], p.robeD);
  for (const x of [-13, 0, 13]) L(c, [[cx + x, waist], [cx + x * 1.45, hem - 3]], p.robeD, 1.5);
  P(c, [[cx - half, hem - 5], [cx + half, hem - 5], [cx + half, hem], [cx - half, hem]], p.trim);

  if (kind === "toga") P(c, [[cx - 23, shY - 1], [cx + 7, waist + 18], [cx + 20, waist + 15], [cx - 9, shY - 2]], p.robeL);
  if (kind === "himation") P(c, [[cx + 23, shY - 1], [cx - 7, waist + 20], [cx - 20, waist + 17], [cx + 9, shY - 2]], p.robeL);
  if (kind === "sari") {
    P(c, [[cx - 22, shY - 1], [cx + 12, hem - 22], [cx + 25, hem - 19], [cx - 9, shY - 2]], p.cloth2);
    L(c, [[cx - 14, shY + 8], [cx + 23, hem - 22]], p.trim, 2.4);
  }
  if (kind === "kaftan" || kind === "deel") { R(c, cx - 4.5, shY, 9, hem - shY, p.cloth2); L(c, [[cx, shY], [cx, hem]], p.trim, 1.6); }
  if (kind === "crossrobe" || kind === "kimono") {
    P(c, [[cx - 17, shY - 2], [cx + 4, shY + 10], [cx + 6, hem], [cx - 20, hem]], p.robeL);
    L(c, [[cx + 17, shY - 2], [cx - 4, shY + 11]], p.trim, 2.6);
  }
  if (kind === "poncho") for (let i = 0; i < 5; i++) R(c, cx - 36 + i * 0.9, shY + 10 + i * 13, 72 - i * 1.8, 5, i % 2 ? p.trim : p.cloth2);
  if (kind === "boubou") for (let i = 0; i < 3; i++) { c.beginPath(); c.arc(cx, shY, 13 + i * 8, 0.25, Math.PI - 0.25); c.strokeStyle = i % 2 ? p.trim : p.cloth2; c.lineWidth = 2.8; c.stroke(); }
  if (kind === "wrap") { P(c, [[cx - 25, shY - 2], [cx + 9, shY + 8], [cx + 11, hem], [cx - 25, hem]], p.robeL); E(c, cx + 18, shY + 5, 5, 5, p.trim); }
  if (kind !== "boubou" && kind !== "wrap") P(c, [[cx - 25, waist - 5], [cx + 25, waist - 5], [cx + 26, waist + 8], [cx - 26, waist + 8]], p.cloth2);
  if (kind === "kimono" || kind === "deel") E(c, cx + 15, waist + 2, 4.4, 4.4, p.trim);
  if (kind === "thobe" || kind === "robe") L(c, [[cx, shY + 8], [cx, waist - 6]], p.trim, 2);

  R(c, cx - 8, 46, 16, 20, p.skinD);
  c.save();
  const s = 17 / HRX;
  c.translate(50 - HX * s, 32 - HY * s); c.scale(s, s);
  headBlock(c, K, p, occ);
  c.restore();
}

export interface DressOpts extends DeriveOpts { occasion?: Occasion; cols?: number }

/** One people's portrait bust, pixel-treated. `size` is the drawn square.
 *  `kit` is a preset index, a derived/creole kit object, or a culture name. */
export function drawBust(ctx: Ctx, x: number, y: number, size: number, kit: KitSpec, opts: DressOpts = {}) {
  const K = resolveKit(kit, opts);
  pixelize(ctx, x, y, size, size, opts.cols || 40, (c) => bustArt(c, K, opts.occasion || "national"));
}

/** One people's full costume plate, pixel-treated. `w` is the drawn width;
 *  the plate is `w × 2.1w`. */
export function drawFigure(ctx: Ctx, x: number, y: number, w: number, kit: KitSpec, opts: DressOpts = {}) {
  const K = resolveKit(kit, opts);
  pixelize(ctx, x, y, w, w * 2.1, opts.cols || 26, (c) => figureArt(c, K, opts.occasion || "national"));
}
