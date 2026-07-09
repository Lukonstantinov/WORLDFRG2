// Procedural full-body CULTURE FIGURES — a stylised "costume-plate" illustration of a
// people in national dress, one male + one female, drawn as flat vector art. Appearance
// (skin/hair) and garments derive from the culture's KIT (matching sim/cultures.rs) plus a
// per-culture hash jitter, so peoples read as visibly distinct human ethnicities. A creole
// blends two parent kits. No raster art / no external assets — pure deterministic SVG.

export interface FigureOpts {
  kit: number;        // 0..11 costume kit (see sim/cultures.rs KITS); <0 → generic
  sex: "m" | "f";
  seed: number;       // hash of the culture name → per-culture jitter
  kit2?: number;      // creole: second parent kit (blends garments/headwear)
}

type Garment = "tunic" | "robe" | "toga" | "thobe" | "kaftan";
type Gown = "gown" | "stola" | "sari" | "robe";
type Head = "laurel" | "helm" | "fur" | "keffiyeh" | "turban" | "conical" | "furhat" | "feather" | "cap" | "band" | "none";

interface Kit {
  env: 0 | 1 | 2 | 3 | 4; // 0 temperate 1 cold 2 arid 3 tropical 4 maritime
  skin: string; hair: string; robe: string; trim: string; cloth2: string;
  gm: Garment; gf: Gown; head: Head; veil: boolean; beard: boolean;
}

// Ordered to match sim/cultures.rs KITS (0 Roman … 11 Turkic).
const KITS: Kit[] = [
  { env: 0, skin: "#e6b98f", hair: "#33241a", robe: "#ece6d6", trim: "#9c2b2b", cloth2: "#b8863b", gm: "toga",   gf: "stola", head: "laurel",  veil: false, beard: false }, // Roman
  { env: 4, skin: "#e3b287", hair: "#2c1e14", robe: "#eef1f4", trim: "#2f6fb0", cloth2: "#c8a33a", gm: "robe",   gf: "stola", head: "band",    veil: false, beard: false }, // Hellene
  { env: 4, skin: "#d8a877", hair: "#241a12", robe: "#6a2f7a", trim: "#d8b23a", cloth2: "#b8623a", gm: "robe",   gf: "robe",  head: "conical", veil: true,  beard: true  }, // Punic
  { env: 2, skin: "#d3a06e", hair: "#201712", robe: "#2f5aa0", trim: "#d8b23a", cloth2: "#8a3b3b", gm: "kaftan", gf: "robe",  head: "cap",     veil: false, beard: true  }, // Persian
  { env: 1, skin: "#eecaa8", hair: "#c08a3a", robe: "#6b4a2e", trim: "#9aa4ad", cloth2: "#4a5a3a", gm: "tunic",  gf: "gown",  head: "fur",     veil: false, beard: true  }, // Norse
  { env: 0, skin: "#e6bd97", hair: "#a8431e", robe: "#4a6a3a", trim: "#b8863b", cloth2: "#8a3b2a", gm: "tunic",  gf: "gown",  head: "none",    veil: false, beard: true  }, // Celtic
  { env: 2, skin: "#cf9a68", hair: "#1c140e", robe: "#eae4d6", trim: "#7a5a2a", cloth2: "#b83a2a", gm: "thobe",  gf: "robe",  head: "keffiyeh",veil: true,  beard: true  }, // Arab
  { env: 3, skin: "#a06a44", hair: "#14100c", robe: "#c0392b", trim: "#e0b13a", cloth2: "#d8763a", gm: "robe",   gf: "sari",  head: "turban",  veil: false, beard: false }, // Indic
  { env: 0, skin: "#e6c090", hair: "#14100c", robe: "#b83a3a", trim: "#2e7d5a", cloth2: "#d8b23a", gm: "robe",   gf: "robe",  head: "cap",     veil: false, beard: false }, // Sinitic
  { env: 1, skin: "#e8c19a", hair: "#7a5a2e", robe: "#ece6d6", trim: "#b83a2a", cloth2: "#2f6fb0", gm: "tunic",  gf: "gown",  head: "furhat",  veil: true,  beard: true  }, // Slavic
  { env: 3, skin: "#9c6a44", hair: "#14100c", robe: "#d8763a", trim: "#2ea0a0", cloth2: "#b83a2a", gm: "tunic",  gf: "gown",  head: "feather", veil: false, beard: false }, // Nahua
  { env: 2, skin: "#cf9e6a", hair: "#1c140e", robe: "#3a6a8a", trim: "#d8b23a", cloth2: "#8a3b3a", gm: "kaftan", gf: "robe",  head: "furhat",  veil: false, beard: true  }, // Turkic
];

// ── small colour helpers ────────────────────────────────────────────────────
function hexToRgb(h: string): [number, number, number] {
  const n = parseInt(h.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
function rgbToHex(r: number, g: number, b: number): string {
  const c = (v: number) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0");
  return `#${c(r)}${c(g)}${c(b)}`;
}
function shade(h: string, f: number): string { // f>0 lighten, f<0 darken
  const [r, g, b] = hexToRgb(h);
  return f >= 0 ? rgbToHex(r + (255 - r) * f, g + (255 - g) * f, b + (255 - b) * f)
                : rgbToHex(r * (1 + f), g * (1 + f), b * (1 + f));
}
function mix(a: string, b: string, t = 0.5): string {
  const x = hexToRgb(a), y = hexToRgb(b);
  return rgbToHex(x[0] + (y[0] - x[0]) * t, x[1] + (y[1] - x[1]) * t, x[2] + (y[2] - x[2]) * t);
}

function jitterKit(base: Kit, seed: number): Kit {
  // Small per-culture variation in skin + hair so two peoples of the same kit still differ.
  const s = ((seed >>> 3) % 1000) / 1000;
  const sk = shade(base.skin, (s - 0.5) * 0.14);
  const hr = shade(base.hair, (((seed >>> 11) % 1000) / 1000 - 0.5) * 0.2);
  return { ...base, skin: sk, hair: hr };
}

// ── the figure ──────────────────────────────────────────────────────────────
/** SVG markup (a `<g>` group) for one figure inside a 0 0 120 300 viewBox. */
function figureGroup(k: Kit, sex: "m" | "f", seed: number): string {
  const skin = k.skin, skinS = shade(skin, -0.16), hair = k.hair;
  const robe = k.robe, robeS = shade(robe, -0.16), robeL = shade(robe, 0.12);
  const trim = k.trim, cloth2 = k.cloth2;
  const female = sex === "f";
  const P: string[] = [];

  // ground shadow
  P.push(`<ellipse cx="60" cy="288" rx="34" ry="6" fill="rgba(0,0,0,0.16)"/>`);

  // ── lower body ──
  if (!female && k.gm === "tunic") {
    // trousers + boots
    P.push(`<path d="M46 168 h13 v104 h-13 z" fill="${cloth2}"/>`);
    P.push(`<path d="M61 168 h13 v104 h-13 z" fill="${cloth2}"/>`);
    P.push(`<path d="M44 268 h17 v10 a3 3 0 0 1 -3 3 h-11 z" fill="${shade(cloth2,-0.4)}"/>`);
    P.push(`<path d="M59 268 h17 v13 h-11 a3 3 0 0 1 -3 -3 z" fill="${shade(cloth2,-0.4)}"/>`);
  } else {
    // long robe/dress/gown/sari — a draped skirt widening to the hem
    P.push(`<path d="M42 150 Q60 158 78 150 L${female ? 96 : 92} 280 Q60 ${female ? 292 : 288} ${female ? 24 : 28} 280 Z" fill="${robe}"/>`);
    P.push(`<path d="M60 156 L60 280" stroke="${trim}" stroke-width="2.4" opacity="0.7"/>`);
    // hem band
    P.push(`<path d="M${female ? 24 : 28} 280 Q60 ${female ? 292 : 288} ${female ? 96 : 92} 280" fill="none" stroke="${trim}" stroke-width="4" opacity="0.85"/>`);
  }

  // ── torso garment ──
  const shL = 40, shR = 80;
  const waistY = 150;
  P.push(`<path d="M${shL} 92 Q60 84 ${shR} 92 L82 ${waistY} Q60 158 38 ${waistY} Z" fill="${robe}"/>`);
  // placket / drape
  if (k.gf === "sari" && female) {
    P.push(`<path d="M40 96 L80 140 L78 150 L40 108 Z" fill="${cloth2}" opacity="0.92"/>`);
  } else if (k.gm === "toga" && !female) {
    P.push(`<path d="M42 96 L78 140 L74 150 L40 106 Z" fill="${shade(robe,0.08)}" opacity="0.9"/>`);
  } else {
    P.push(`<path d="M60 90 L60 ${waistY}" stroke="${trim}" stroke-width="2.6" opacity="0.75"/>`);
  }
  // shoulders trim
  P.push(`<path d="M${shL} 92 Q60 84 ${shR} 92" fill="none" stroke="${trim}" stroke-width="3" opacity="0.7"/>`);
  // waist sash
  P.push(`<path d="M40 ${waistY - 4} Q60 ${waistY + 4} 80 ${waistY - 4} L80 ${waistY + 6} Q60 ${waistY + 14} 40 ${waistY + 6} Z" fill="${cloth2}"/>`);

  // ── arms ──
  P.push(`<path d="M40 94 Q30 96 30 120 L34 176 h9 L44 120 Q46 100 ${shL} 96 Z" fill="${robeS}"/>`);
  P.push(`<path d="M80 94 Q90 96 90 120 L86 176 h-9 L76 120 Q74 100 ${shR} 96 Z" fill="${robeS}"/>`);
  P.push(`<circle cx="37" cy="180" r="6" fill="${skin}"/>`);
  P.push(`<circle cx="83" cy="180" r="6" fill="${skin}"/>`);

  // ── neck + head ──
  P.push(`<rect x="53" y="74" width="14" height="14" rx="3" fill="${skinS}"/>`);
  // hair behind
  if (female) P.push(`<path d="M34 54 Q34 96 44 116 L52 112 Q46 84 48 56 Z M86 54 Q86 96 76 116 L68 112 Q74 84 72 56 Z" fill="${hair}"/>`);
  P.push(`<circle cx="60" cy="56" r="22" fill="${skin}"/>`);
  // face
  P.push(`<circle cx="53" cy="56" r="2.1" fill="#2a211c"/>`);
  P.push(`<circle cx="67" cy="56" r="2.1" fill="#2a211c"/>`);
  P.push(`<path d="M55 66 Q60 69 65 66" fill="none" stroke="${shade(skin,-0.28)}" stroke-width="1.4" stroke-linecap="round"/>`);
  if (k.beard && !female) {
    P.push(`<path d="M42 58 Q46 84 60 88 Q74 84 78 58 Q72 74 60 76 Q48 74 42 58 Z" fill="${hair}"/>`);
  }
  // hair on top / fringe
  if (female) {
    P.push(`<path d="M38 52 Q42 30 60 30 Q78 30 82 52 Q74 42 60 42 Q46 42 38 52 Z" fill="${hair}"/>`);
  } else {
    P.push(`<path d="M39 52 Q42 32 60 32 Q78 32 81 52 Q72 44 60 44 Q48 44 39 52 Z" fill="${hair}"/>`);
  }

  // ── headwear ──
  P.push(headwear(k.head, female && k.veil, { skin, hair, robe: robe, robeL, trim, cloth2 }));

  // simple jewellery for women
  if (female) P.push(`<path d="M50 80 Q60 88 70 80" fill="none" stroke="${trim}" stroke-width="1.6" opacity="0.85"/>`);

  return `<g>${P.join("")}</g>`;
}

function headwear(h: Head, veil: boolean, c: { skin: string; hair: string; robe: string; robeL: string; trim: string; cloth2: string }): string {
  if (veil) {
    // draped veil/head-cloth framing the face
    return `<path d="M34 52 Q60 20 86 52 Q92 90 78 120 L72 118 Q84 84 78 54 Q60 34 42 54 Q36 84 48 118 L42 120 Q28 90 34 52 Z" fill="${c.robeL}" opacity="0.96"/>`;
  }
  switch (h) {
    case "laurel":
      return `<path d="M40 44 Q34 34 46 30 M80 44 Q86 34 74 30" fill="none" stroke="${c.trim}" stroke-width="3" stroke-linecap="round"/>`;
    case "band":
      return `<path d="M39 46 Q60 40 81 46" fill="none" stroke="${c.trim}" stroke-width="3.4" stroke-linecap="round"/>`;
    case "helm":
      return `<path d="M38 50 Q60 22 82 50 L82 46 Q60 30 38 46 Z M58 30 L62 30 L62 16 L58 16 Z" fill="${c.trim}"/><path d="M38 50 Q60 28 82 50" fill="none" stroke="${shade(c.trim,-0.3)}" stroke-width="2"/>`;
    case "fur":
      return `<path d="M34 48 Q60 26 86 48 Q86 40 60 32 Q34 40 34 48 Z" fill="${shade(c.robe,-0.1)}"/><path d="M34 48 Q60 40 86 48" fill="none" stroke="${c.robeL}" stroke-width="4" stroke-linecap="round"/>`;
    case "keffiyeh":
      return `<path d="M32 46 Q60 22 88 46 L90 74 L84 74 Q86 52 60 40 Q34 52 36 74 L30 74 Z" fill="${c.robeL}"/><path d="M32 46 Q60 32 88 46" fill="none" stroke="${c.cloth2}" stroke-width="3"/>`;
    case "turban":
      return `<path d="M36 48 Q60 24 84 48 Q86 40 60 34 Q34 40 36 48 Z" fill="${c.robe}"/><path d="M37 46 Q60 36 83 46" fill="none" stroke="${c.trim}" stroke-width="3"/><path d="M40 42 Q60 34 80 42" fill="none" stroke="${shade(c.robe,-0.2)}" stroke-width="2.4"/>`;
    case "conical":
      return `<path d="M46 44 L60 14 L74 44 Z" fill="${c.cloth2}"/><path d="M46 44 h28" stroke="${c.trim}" stroke-width="3"/>`;
    case "furhat":
      return `<path d="M42 40 Q60 20 78 40 L78 46 Q60 34 42 46 Z" fill="${shade(c.robe,-0.15)}"/><path d="M40 46 Q60 40 80 46 L80 50 Q60 44 40 50 Z" fill="${c.robeL}"/>`;
    case "feather":
      return `<g fill="none" stroke="${c.trim}" stroke-width="3" stroke-linecap="round">`
        + `<path d="M60 34 L60 8"/><path d="M52 36 L44 12"/><path d="M68 36 L76 12"/><path d="M46 40 L34 22"/><path d="M74 40 L86 22"/></g>`
        + `<path d="M40 44 Q60 34 80 44" fill="none" stroke="${c.cloth2}" stroke-width="4"/>`;
    case "cap":
      return `<path d="M42 46 Q60 30 78 46 Q60 40 42 46 Z" fill="${c.cloth2}"/><path d="M58 32 h4 v-6 h-4 z" fill="${c.trim}"/>`;
    default:
      return "";
  }
}

/** Full SVG string for one culture figure (male or female) in national dress. */
export function cultureFigureSVG(o: FigureOpts): string {
  let base = o.kit >= 0 && o.kit < KITS.length ? KITS[o.kit] : KITS[0];
  if (o.kit2 != null && o.kit2 >= 0 && o.kit2 < KITS.length && o.kit2 !== o.kit) {
    // Creole: blend a second parent kit's palette + take its headwear/second garment.
    const b = KITS[o.kit2];
    base = {
      ...base,
      robe: mix(base.robe, b.robe, 0.5),
      trim: mix(base.trim, b.trim, 0.5),
      cloth2: b.cloth2,
      skin: mix(base.skin, b.skin, 0.5),
      hair: mix(base.hair, b.hair, 0.5),
      head: o.sex === "f" ? base.head : b.head,
      veil: base.veil || b.veil,
    };
  }
  const k = jitterKit(base, o.seed);
  return `<svg viewBox="0 0 120 300" width="100%" height="100%" role="img" xmlns="http://www.w3.org/2000/svg">${figureGroup(k, o.sex, o.seed)}</svg>`;
}

/** splitmix-ish hash of a culture name → stable per-culture seed. */
export function cultureSeed(name: string): number {
  let h = 2166136261;
  for (let i = 0; i < name.length; i++) { h ^= name.charCodeAt(i); h = Math.imul(h, 16777619); }
  return h >>> 0;
}

export const CULTURE_KIT_COUNT = KITS.length;
