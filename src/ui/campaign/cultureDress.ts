// Cultural DRESS PLATES — a portrait bust and a small full figure per people,
// drawn in the same pixel treatment as the goods set (coarse author grid, hard
// dark edge, one-pixel bevel, nearest-neighbour upscale).
//
// `DRESS_KITS` is index-aligned to the old `cultureFigure.ts`'s KITS (since
// deleted) and therefore to `sim/cultures.rs` — kit 7 is Indic in both. It is
// a FLATTENED kit: the old SVG figure system picked one option per individual
// out of an array of hairs/robes/trims; a dress plate states the people's
// dress once, so each entry carries a single colour per axis.
//
// `cultureFigure.ts` drew an INDIVIDUAL — a named house head, with a sex the
// sim actually models (matrilineal succession, widow regents) and a
// per-person seed — while these plates draw a PEOPLE and carry no sex axis.
// That was real grounds to keep them separate, and this file's own history
// records the tradeoff rather than hiding it: the maintainer chose the
// unified look anyway (`HousesPanel.tsx`/`HouseCompare.tsx` now call
// `drawFigure` directly, same as `CultureFigures.tsx`), so a house's portrait
// is its seat culture's costume plate — not a likeness of the current head —
// and every head of the same culture at the same tier renders identically.
// The per-individual pose/build jitter `cultureFigure.ts` gave two heads is
// gone with it; the head's own name/sex/character still read from the text
// beside the portrait, not from the art.

import { pixelize, shade } from "@canvas/pixelize";

export type Occasion = "everyday" | "national" | "ceremonial";
/** The three registers every people has, whatever its kit's provenance. */
export const REGISTERS: Occasion[] = ["everyday", "national", "ceremonial"];

// ── 02_PEOPLE.md §Faces (slice 02.7) — the sex axis + acquired features ────
// A dress plate draws a PEOPLE and has no notion of a sex; an `Individual`
// (sim/campaign/tick/individuals.rs) does, and needs one for its portrait to
// differ between a male and female holder of the same seat. `female` and
// `features` are both OPTIONAL on `DressOpts` — every existing caller (the
// Peoples panel, a house head, a figure) omits them and renders exactly as
// before; only a caller that has a real `Individual` (`IndividualBrief`'s own
// `female`/`features` fields) passes them.
//
// Bit values mirror `individuals.rs`'s `FEATURE_*` consts exactly (ONE_EYED
// 1<<0 · SCARRED 1<<1 · LAME 1<<2 · BALD 1<<3 · GREY 1<<4 · TATTOOED 1<<5 ·
// MAIMED_HAND 1<<6) so a stored `Individual.features` value can be passed
// straight through with no translation.
export const FEATURE_ONE_EYED = 1 << 0;
export const FEATURE_SCARRED = 1 << 1;
/** `LAME` has no bust/figure-scale visual (a limp is not visible on a static
 *  portrait) — deliberately not drawn, not a gap: nothing here claims to. */
export const FEATURE_LAME = 1 << 2;
export const FEATURE_BALD = 1 << 3;
export const FEATURE_GREY = 1 << 4;
export const FEATURE_TATTOOED = 1 << 5;
/** Drawn only on the full figure (`drawFigure`) — a bust crops above the
 *  hands, so there is nothing for it to change there. */
export const FEATURE_MAIMED_HAND = 1 << 6;

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

export const mix = (a: string, b: string, t: number): string => {
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

const BY_NAME = new Map(DRESS_KITS.map((k) => [k.name.toLowerCase(), k]));

/** A people's kit from its NAME alone — a preset where the name is one of the
 *  eighteen, else a kit derived from the name. Used where the caller has a
 *  culture name but not the sim's kit index. */
export function kitForCulture(name: string): DressKit {
  return BY_NAME.get((name || "").trim().toLowerCase()) ?? deriveKit(name || "unknown");
}

/** An `Individual`'s own portrait kit (02.7): the culture's dress (garment,
 *  headwear, neckline) stays the people's own, but hair/beard/trim/cloth2 are
 *  varied off `faceSeed` — the same "vary per person, keep the people's cut"
 *  trick `FiguresPanel.tsx`'s `personalKit` already uses for a `Figure`, keyed
 *  on the stable numeric seed an `Individual` actually carries (`face_seed`)
 *  rather than their name, since two people can share a name. */
export function individualKit(culture: string, faceSeed: number): DressKit {
  const base = kitForCulture(culture);
  const v = deriveKit("face" + faceSeed, { seed: faceSeed || 1 });
  return { ...base, hair: v.hair, beard: v.beard, trim: v.trim, cloth2: v.cloth2 };
}

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

function pal(K: DressKit, occ: Occasion, features = 0): Pal {
  const dull = occ === "everyday" ? 0.86 : 1;
  const robe = shade(K.robe, dull);
  const hairBase = features & FEATURE_GREY ? mix(K.hair, "#cfc9bc", 0.7) : K.hair;
  return {
    skin: K.skin, skinD: shade(K.skin, 0.8), skinL: shade(K.skin, 1.12),
    hair: hairBase, hairL: shade(hairBase, 1.35),
    robe, robeL: shade(K.robe, dull * 1.18), robeD: shade(K.robe, dull * 0.72),
    trim: occ === "everyday" ? shade(K.trim, 0.8) : K.trim,
    trimL: shade(K.trim, 1.3),
    cloth2: shade(K.cloth2, dull), cloth2D: shade(K.cloth2, dull * 0.72),
    rich: occ === "ceremonial",
  };
}

// ── headwear: one distinct silhouette per people ───────────────────────────
function headwear(c: Ctx, id: number, p: Pal, occ: Occasion, g = false) {
  const top = HY - HRY, cx = HX;
  switch (id) {
    case 99: break; // bareheaded
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
      if (!g) for (const s of [-1, 1]) E(c, cx + s * 13, top + 4, 5, 4, p.hairL);
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
      if (!g) for (const s of [-1, 1]) {
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
      if (!g) L(c, [[cx - 3, top + 4], [cx, top - 1], [cx + 3, top + 4]], p.trim, 2.2);
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
      if (!g) L(c, [[cx + 2, top - 12], [cx + 12, top - 18]], p.hair, 4);
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
    case 101: // palla — a mantle drawn up over the hair
      P(c, [[cx - HRX - 4, HY + 6], [cx - HRX - 3, top + 2], [cx - 8, top - 5], [cx + 8, top - 5], [cx + HRX + 3, top + 2], [cx + HRX + 4, HY + 6], [cx + HRX + 7, HY + 34], [cx + HRX, HY + 34], [cx + HRX - 1, HY], [cx + 10, top + 5], [cx - 10, top + 5], [cx - HRX + 1, HY], [cx - HRX, HY + 34], [cx - HRX - 7, HY + 34]], p.cloth2);
      L(c, [[cx - HRX - 1, top + 6], [cx, top - 1], [cx + HRX + 1, top + 6]], shade(p.cloth2, 1.25), 1.6); break;
    case 102: // shawl wrapped close round the face
      P(c, [[cx - HRX - 4, HY + 4], [cx - HRX - 3, top - 1], [cx, top - 6], [cx + HRX + 3, top - 1], [cx + HRX + 4, HY + 4], [cx + HRX + 6, HY + 34], [cx - HRX - 6, HY + 34], [cx - HRX - 4, HY + 4], [cx - HRX + 2, HY + 14], [cx - 8, HY + HRY + 1], [cx + 8, HY + HRY + 1], [cx + HRX - 2, HY + 14], [cx + HRX - 1, HY - 4], [cx, top + 3], [cx - HRX + 1, HY - 4]], p.robeL);
      L(c, [[cx - HRX + 1, HY - 4], [cx, top + 3], [cx + HRX - 1, HY - 4]], p.trim, 1.6); break;
    case 103: // dupatta — falls from the back of the head, parting shows
      P(c, [[cx - HRX - 2, top + 8], [cx - 6, top - 3], [cx + 6, top - 3], [cx + HRX + 2, top + 8], [cx + HRX + 6, HY + 34], [cx + HRX + 1, HY + 34], [cx + HRX - 2, top + 12], [cx - HRX + 2, top + 12], [cx - HRX - 1, HY + 34], [cx - HRX - 6, HY + 34]], p.cloth2);
      L(c, [[cx - HRX - 1, top + 9], [cx, top - 1], [cx + HRX + 1, top + 9]], p.trim, 1.8); E(c, cx, HY - 7, 1.4, 1.4, p.trim); break;
    case 104: // gele — tall knotted headwrap
      P(c, [[cx - HRX - 2, top + 9], [cx + HRX + 2, top + 9], [cx + HRX + 7, top - 10], [cx + 6, top - 18], [cx - 6, top - 16], [cx - HRX - 6, top - 6]], p.cloth2);
      for (let i = 0; i < 3; i++) L(c, [[cx - HRX + i * 4, top + 6 - i * 2], [cx + HRX - 2 - i * 3, top - 4 - i * 4]], shade(p.cloth2, 0.78), 1.6);
      P(c, [[cx + 8, top - 16], [cx + 18, top - 24], [cx + 14, top - 12]], p.trim); P(c, [[cx - 4, top - 15], [cx - 12, top - 24], [cx - 10, top - 12]], shade(p.trim, 1.15)); break;
    case 105: // bun with hairpins
      E(c, cx, top - 4, 9, 6.5, p.hair); E(c, cx - 2, top - 6, 4, 2, p.hairL); L(c, [[cx - 14, top - 10], [cx + 4, top - 2]], p.trim, 1.6); L(c, [[cx + 14, top - 12], [cx - 2, top - 2]], p.trim, 1.6); E(c, cx - 14, top - 10, 1.8, 1.8, p.cloth2); break;
    case 106: // band over braids
      L(c, [[cx - HRX - 1, top + 9], [cx, top + 4], [cx + HRX + 1, top + 9]], p.trim, 3); for (let i = -2; i <= 2; i++) E(c, cx + i * 7, top + 6 - Math.abs(i) * 0.4, 1.6, 1.6, p.trimL); break;
    case 107: // braid ties
      for (const s of [-1, 1]) { R(c, cx + s * (HRX + 1) - 2.5, HY + 18, 5, 3, p.trim); R(c, cx + s * (HRX + 1.6) - 2.5, HY + 28, 5, 3, p.cloth2); } break;
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

/** Head, hair, face and headwear in the canonical frame. `female`/`features`
 *  are both optional (02.7): omitted, this renders bit-identically to before
 *  that slice. */
function headBlock(c: Ctx, K: DressKit, p: Pal, occ: Occasion, female?: boolean, features = 0) {
  const hat = K.hat ?? K.id, veiled = K.veil ?? (hat === 13);
  const bald = !!(features & FEATURE_BALD);
  for (const s of [-1, 1]) E(c, HX + s * (HRX - 1), HY + 3, 3.4, 4.2, p.skinD);
  if (!COVERED.has(hat) && !bald) {
    P(c, [[HX - HRX, HY - 6], [HX - HRX - 3, HY + 30], [HX + HRX + 3, HY + 30], [HX + HRX, HY - 6]], p.hair);
  }
  E(c, HX, HY, HRX, HRY, p.skin);
  P(c, [[HX + HRX - 7, HY - HRY + 4], [HX + HRX, HY - 2], [HX + HRX - 2, HY + HRY - 6], [HX + 6, HY + HRY - 1]], p.skinD);
  if (!COVERED.has(hat)) {
    if (bald) {
      E(c, HX, HY - HRY + 9, HRX - 4, 4.4, p.skinL); // a bare crown catches the light instead
    } else {
      P(c, [[HX - HRX - 1, HY - 6], [HX - HRX + 2, HY - HRY - 3], [HX, HY - HRY - 4], [HX + HRX - 2, HY - HRY - 3], [HX + HRX + 1, HY - 6],
        [HX + 7, HY - HRY + 7], [HX, HY - HRY + 8], [HX - 7, HY - HRY + 7]], p.hair);
    }
  } else if (!bald) {
    P(c, [[HX - HRX + 1, HY - 8], [HX + HRX - 1, HY - 8], [HX + 8, HY - HRY + 8], [HX - 8, HY - HRY + 8]], p.hair);
  }
  if (!veiled) {
    for (const s of [-1, 1]) L(c, [[HX + s * 3, HY - 5], [HX + s * 10, HY - 4]], shade(p.hair, 1.1), 1.8);
    const patchedSide = features & FEATURE_ONE_EYED ? -1 : 0; // always the same (left) eye — a face_seed-varied side needs no new bit, but every reader must agree which one, so it is fixed rather than re-derived per call
    for (const s of [-1, 1]) {
      if (s === patchedSide) {
        E(c, HX + s * 6.5, HY + 0.6, 2.6, 3, "#1c1712");
        L(c, [[HX + s * 12, HY - 3.5], [HX - s * 4, HY + 5]], "#1c1712", 1.3);
      } else {
        E(c, HX + s * 6.5, HY + 1, 2, 2.4, "#2a211c");
        E(c, HX + s * 5.6, HY + 0.2, 0.8, 0.9, "#f2ece4");
      }
    }
    L(c, [[HX, HY + 3], [HX, HY + 8]], p.skinD, 1.6);
    L(c, [[HX - 4, HY + 13], [HX, HY + 14.6], [HX + 4, HY + 13]], shade(p.skin, 0.66), 1.8);
    if (features & FEATURE_SCARRED) {
      L(c, [[HX + 8, HY - 9], [HX + 3, HY + 12]], shade(p.skin, 0.5), 1.1);
    }
    if (features & FEATURE_TATTOOED) {
      for (let i = 0; i < 3; i++) L(c, [[HX - HRX + 2, HY - 7 + i * 4], [HX - HRX + 6, HY - 5 + i * 4]], p.trim, 1);
    }
    if (K.beard && occ !== "everyday" && !female) {
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
function bustArt(c: Ctx, K: DressKit, occ: Occasion, female?: boolean, features = 0) {
  const p = pal(K, occ, features);
  const sy = 74, by = 100, halfTop = 26, halfBot = 46;
  R(c, HX - 8, HY + HRY - 6, 16, 14, p.skinD);
  P(c, [[HX - halfTop, sy], [HX + halfTop, sy], [HX + halfBot, by], [HX - halfBot, by]], p.robe);
  P(c, [[HX + 6, sy + 1], [HX + halfTop, sy], [HX + halfBot, by], [HX + 14, by]], p.robeD);
  collar(c, K.neck ?? K.id, p, occ, sy, halfTop, halfBot, by);
  headBlock(c, K, p, occ, female, features);
}

// ── the figure: 100 × 210 author box ───────────────────────────────────────
function figureArt(c: Ctx, K: DressKit, occ: Occasion, female?: boolean, features = 0) {
  const p = pal(K, occ, features);
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
  const maimedSide = features & FEATURE_MAIMED_HAND ? 1 : 0; // fixed to the same (right) hand for every reader
  for (const s of [-1, 1]) {
    P(c, [[cx + s * 22, shY - 2], [cx + s * (22 + aw), shY + 10], [cx + s * (20 + aw), waist + 16], [cx + s * 19, waist + 10]], s < 0 ? p.robe : p.robeD);
    if (wide) L(c, [[cx + s * (21 + aw), waist + 4], [cx + s * 20, waist + 14]], p.trim, 2.2);
    const hx = cx + s * (19 + aw * 0.35), hy = waist + 22;
    if (s === maimedSide) {
      E(c, hx, hy, 5.4, 5.4, "#d8cdb8");
      L(c, [[hx - 4, hy - 3], [hx + 4, hy + 3]], "#a89a7c", 1.2);
      L(c, [[hx - 4, hy + 3], [hx + 4, hy - 3]], "#a89a7c", 1.2);
    } else {
      E(c, hx, hy, 6, 6, p.skin);
    }
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
  headBlock(c, K, p, occ, female, features);
  c.restore();
}

export interface DressOpts extends DeriveOpts {
  occasion?: Occasion; cols?: number;
  /** 02.7 — an `Individual`'s sex; omitted for a plain people/culture plate. */
  female?: boolean;
  /** 02.7 — an `Individual.features` bitflag value (`FEATURE_*` above);
   *  omitted or 0 renders bit-identically to before this slice. */
  features?: number;
  /** A per-person face genome from `makePerson` — when present the head is
   *  drawn from it (eyes, nose, brows, hair, beard, age) instead of the
   *  people's single shared face. Omitted ⇒ bit-identical to before. */
  person?: Person;
  /** With `person`: draw the head bare in the everyday register. */
  bare?: boolean;
}

/** One people's portrait bust, pixel-treated. `size` is the drawn square.
 *  `kit` is a preset index, a derived/creole kit object, or a culture name. */
export function drawBust(ctx: Ctx, x: number, y: number, size: number, kit: KitSpec, opts: DressOpts = {}) {
  const K = resolveKit(kit, opts);
  if (opts.person) { const Pn = opts.person; pixelize(ctx, x, y, size, size, opts.cols || 44, (c) => bustArtGene(c, Pn, opts.occasion || "national", opts.bare ?? Pn.G.bare)); return; }
  pixelize(ctx, x, y, size, size, opts.cols || 40, (c) => bustArt(c, K, opts.occasion || "national", opts.female, opts.features ?? 0));
}

/** One people's full costume plate, pixel-treated. `w` is the drawn width;
 *  the plate is `w × 2.1w`. */
export function drawFigure(ctx: Ctx, x: number, y: number, w: number, kit: KitSpec, opts: DressOpts = {}) {
  const K = resolveKit(kit, opts);
  if (opts.person) { const Pn = opts.person; pixelize(ctx, x, y, w, w * 2.1, opts.cols || 26, (c) => figureArtGene(c, Pn, opts.occasion || "national", opts.bare ?? Pn.G.bare)); return; }
  pixelize(ctx, x, y, w, w * 2.1, opts.cols || 26, (c) => figureArt(c, K, opts.occasion || "national", opts.female, opts.features ?? 0));
}

// ═══ Per-person face genome (design handoff "Council & People", wf-faces.js) ═══
// Two people of one culture no longer share a face: eye shape + colour, nose,
// brows, face/jaw, lips, hair style, beard style, age lines and greying are
// drawn per person from a seeded genome whose weights come from the culture's
// PHENOTYPE GROUP. Skin follows the HOMELAND's latitude, not the culture.

function rng(seed: number) {
  let s = (seed >>> 0) || 1;
  return () => { s = (s + 0x6D2B79F5) >>> 0; let t = s; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}
type Wt = [string, number][];
const Wpick = (r: () => number, arr: Wt): string => {
  let t = 0; for (const a of arr) t += a[1];
  let x = r() * t; for (const a of arr) { x -= a[1]; if (x <= 0) return a[0]; }
  return arr[arr.length - 1][0];
};

const FACE_SKINS = ["#f4d6b8", "#ecc7a2", "#e3b78c", "#d8ac78", "#cc9a68", "#bb8858", "#a6744a", "#8c5c3a", "#6f472c", "#5a3922"];
const HAIR_TONES: Record<string, string> = { flax: "#d8c08a", blond: "#c09a52", red: "#a8431e", auburn: "#7e3a22", chestnut: "#6a4228", brown: "#4a3220", dkbrown: "#2e2018", black: "#16110d", blue: "#0f1014" };
export const IRIS: Record<string, string> = { dk: "#24160c", br: "#4e2e16", hz: "#7a5a2a", gr: "#4f7044", bl: "#4a6c9c", gy: "#6c7c88", am: "#8a6420" };

/** Kit index → phenotype group. */
export const PHENO_GROUP: Record<number, string> = { 0: "med", 1: "med", 2: "med", 3: "wasia", 4: "north", 5: "north", 6: "wasia", 7: "sasia", 8: "easia", 9: "north", 10: "amer", 11: "steppe", 12: "afr", 13: "wasia", 14: "easia", 15: "steppe", 16: "amer", 17: "afr" };
interface Pheno { eye: Wt; iris: Wt; nose: Wt; brow: Wt; face: Wt; lips: Wt; hair: Wt; hsM: Wt; hsF: Wt; beard: Wt; fhat: string; grey: number }
const PH: Record<string, Pheno> = {
  north: { eye: [["round", 3], ["deep", 3], ["almond", 1.5], ["hooded", 1]], iris: [["bl", 4], ["gy", 3], ["gr", 2], ["hz", 1.5], ["br", 1]], nose: [["straight", 3], ["button", 2], ["long", 2], ["aquiline", 1]], brow: [["thin", 2], ["straight", 3], ["thick", 1], ["angled", 1]], face: [["long", 2], ["square", 3], ["oval", 2], ["broad", 1]], lips: [["thin", 4], ["mid", 3]], hair: [["flax", 3], ["blond", 3], ["red", 1.5], ["chestnut", 2], ["brown", 2]], hsM: [["crop", 3], ["long", 3], ["braids", 2], ["receding", 1.5]], hsF: [["braids", 4], ["long", 3], ["bun", 1]], beard: [["full", 4], ["long", 2], ["forked", 1.5], ["braided", 1.5], ["moustache", 1], ["none", 0.6]], fhat: "band", grey: 1.1 },
  med: { eye: [["almond", 4], ["round", 2], ["large", 2], ["hooded", 1]], iris: [["br", 4], ["dk", 3], ["hz", 2], ["gr", 1]], nose: [["aquiline", 3], ["straight", 3], ["long", 2], ["broad", 0.6]], brow: [["thick", 3], ["arched", 2], ["straight", 2]], face: [["oval", 4], ["square", 2], ["heart", 1.5], ["long", 1]], lips: [["mid", 4], ["full", 2], ["thin", 1]], hair: [["dkbrown", 4], ["black", 3], ["brown", 2], ["chestnut", 1]], hsM: [["crop", 4], ["curly", 3], ["receding", 2]], hsF: [["bun", 3], ["curly", 2], ["long", 2]], beard: [["none", 4], ["stubble", 2], ["full", 2], ["goatee", 1]], fhat: "palla", grey: 1 },
  wasia: { eye: [["almond", 4], ["deep", 2], ["large", 2], ["hooded", 1]], iris: [["dk", 4], ["br", 4], ["hz", 1.5], ["gr", 1]], nose: [["aquiline", 4], ["long", 3], ["straight", 2]], brow: [["thick", 4], ["joined", 2], ["arched", 2]], face: [["long", 3], ["oval", 3], ["heart", 1]], lips: [["mid", 3], ["full", 2], ["thin", 1]], hair: [["black", 5], ["dkbrown", 3]], hsM: [["crop", 3], ["curly", 2], ["receding", 1]], hsF: [["long", 4], ["braids", 1]], beard: [["full", 5], ["long", 2], ["goatee", 1], ["moustache", 1]], fhat: "shawl", grey: 0.9 },
  sasia: { eye: [["large", 4], ["almond", 4], ["round", 1]], iris: [["dk", 5], ["br", 3], ["hz", 1], ["am", 0.5]], nose: [["straight", 3], ["broad", 2], ["aquiline", 2], ["button", 1]], brow: [["arched", 4], ["thick", 2], ["joined", 1]], face: [["oval", 4], ["round", 2], ["heart", 2]], lips: [["full", 4], ["mid", 3]], hair: [["black", 6], ["dkbrown", 2]], hsM: [["crop", 3], ["topknot", 2], ["long", 1]], hsF: [["long", 3], ["bun", 3], ["braids", 2]], beard: [["moustache", 4], ["full", 2], ["none", 2], ["mutton", 1]], fhat: "dupatta", grey: 0.9 },
  easia: { eye: [["mono", 6], ["almond", 2], ["hooded", 1.5]], iris: [["dk", 6], ["br", 2]], nose: [["flat", 4], ["button", 3], ["straight", 1.5]], brow: [["straight", 4], ["thin", 3], ["angled", 1]], face: [["round", 3], ["broad", 3], ["oval", 2], ["square", 1]], lips: [["mid", 3], ["thin", 3], ["full", 1]], hair: [["black", 6], ["blue", 3]], hsM: [["topknot", 4], ["crop", 2], ["long", 1]], hsF: [["bun", 5], ["long", 2]], beard: [["none", 5], ["moustache", 2], ["goatee", 2]], fhat: "pins", grey: 0.8 },
  steppe: { eye: [["mono", 4], ["almond", 3], ["hooded", 2], ["deep", 0.6]], iris: [["dk", 4], ["br", 3], ["hz", 1], ["gr", 0.5]], nose: [["flat", 2], ["straight", 2], ["aquiline", 2], ["broad", 1]], brow: [["thick", 3], ["straight", 2], ["angled", 2]], face: [["broad", 4], ["round", 2], ["square", 2]], lips: [["thin", 3], ["mid", 3]], hair: [["black", 5], ["dkbrown", 2], ["brown", 1]], hsM: [["braids", 3], ["topknot", 2], ["shaved", 2], ["crop", 1]], hsF: [["braids", 5], ["long", 1]], beard: [["moustache", 4], ["goatee", 2], ["none", 2], ["full", 1]], fhat: "braids", grey: 0.9 },
  afr: { eye: [["large", 4], ["round", 3], ["almond", 2]], iris: [["dk", 6], ["br", 2]], nose: [["broad", 5], ["button", 2], ["straight", 1]], brow: [["thin", 2], ["arched", 2], ["straight", 2]], face: [["long", 3], ["oval", 3], ["round", 1], ["heart", 1]], lips: [["full", 6], ["mid", 2]], hair: [["black", 6], ["blue", 2]], hsM: [["coils", 4], ["shaved", 3], ["topknot", 1]], hsF: [["coils", 3], ["braids", 3]], beard: [["none", 4], ["goatee", 2], ["full", 1.5], ["stubble", 1]], fhat: "gele", grey: 0.8 },
  amer: { eye: [["almond", 4], ["hooded", 3], ["mono", 1]], iris: [["dk", 6], ["br", 2]], nose: [["aquiline", 4], ["broad", 3], ["straight", 1]], brow: [["straight", 3], ["thick", 2], ["thin", 1]], face: [["broad", 4], ["square", 2], ["round", 2]], lips: [["mid", 4], ["full", 2]], hair: [["black", 6], ["blue", 2]], hsM: [["long", 3], ["topknot", 2], ["crop", 2]], hsF: [["braids", 5], ["long", 2]], beard: [["none", 6], ["stubble", 1]], fhat: "braids", grey: 0.7 },
};
const FACE_DIMS: Record<string, [number, number]> = { oval: [19, 22], round: [20.5, 20.6], long: [17.6, 23.6], square: [19.8, 21.6], heart: [19.6, 21.6], broad: [21.4, 21] };
const COVERED_G = new Set([2, 3, 4, 6, 7, 8, 9, 11, 13, 15, 16, 17, 101, 102, 104]);
const FEM_HAT: Record<string, number> = { palla: 101, shawl: 102, dupatta: 103, gele: 104, pins: 105, band: 106, braids: 107 };

export interface Gene {
  female: boolean; age: number; features: number; skin: string;
  eye: string; iris: string; eyeSet: number; nose: string; brow: string; face: string; lips: string;
  hs: string; beard: string; hairC: string; bald: boolean; hat: number; bare: boolean; earring: boolean;
}
export interface Person { K: DressKit; G: Gene; seed: number; kit: number; culture: string }
export interface PersonSpec {
  seed?: number | string; name?: string; kit?: KitSpec; female?: boolean; age?: number;
  /** Homeland latitude, 0 equator → 1 polar. Drives skin. */
  lat?: number; features?: number; hat?: number; bare?: boolean;
}

const colorDist = (a: string, b: string) => {
  const p = (h: string) => { const t = h.replace("#", ""); return [0, 2, 4].map((i) => parseInt(t.slice(i, i + 2), 16)); };
  const A = p(a), B = p(b); return Math.abs(A[0] - B[0]) + Math.abs(A[1] - B[1]) + Math.abs(A[2] - B[2]);
};

/** One person: their culture's dress kit with personal dyes, and a face genome. */
export function makePerson(spec: PersonSpec = {}): Person {
  const base = resolveKit(spec.kit ?? 0);
  const seed = spec.seed != null ? (typeof spec.seed === "number" ? spec.seed : strSeed(spec.seed)) : strSeed(spec.name || "p");
  const r = rng(seed ^ 0x51ed), grp = PH[PHENO_GROUP[base.id] ?? "med"] || PH.med, female = !!spec.female;
  const age = spec.age ?? Math.round(28 + r() * 36);
  let skinI = spec.lat != null ? (1 - spec.lat) * (FACE_SKINS.length - 1)
    : FACE_SKINS.reduce((b, sk, i) => (colorDist(sk, base.skin) < colorDist(FACE_SKINS[b], base.skin) ? i : b), 0);
  skinI = Math.max(0, Math.min(FACE_SKINS.length - 1, Math.round(skinI + (r() - 0.5) * 1.4)));
  const tier = r() > 0.7 ? DYES.costly : DYES.fine;
  const K: DressKit = { ...base, trim: r() < 0.5 ? base.trim : tier[Math.floor(r() * tier.length)], cloth2: r() < 0.45 ? base.cloth2 : DYES.fine[Math.floor(r() * DYES.fine.length)] };
  let hairC = HAIR_TONES[Wpick(r, grp.hair)] || base.hair;
  if (r() < 0.35) hairC = base.hair;
  const greyT = Math.max(0, Math.min(1, (age - 44) / 28)) * (grp.grey || 1) * (0.6 + r() * 0.6);
  const features = spec.features || 0;
  const G: Gene = {
    female, age, features, skin: FACE_SKINS[skinI],
    eye: Wpick(r, grp.eye), iris: IRIS[Wpick(r, grp.iris)], eyeSet: 6 + r() * 1.6,
    nose: Wpick(r, grp.nose), brow: Wpick(r, grp.brow), face: Wpick(r, grp.face), lips: Wpick(r, grp.lips),
    hs: Wpick(r, female ? grp.hsF : grp.hsM),
    beard: female ? "none" : (base.beard || r() < 0.3 ? Wpick(r, grp.beard) : (r() < 0.7 ? "none" : "stubble")),
    hairC: features & FEATURE_GREY ? mix(hairC, "#d2ccc0", 0.75) : mix(hairC, "#cfc9bc", Math.min(0.85, greyT)),
    bald: !!(features & FEATURE_BALD) || (!female && age > 50 && r() < 0.18),
    hat: spec.hat != null ? spec.hat : (female ? (r() < 0.78 ? FEM_HAT[grp.fhat] : (base.hat ?? base.id)) : (base.hat ?? base.id)),
    bare: spec.bare != null ? spec.bare : r() < 0.42,
    earring: female ? r() < 0.7 : r() < 0.15,
  };
  if (G.bald && G.hs !== "shaved") G.hs = "receding";
  return { K, G, seed, kit: base.id, culture: base.name };
}

/** Human-readable genome, for the person sheet. */
export function describePerson(Pn: Person): string {
  const G = Pn.G;
  const eyeW: Record<string, string> = { round: "round", deep: "deep-set", large: "large", almond: "almond", hooded: "hooded", mono: "narrow, single-lidded" };
  const irisW = Object.entries(IRIS).find(([, v]) => v === G.iris)?.[0] || "";
  const IW: Record<string, string> = { dk: "dark", br: "brown", hz: "hazel", gr: "green", bl: "blue", gy: "grey", am: "amber" };
  return `${eyeW[G.eye]} ${IW[irisW] || ""} eyes · ${G.nose} nose · ${G.face} face`;
}

function palG(K: DressKit, G: Gene, occ: Occasion): Pal {
  const dull = occ === "everyday" ? 0.86 : 1, robe = shade(K.robe, dull);
  return {
    skin: G.skin, skinD: shade(G.skin, 0.8), skinL: shade(G.skin, 1.12), hair: G.hairC, hairL: shade(G.hairC, 1.35),
    robe, robeL: shade(K.robe, dull * 1.18), robeD: shade(K.robe, dull * 0.72),
    trim: occ === "everyday" ? shade(K.trim, 0.8) : K.trim, trimL: shade(K.trim, 1.3),
    cloth2: shade(K.cloth2, dull), cloth2D: shade(K.cloth2, dull * 0.72), rich: occ === "ceremonial",
  };
}

function eyeG(c: Ctx, x: number, y: number, s: number, G: Gene, p: Pal) {
  const sc = "#ece3d4", lid = shade(G.hairC, 0.55), ir = G.iris, pu = "#120c08", sh = G.eye;
  if (sh === "deep") E(c, x, y - 1.3, 4.4, 3.1, p.skinD);
  if (sh === "round" || sh === "deep") { E(c, x, y, 2.9, 2.2, sc); E(c, x + 0.2, y + 0.1, 1.8, 1.9, ir); E(c, x + 0.2, y, 0.8, 0.9, pu); L(c, [[x - 3, y - 1.4], [x, y - 2.5], [x + 3, y - 1.4]], lid, 1.2); }
  else if (sh === "large") { E(c, x, y, 3.4, 2.7, sc); E(c, x, y + 0.1, 2.2, 2.3, ir); E(c, x, y, 1, 1.1, pu); E(c, x - 0.8, y - 0.8, 0.6, 0.6, "#fbf6ee"); L(c, [[x - 3.6, y - 1.4], [x, y - 3], [x + 3.6, y - 1.4]], lid, 1.5); }
  else if (sh === "almond" || sh === "hooded") {
    P(c, [[x - 3.8, y + 0.2], [x - 1.2, y - 1.8], [x + 1.4, y - 1.8], [x + 3.8, y - 0.3], [x + 1.4, y + 1.5], [x - 1.4, y + 1.4]], sc);
    E(c, x, y, 1.7, 1.7, ir); E(c, x, y, 0.8, 0.8, pu);
    if (sh === "hooded") P(c, [[x - 4, y - 0.3], [x, y - 3], [x + 4, y - 0.9], [x + 3.8, y - 0.1], [x, y - 1.1], [x - 3.8, y + 0.2]], p.skinD);
    L(c, [[x - 3.8, y + 0.1], [x - 1, y - 1.9], [x + 1.6, y - 1.8], [x + 3.9, y - 0.4]], lid, 1.3);
  } else if (sh === "mono") {
    const ix = x - s * 3.5, ox = x + s * 3.5;
    P(c, [[ix, y + 0.5], [x, y - 1.1], [ox, y - 1], [x, y + 1.1]], sc); E(c, x, y, 1.4, 1.05, ir);
    L(c, [[ix, y + 0.2], [x, y - 1.3], [ox, y - 1.4]], lid, 1.8);
  }
  if (G.female) L(c, [[x + s * 3.4, y - 1.2], [x + s * 4.8, y - 2.4]], lid, 1.1);
}
function browsG(c: Ctx, G: Gene) {
  const col = shade(G.hairC, 0.82), y = HY - (G.eye === "deep" ? 4.2 : 5.2);
  const w = ({ thin: 1.1, straight: 2.1, thick: 2.9, arched: 1.8, joined: 2.7, angled: 2 } as Record<string, number>)[G.brow] || 1.8;
  const e = G.eyeSet;
  for (const s of [-1, 1]) {
    const a = HX + s * (e - 3.6), b = HX + s * (e + 4);
    if (G.brow === "arched") L(c, [[a, y + 0.5], [HX + s * (e + 0.4), y - 1.7], [b, y + 1]], col, w);
    else if (G.brow === "angled") L(c, [[a, y + 0.8], [b, y - 1.4]], col, w);
    else L(c, [[a, y], [b, y + (G.brow === "thin" ? 0.6 : 0.2)]], col, w);
  }
  if (G.brow === "joined") L(c, [[HX - 2.6, y + 0.4], [HX + 2.6, y + 0.4]], col, 1.3);
}
function noseG(c: Ctx, G: Gene, p: Pal) {
  const d = shade(G.skin, 0.7), n = shade(G.skin, 0.52), hi = p.skinL;
  switch (G.nose) {
    case "aquiline": P(c, [[HX + 0.6, HY + 1], [HX + 3, HY + 5.6], [HX + 1.8, HY + 9.6], [HX, HY + 9], [HX + 1, HY + 5]], d); E(c, HX - 1.8, HY + 9.4, 0.9, 0.7, n); E(c, HX + 2.4, HY + 9.4, 0.9, 0.7, n); break;
    case "button": E(c, HX + 0.8, HY + 7.8, 2.6, 1.8, d); E(c, HX, HY + 7, 1.2, 1, hi); E(c, HX - 1.6, HY + 8.8, 0.7, 0.6, n); E(c, HX + 1.8, HY + 8.8, 0.7, 0.6, n); break;
    case "broad": L(c, [[HX + 0.3, HY + 2], [HX + 0.8, HY + 6]], d, 1.4); E(c, HX + 0.6, HY + 8.4, 4.4, 2.2, d); E(c, HX - 2.6, HY + 8.8, 1.2, 0.9, n); E(c, HX + 3.2, HY + 8.8, 1.2, 0.9, n); E(c, HX, HY + 7.4, 1.3, 0.9, hi); break;
    case "long": L(c, [[HX + 0.4, HY + 1], [HX + 1.4, HY + 10.4]], d, 1.7); E(c, HX + 0.9, HY + 10.4, 2.4, 1.2, d); E(c, HX - 1.4, HY + 11, 0.7, 0.6, n); E(c, HX + 2.6, HY + 11, 0.7, 0.6, n); break;
    case "flat": E(c, HX + 0.6, HY + 7.4, 3.4, 1.7, d); E(c, HX - 1.8, HY + 8, 0.9, 0.7, n); E(c, HX + 2.6, HY + 8, 0.9, 0.7, n); break;
    default: L(c, [[HX + 0.2, HY + 2], [HX + 1.2, HY + 8]], d, 1.6); E(c, HX - 1.5, HY + 8.6, 0.8, 0.6, n); E(c, HX + 2.2, HY + 8.6, 0.8, 0.6, n);
  }
}
function mouthG(c: Ctx, G: Gene, full: boolean) {
  const y = HY + 13.6 + (G.face === "long" ? 1 : 0), lip = mix(G.skin, "#9a3838", G.female ? 0.4 : 0.2), ln = shade(G.skin, 0.55);
  if (G.lips === "full") { P(c, [[HX - 4.6, y], [HX - 1.6, y - 1.4], [HX, y - 0.8], [HX + 1.6, y - 1.4], [HX + 4.6, y]], lip); E(c, HX, y + 1.6, 3.8, 1.9, lip); L(c, [[HX - 4.4, y], [HX + 4.4, y]], ln, 1.1); }
  else if (G.lips === "mid") { E(c, HX, y + 1.3, 3, 1.2, lip); L(c, [[HX - 4, y - 0.2], [HX, y + 0.5], [HX + 4, y - 0.2]], ln, 1.3); }
  else L(c, [[HX - 4, y - 0.2], [HX, y + 0.6], [HX + 4, y - 0.2]], ln, 1.3);
  if (G.age > 60 && !full) L(c, [[HX - 5, y + 1], [HX - 6, y + 3]], shade(G.skin, 0.72), 0.8);
}
function beardG(c: Ctx, G: Gene, p: Pal, rx: number): (() => void) | null {
  const h = p.hair, hl = shade(p.hair, 1.2), b = G.beard;
  const must = () => P(c, [[HX - 6, HY + 13.8], [HX - 2.2, HY + 11], [HX, HY + 11.6], [HX + 2.2, HY + 11], [HX + 6, HY + 13.8], [HX + 4, HY + 13], [HX, HY + 12.4], [HX - 4, HY + 13]], h);
  const fullP = (lo: number) => P(c, [[HX - rx + 2, HY + 3], [HX - rx + 3, HY + 19], [HX - 4, HY + lo - 2], [HX, HY + lo], [HX + 4, HY + lo - 2], [HX + rx - 3, HY + 19], [HX + rx - 2, HY + 3], [HX + 8, HY + 14], [HX + 4, HY + 16], [HX - 4, HY + 16], [HX - 8, HY + 14]], h);
  if (b === "stubble") { c.globalAlpha = 0.38; fullP(23); c.globalAlpha = 1; }
  else if (b === "full") { fullP(27); L(c, [[HX - 5, HY + 21], [HX + 5, HY + 21]], hl, 1); }
  else if (b === "long") { fullP(36); L(c, [[HX - 2, HY + 24], [HX - 1, HY + 33]], hl, 1); L(c, [[HX + 3, HY + 22], [HX + 3, HY + 31]], hl, 1); }
  else if (b === "forked") { P(c, [[HX - rx + 2, HY + 3], [HX - rx + 3, HY + 19], [HX - 7, HY + 34], [HX - 3, HY + 36], [HX, HY + 28], [HX + 3, HY + 36], [HX + 7, HY + 34], [HX + rx - 3, HY + 19], [HX + rx - 2, HY + 3], [HX + 8, HY + 14], [HX - 8, HY + 14]], h); }
  else if (b === "braided") { fullP(26); R(c, HX - 2.2, HY + 24, 4.4, 12, h); for (let i = 0; i < 3; i++) R(c, HX - 2.4, HY + 26 + i * 3.6, 4.8, 1.2, p.trim); }
  else if (b === "goatee") { P(c, [[HX - 4, HY + 16.4], [HX + 4, HY + 16.4], [HX + 3, HY + 22], [HX, HY + 24.5], [HX - 3, HY + 22]], h); }
  else if (b === "mutton") { for (const s of [-1, 1]) P(c, [[HX + s * (rx - 1), HY - 2], [HX + s * (rx - 1), HY + 14], [HX + s * 8, HY + 16], [HX + s * (rx - 5), HY + 4]], h); }
  return ["moustache", "goatee", "full", "long", "forked", "braided", "mutton"].includes(b) ? must : null;
}
function hairBackG(c: Ctx, G: Gene, p: Pal, rx: number, covered: boolean) {
  if (G.bald || ["shaved", "coils", "crop", "receding"].includes(G.hs)) return;
  if (G.hs === "long" || (G.female && (G.hs === "curly" || G.hs === "braids") && !covered)) {
    const lo = G.female ? 36 : 28;
    P(c, [[HX - rx - 1, HY - 8], [HX - rx - 5, HY + lo], [HX - rx + 4, HY + lo + 2], [HX + rx - 4, HY + lo + 2], [HX + rx + 5, HY + lo], [HX + rx + 1, HY - 8]], shade(p.hair, 0.88));
  }
}
function hairCapG(c: Ctx, G: Gene, p: Pal, rx: number, ry: number) {
  const top = HY - ry, h = p.hair;
  const crop = () => P(c, [[HX - rx - 1, HY - 6], [HX - rx + 2, top - 3], [HX, top - 4], [HX + rx - 2, top - 3], [HX + rx + 1, HY - 6], [HX + 7, top + 7], [HX, top + 8], [HX - 7, top + 7]], h);
  if (G.bald) { E(c, HX, top + 9, rx - 4, 4.4, p.skinL); for (const s of [-1, 1]) P(c, [[HX + s * (rx + 1), HY - 2], [HX + s * (rx - 1), top + 8], [HX + s * (rx - 5), top + 10], [HX + s * (rx - 3), HY - 2]], h); return; }
  switch (G.hs) {
    case "receding": for (const s of [-1, 1]) P(c, [[HX + s * (rx + 1), HY - 4], [HX + s * (rx - 1), top + 1], [HX + s * (rx - 6), top - 2], [HX + s * (rx - 6), top + 6], [HX + s * (rx - 2), HY - 6]], h); E(c, HX, top + 1, rx - 6, 3, h); E(c, HX, top + 5, 6, 2.6, p.skinL); break;
    case "curly": crop(); for (let i = 0; i <= 8; i++) { const a = Math.PI * (1.05 + i * 0.1125); E(c, HX + Math.cos(a) * (rx + 0.5), HY - 3 + Math.sin(a) * (ry + 1), 4.2, 4, i % 2 ? h : p.hairL); } break;
    case "coils": P(c, [[HX - rx - 1.4, HY - 5], [HX - rx + 1, top - 4], [HX, top - 6], [HX + rx - 1, top - 4], [HX + rx + 1.4, HY - 5], [HX + 7, top + 6], [HX - 7, top + 6]], h); for (let i = -3; i <= 3; i++) for (let j = 0; j < 2; j++) E(c, HX + i * 4.6 + j * 2.2, top - 2 + j * 4, 1, 1, p.hairL); break;
    case "shaved": E(c, HX, top + 6, rx - 1.5, 7.5, mix(G.skin, h, 0.32)); break;
    case "topknot": crop(); E(c, HX, top - 5, 5.4, 4.8, h); E(c, HX - 1.4, top - 6.4, 2, 1.4, p.hairL); R(c, HX - 3, top - 1.6, 6, 1.8, p.trim); break;
    case "bun": crop(); E(c, HX + rx - 4, top + 2, 6, 5.4, h); E(c, HX + rx - 5.4, top + 0.6, 2.2, 1.4, p.hairL); break;
    case "braids": crop(); L(c, [[HX, top - 3], [HX, top + 7]], p.skinD, 0.9); break;
    case "long": crop(); if (G.female) L(c, [[HX - 1, top - 3], [HX - 1, top + 7]], p.skinD, 0.9); break;
    default: crop();
  }
}
function braidsG(c: Ctx, p: Pal, rx: number) {
  for (const s of [-1, 1]) { L(c, [[HX + s * (rx - 1), HY + 4], [HX + s * (rx + 3), HY + 30]], p.hair, 5); for (let i = 0; i < 4; i++) E(c, HX + s * (rx + 0.6 + i * 0.9), HY + 9 + i * 5.6, 2.6, 2.1, p.hairL); }
}
function ageLinesG(c: Ctx, G: Gene, p: Pal, rx: number) {
  const ln = shade(G.skin, 0.7), a = G.age;
  if (a >= 38) for (const s of [-1, 1]) L(c, [[HX + s * 3.6, HY + 8.4], [HX + s * 6.2, HY + 14]], ln, 0.9);
  if (a >= 48) { L(c, [[HX - 6, HY - 11], [HX + 6, HY - 11.6]], ln, 0.8); if (a >= 56) L(c, [[HX - 4, HY - 13.6], [HX + 4.6, HY - 13.8]], ln, 0.8); }
  if (a >= 54) for (const s of [-1, 1]) { const x = HX + s * G.eyeSet; L(c, [[x - 1.8, HY + 3.4], [x + 1.8, HY + 3.4]], ln, 0.8); L(c, [[x + s * 4.4, HY + 0.2], [x + s * 6, HY + 1.4]], ln, 0.8); }
  if (a >= 64) for (const s of [-1, 1]) P(c, [[HX + s * (rx - 3), HY + 10], [HX + s * (rx - 2), HY + 17], [HX + s * (rx - 6), HY + 20]], p.skinD);
}

function headBlockG(c: Ctx, K: DressKit, G: Gene, p: Pal, occ: Occasion, bare: boolean) {
  let hat = G.hat; if (bare && occ === "everyday" && hat < 99 && !G.female) hat = 99;
  const [rx, ry] = FACE_DIMS[G.face] || FACE_DIMS.oval, veiled = hat === 13, covered = COVERED_G.has(hat);
  hairBackG(c, G, p, rx, covered);
  for (const s of [-1, 1]) E(c, HX + s * (rx - 1), HY + 3, 3.4, 4.2, p.skinD);
  E(c, HX, HY, rx, ry, G.skin);
  if (G.face === "square") P(c, [[HX - rx + 0.4, HY + 2], [HX + rx - 0.4, HY + 2], [HX + rx - 1.6, HY + ry - 4], [HX + 6, HY + ry + 0.4], [HX - 6, HY + ry + 0.4], [HX - rx + 1.6, HY + ry - 4]], G.skin);
  if (G.face === "heart") P(c, [[HX - 9, HY + 12], [HX + 9, HY + 12], [HX, HY + ry + 2.4]], G.skin);
  if (G.face === "broad") for (const s of [-1, 1]) E(c, HX + s * (rx - 4), HY + 3, 5.4, 6.4, G.skin);
  P(c, [[HX + rx - 7, HY - ry + 4], [HX + rx, HY - 2], [HX + rx - 2, HY + ry - 6], [HX + 6, HY + ry - 1]], p.skinD);
  if (G.face === "broad" || G.face === "round") E(c, HX - rx + 6, HY + 5, 2.6, 1.6, p.skinL);
  if (!covered) hairCapG(c, G, p, rx, ry);
  else if (!G.bald && hat !== 105) P(c, [[HX - rx + 1, HY - 8], [HX + rx - 1, HY - 8], [HX + 8, HY - ry + 8], [HX - 8, HY - ry + 8]], p.hair);
  if (!veiled) {
    browsG(c, G);
    const one = G.features & FEATURE_ONE_EYED ? -1 : 0;
    for (const s of [-1, 1]) {
      const x = HX + s * G.eyeSet;
      if (s === one) { E(c, x, HY + 0.6, 2.8, 3, "#1c1712"); L(c, [[HX + s * 12, HY - 3.5], [HX - s * 4, HY + 5]], "#1c1712", 1.3); }
      else eyeG(c, x, HY + 1, s, G, p);
    }
    noseG(c, G, p);
    const must = beardG(c, G, p, rx);
    mouthG(c, G, !!must && G.beard !== "goatee");
    if (must) must();
    ageLinesG(c, G, p, rx);
    if (G.features & FEATURE_SCARRED) L(c, [[HX + 8, HY - 9], [HX + 3, HY + 12]], shade(G.skin, 0.5), 1.1);
    if (G.features & FEATURE_TATTOOED) for (let i = 0; i < 3; i++) L(c, [[HX - rx + 2, HY - 7 + i * 4], [HX - rx + 6, HY - 5 + i * 4]], p.trim, 1);
  }
  if (G.hs === "braids" && !G.bald && hat !== 102 && hat !== 101) braidsG(c, p, rx);
  if (G.earring && hat !== 102 && hat !== 101 && hat !== 6 && hat !== 13) for (const s of [-1, 1]) E(c, HX + s * (rx - 0.5), HY + 9, 1.6, 1.6, p.trim);
  headwear(c, hat, p, occ, true);
  void K;
}

function bustArtGene(c: Ctx, Pn: Person, occ: Occasion, bare: boolean) {
  const { K, G } = Pn, p = palG(K, G, occ);
  const sy = 74, by = 100, halfTop = G.female ? 23 : 26, halfBot = G.female ? 42 : 46;
  R(c, HX - 8, HY + HRY - 6, 16, 14, p.skinD);
  P(c, [[HX - halfTop, sy], [HX + halfTop, sy], [HX + halfBot, by], [HX - halfBot, by]], p.robe);
  P(c, [[HX + 6, sy + 1], [HX + halfTop, sy], [HX + halfBot, by], [HX + 14, by]], p.robeD);
  collar(c, K.neck ?? K.id, p, occ, sy, halfTop, halfBot, by);
  if (G.female && occ !== "everyday") { c.beginPath(); c.arc(HX, sy - 4, 9, 0.3, Math.PI - 0.3); c.strokeStyle = p.trimL; c.lineWidth = 1.6; c.stroke(); }
  headBlockG(c, K, G, p, occ, bare);
}

function figureArtGene(c: Ctx, Pn: Person, occ: Occasion, bare: boolean) {
  const { K, G } = Pn, p = palG(K, G, occ), f = G.features;
  const cx = 50, shY = 62, waist = 118, foot = 202, kind = K.garment;
  const wide = ["robe", "kaftan", "thobe", "boubou", "kimono", "deel"].includes(kind), legs = ["tunic", "poncho", "wrap"].includes(kind);
  const half = kind === "boubou" ? 38 : kind === "thobe" ? 27 : kind === "poncho" ? 36 : kind === "wrap" ? 26 : 34;
  const hem = kind === "tunic" ? 134 : kind === "poncho" ? 146 : kind === "wrap" ? 156 : 186;
  if (occ === "ceremonial") P(c, [[cx - 27, shY], [cx + 27, shY], [cx + 41, 190], [cx - 41, 190]], p.cloth2D);
  const legTop = legs ? hem - 6 : 170;
  for (const s of [-1, 1]) { R(c, cx + (s < 0 ? -16 : 3), legTop, 13, foot - 10 - legTop, legs ? p.cloth2 : p.skinD); R(c, cx + (s < 0 ? -19 : 3), foot - 11, 16, 11, "#33251a"); R(c, cx + (s < 0 ? -19 : 3), foot - 11, 16, 2.5, "#4a3823"); }
  const aw = wide ? 16 : 10, maimed = f & FEATURE_MAIMED_HAND ? 1 : 0;
  for (const s of [-1, 1]) {
    P(c, [[cx + s * 22, shY - 2], [cx + s * (22 + aw), shY + 10], [cx + s * (20 + aw), waist + 16], [cx + s * 19, waist + 10]], s < 0 ? p.robe : p.robeD);
    if (wide) L(c, [[cx + s * (21 + aw), waist + 4], [cx + s * 20, waist + 14]], p.trim, 2.2);
    const hx2 = cx + s * (19 + aw * 0.35), hy = waist + 22;
    if (s === maimed) { E(c, hx2, hy, 5.4, 5.4, "#d8cdb8"); L(c, [[hx2 - 4, hy - 3], [hx2 + 4, hy + 3]], "#a89a7c", 1.2); } else E(c, hx2, hy, 6, 6, G.skin);
  }
  P(c, [[cx - 24, shY], [cx + 24, shY], [cx + half, hem], [cx - half, hem]], p.robe);
  P(c, [[cx + 5, shY], [cx + 24, shY], [cx + half, hem], [cx + half * 0.32, hem]], p.robeD);
  for (const x of [-13, 0, 13]) L(c, [[cx + x, waist], [cx + x * 1.45, hem - 3]], p.robeD, 1.5);
  P(c, [[cx - half, hem - 5], [cx + half, hem - 5], [cx + half, hem], [cx - half, hem]], p.trim);
  if (kind === "toga") P(c, [[cx - 23, shY - 1], [cx + 7, waist + 18], [cx + 20, waist + 15], [cx - 9, shY - 2]], p.robeL);
  if (kind === "himation") P(c, [[cx + 23, shY - 1], [cx - 7, waist + 20], [cx - 20, waist + 17], [cx + 9, shY - 2]], p.robeL);
  if (kind === "sari") { P(c, [[cx - 22, shY - 1], [cx + 12, hem - 22], [cx + 25, hem - 19], [cx - 9, shY - 2]], p.cloth2); L(c, [[cx - 14, shY + 8], [cx + 23, hem - 22]], p.trim, 2.4); }
  if (kind === "kaftan" || kind === "deel") { R(c, cx - 4.5, shY, 9, hem - shY, p.cloth2); L(c, [[cx, shY], [cx, hem]], p.trim, 1.6); }
  if (kind === "crossrobe" || kind === "kimono") { P(c, [[cx - 17, shY - 2], [cx + 4, shY + 10], [cx + 6, hem], [cx - 20, hem]], p.robeL); L(c, [[cx + 17, shY - 2], [cx - 4, shY + 11]], p.trim, 2.6); }
  if (kind === "poncho") for (let i = 0; i < 5; i++) R(c, cx - 36 + i * 0.9, shY + 10 + i * 13, 72 - i * 1.8, 5, i % 2 ? p.trim : p.cloth2);
  if (kind === "boubou") for (let i = 0; i < 3; i++) { c.beginPath(); c.arc(cx, shY, 13 + i * 8, 0.25, Math.PI - 0.25); c.strokeStyle = i % 2 ? p.trim : p.cloth2; c.lineWidth = 2.8; c.stroke(); }
  if (kind === "wrap") { P(c, [[cx - 25, shY - 2], [cx + 9, shY + 8], [cx + 11, hem], [cx - 25, hem]], p.robeL); E(c, cx + 18, shY + 5, 5, 5, p.trim); }
  if (kind !== "boubou" && kind !== "wrap") P(c, [[cx - 25, waist - 5], [cx + 25, waist - 5], [cx + 26, waist + 8], [cx - 26, waist + 8]], p.cloth2);
  if (kind === "kimono" || kind === "deel") E(c, cx + 15, waist + 2, 4.4, 4.4, p.trim);
  if (kind === "thobe" || kind === "robe") L(c, [[cx, shY + 8], [cx, waist - 6]], p.trim, 2);
  R(c, cx - 8, 46, 16, 20, p.skinD);
  c.save(); const s = 17 / HRX; c.translate(50 - HX * s, 32 - HY * s); c.scale(s, s); headBlockG(c, K, G, p, occ, bare); c.restore();
}

// ── bridging the sim's `IndividualBrief` to a genome ──────────────────────
const PERSON_CACHE = new Map<string, Person>();
/** The structural subset of `IndividualBrief` a likeness needs. */
export interface LikenessSource {
  id?: number; name?: string; culture?: string; face_seed: number; female: boolean;
  age?: number; features: number; world_h?: number; places?: { y?: number }[];
}
/** A person's cached likeness. Skin follows the HOMELAND's latitude (their first
 *  recorded place, as a fraction of world height away from the equator); absent
 *  that, the culture's own skin tone stands in. */
export function individualPerson(p: LikenessSource): Person {
  const y0 = p.places?.[0]?.y;
  const lat = y0 != null && p.world_h ? Math.min(1, Math.abs(y0 / p.world_h - 0.5) * 2) : undefined;
  const key = `${p.id ?? ""}|${p.face_seed}|${p.culture}|${p.female ? 1 : 0}|${Math.floor((p.age ?? 40) / 4)}|${p.features}|${lat != null ? lat.toFixed(1) : "-"}`;
  let Pn = PERSON_CACHE.get(key);
  if (!Pn) {
    Pn = makePerson({ seed: p.face_seed || strSeed(`${p.name}${p.id}`), kit: kitForCulture(p.culture || "unknown"), female: p.female, age: p.age, lat, features: p.features });
    if (PERSON_CACHE.size > 600) PERSON_CACHE.clear();
    PERSON_CACHE.set(key, Pn);
  }
  return Pn;
}
