// Procedural full-body CULTURE FIGURES — stylised "costume-plate" people in national
// dress, drawn as flat vector art. Appearance (skin, hair, build) and garments derive
// from the culture's KIT (matching sim/cultures.rs) and are RANDOMISED per individual
// from a seed, so a people shows real within-culture variety while staying recognisable.
// Deterministic, self-contained SVG — no raster art, no external assets.

export interface FigureOpts {
  kit: number;          // 0..11 costume kit (see sim/cultures.rs KITS); <0 → generic
  sex: "m" | "f";
  seed: number;         // per-individual seed (drives every random roll)
  kit2?: number;        // creole: second parent kit (blends palette/dress/headwear)
  variant?: number;     // extra roll offset (e.g. re-roll button) — folded into seed
  occasion?: Occasion;  // dress register: everyday (casual, trousers) vs ceremonial (robes, finery). Default "national".
}

/** Dress register. "national" = the default costume-plate; "everyday" = casual
 *  working dress (trousers/short tunic, plain, no cloak); "ceremonial" = finery
 *  (long robe, cloak, jewels, richer trim). */
export type Occasion = "national" | "everyday" | "ceremonial";

type Garment = "tunic" | "robe" | "toga" | "thobe" | "kaftan";
type Gown = "gown" | "stola" | "sari" | "robe";
type Head = "laurel" | "helm" | "fur" | "keffiyeh" | "turban" | "conical" | "furhat" | "feather" | "cap" | "band" | "veil" | "none";
type Motif = "plain" | "key" | "check" | "geo" | "embroider";

interface Kit {
  env: 0 | 1 | 2 | 3 | 4;   // 0 temperate 1 cold 2 arid 3 tropical 4 maritime
  hairs: string[];          // culture-plausible hair colours
  robes: string[];          // main-garment colour options
  trims: string[];          // trim/accent options
  cloth2: string[];         // secondary (sash / trousers / drape)
  gm: Garment; gf: Gown;
  heads: Head[];            // possible male headwear (includes "none" when optional)
  headsF: Head[];           // possible female headwear
  veil: boolean;            // women may wear a veil/headcloth
  beardable: boolean;
  motif: Motif;
  cloak: number;            // 0..1 chance of a cloak/mantle
}

// Ordered to match sim/cultures.rs KITS (0 Roman … 11 Turkic).
const KITS: Kit[] = [
  { env: 0, hairs: ["#33241a", "#241812", "#4a3320"], robes: ["#ece6d6", "#e4dcc8", "#f2eede"], trims: ["#9c2b2b", "#8a2420"], cloth2: ["#b8863b", "#a86a2e"], gm: "toga",   gf: "stola", heads: ["laurel", "none", "band"], headsF: ["band", "none"], veil: false, beardable: false, motif: "key",       cloak: 0.25 }, // Roman
  { env: 4, hairs: ["#2c1e14", "#3a2818", "#1c140e"], robes: ["#eef1f4", "#e6ebf0", "#dfe6ec"], trims: ["#2f6fb0", "#2a5f96"], cloth2: ["#c8a33a", "#b58a2c"], gm: "robe",   gf: "stola", heads: ["band", "none", "laurel"], headsF: ["band", "none"], veil: false, beardable: true,  motif: "key",       cloak: 0.3  }, // Hellene
  { env: 4, hairs: ["#241a12", "#1a120c"], robes: ["#6a2f7a", "#5a2868", "#7a3a88"], trims: ["#d8b23a", "#c89a2c"], cloth2: ["#b8623a", "#a24f2e"], gm: "robe",   gf: "robe",  heads: ["conical", "cap"], headsF: ["veil", "none"] as Head[], veil: true,  beardable: true,  motif: "geo",       cloak: 0.2  }, // Punic
  { env: 2, hairs: ["#201712", "#2a1c12"], robes: ["#2f5aa0", "#284e8c", "#355fa8"], trims: ["#d8b23a", "#c89a2c"], cloth2: ["#8a3b3b", "#6f2f2f"], gm: "kaftan", gf: "robe",  heads: ["cap", "turban"], headsF: ["veil", "cap"] as Head[], veil: true,  beardable: true,  motif: "geo",       cloak: 0.25 }, // Persian
  { env: 1, hairs: ["#c08a3a", "#a8431e", "#6b4a2e", "#d8b06a"], robes: ["#6b4a2e", "#5a3f28", "#4a5a3a"], trims: ["#9aa4ad", "#8590a0"], cloth2: ["#4a5a3a", "#3a4a2e"], gm: "tunic",  gf: "gown",  heads: ["fur", "helm", "none"], headsF: ["none", "band"], veil: false, beardable: true, motif: "check",     cloak: 0.55 }, // Norse
  { env: 0, hairs: ["#a8431e", "#7a3a20", "#4a3320", "#c06a2e"], robes: ["#4a6a3a", "#3f5e32", "#5a6a3a"], trims: ["#b8863b", "#a86a2e"], cloth2: ["#8a3b2a", "#6f2f22"], gm: "tunic",  gf: "gown",  heads: ["none", "helm"], headsF: ["none", "band"], veil: false, beardable: true, motif: "check",     cloak: 0.5  }, // Celtic
  { env: 2, hairs: ["#1c140e", "#241812"], robes: ["#eae4d6", "#e2dccc", "#f0ebde"], trims: ["#7a5a2a", "#6a4e24"], cloth2: ["#b83a2a", "#9a3020"], gm: "thobe",  gf: "robe",  heads: ["keffiyeh", "turban"], headsF: ["veil"] as Head[], veil: true,  beardable: true,  motif: "embroider", cloak: 0.15 }, // Arab
  { env: 3, hairs: ["#14100c", "#1c130c"], robes: ["#c0392b", "#a83024", "#d8763a", "#b83a6a"], trims: ["#e0b13a", "#d8a02c"], cloth2: ["#d8763a", "#2ea0a0"], gm: "robe",   gf: "sari",  heads: ["turban", "none"], headsF: ["none", "veil"] as Head[], veil: true,  beardable: false, motif: "embroider", cloak: 0.15 }, // Indic
  { env: 0, hairs: ["#14100c", "#1a130c"], robes: ["#b83a3a", "#a83030", "#2e6a5a", "#3a4a8a"], trims: ["#2e7d5a", "#d8b23a"], cloth2: ["#d8b23a", "#c89a2c"], gm: "robe",   gf: "robe",  heads: ["cap", "none"], headsF: ["none", "band"], veil: false, beardable: false, motif: "plain",     cloak: 0.2  }, // Sinitic
  { env: 1, hairs: ["#7a5a2e", "#4a3320", "#c08a3a"], robes: ["#ece6d6", "#e4dcc8", "#dfe6ec"], trims: ["#b83a2a", "#2f6fb0"], cloth2: ["#2f6fb0", "#b83a2a"], gm: "tunic",  gf: "gown",  heads: ["furhat", "none"], headsF: ["veil", "band"] as Head[], veil: true,  beardable: true,  motif: "embroider", cloak: 0.4  }, // Slavic
  { env: 3, hairs: ["#14100c", "#1a130c"], robes: ["#d8763a", "#c26430", "#b83a2a"], trims: ["#2ea0a0", "#1c8a8a"], cloth2: ["#b83a2a", "#2ea0a0"], gm: "tunic",  gf: "gown",  heads: ["feather", "none"], headsF: ["feather", "none"], veil: false, beardable: false, motif: "geo",       cloak: 0.3  }, // Nahua
  { env: 2, hairs: ["#1c140e", "#241812"], robes: ["#3a6a8a", "#325e7a", "#8a3b3a"], trims: ["#d8b23a", "#c89a2c"], cloth2: ["#8a3b3a", "#6f2f2f"], gm: "kaftan", gf: "robe",  heads: ["furhat", "cap"], headsF: ["cap", "veil"] as Head[], veil: false, beardable: true,  motif: "geo",       cloak: 0.35 }, // Turkic
  // ── minor peoples (kits 12..17) ──
  { env: 2, hairs: ["#14100c", "#1c130c"], robes: ["#eae4d6", "#e6dcc4", "#dfe6ec"], trims: ["#c9a227", "#2ea0a0"], cloth2: ["#2a5fa0", "#c9a227"], gm: "robe",   gf: "robe",  heads: ["cap", "none"], headsF: ["band", "none"], veil: false, beardable: false, motif: "geo",       cloak: 0.15 }, // Nilotic
  { env: 2, hairs: ["#1c140e", "#241812"], robes: ["#3a6a8a", "#2a5fa0", "#8a3b3a"], trims: ["#e0b13a", "#c8ced6"], cloth2: ["#8a3b3a", "#c9a227"], gm: "thobe",  gf: "robe",  heads: ["keffiyeh", "turban"], headsF: ["veil"] as Head[], veil: true,  beardable: true,  motif: "geo",       cloak: 0.3  }, // Amazigh
  { env: 4, hairs: ["#14100c", "#1a130c"], robes: ["#2e6a5a", "#b83a3a", "#31405a", "#d8b23a"], trims: ["#1d2733", "#d8b23a"], cloth2: ["#b83a3a", "#1d2733"], gm: "robe",   gf: "robe",  heads: ["cap", "none"], headsF: ["none", "band"], veil: false, beardable: false, motif: "plain",     cloak: 0.2  }, // Yamato
  { env: 1, hairs: ["#1c140e", "#241812"], robes: ["#8a5a2e", "#6f4a24", "#3a6a8a"], trims: ["#c9a227", "#b83a2a"], cloth2: ["#6f4a24", "#8a3b3a"], gm: "kaftan", gf: "robe",  heads: ["furhat", "cap"], headsF: ["furhat", "cap"] as Head[], veil: false, beardable: true,  motif: "geo",       cloak: 0.5  }, // Mongol
  { env: 3, hairs: ["#14100c", "#1a130c"], robes: ["#b83a2a", "#c26430", "#2ea0a0", "#d8b23a"], trims: ["#e0b13a", "#2ea0a0"], cloth2: ["#2ea0a0", "#b83a2a"], gm: "tunic",  gf: "gown",  heads: ["feather", "cap"], headsF: ["cap", "none"], veil: false, beardable: false, motif: "geo",       cloak: 0.35 }, // Quechua
  { env: 3, hairs: ["#14100c", "#1a130c"], robes: ["#d8b23a", "#c26430", "#2e7d5a", "#3a6a8a"], trims: ["#b83a2a", "#1d2733"], cloth2: ["#b83a2a", "#2e7d5a"], gm: "robe",   gf: "robe",  heads: ["cap", "turban"], headsF: ["veil", "none"] as Head[], veil: true,  beardable: false, motif: "embroider", cloak: 0.2  }, // Mande
];

// Skin ramps per environment (light → deep). An individual picks one, weighted mid.
const SKIN: Record<number, string[]> = {
  0: ["#f0cba3", "#e8bd90", "#dcae7e", "#cf9e6a"],
  1: ["#f4d6b8", "#eecaa8", "#e6bd97", "#dcaf88"],
  2: ["#d8ac78", "#cf9e6a", "#c28f5c", "#b07d4c"],
  3: ["#b07f52", "#9c6a44", "#875838", "#6f472c"],
  4: ["#ecc39a", "#e0b488", "#d4a576", "#c69566"],
};

// ── colour helpers ──────────────────────────────────────────────────────────
function h2r(h: string): [number, number, number] { const n = parseInt(h.slice(1), 16); return [(n >> 16) & 255, (n >> 8) & 255, n & 255]; }
function r2h(r: number, g: number, b: number): string { const c = (v: number) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0"); return `#${c(r)}${c(g)}${c(b)}`; }
function shade(h: string, f: number): string { const [r, g, b] = h2r(h); return f >= 0 ? r2h(r + (255 - r) * f, g + (255 - g) * f, b + (255 - b) * f) : r2h(r * (1 + f), g * (1 + f), b * (1 + f)); }
function mix(a: string, b: string, t = 0.5): string { const x = h2r(a), y = h2r(b); return r2h(x[0] + (y[0] - x[0]) * t, x[1] + (y[1] - x[1]) * t, x[2] + (y[2] - x[2]) * t); }

// ── deterministic RNG (mulberry32) ──────────────────────────────────────────
function rngFrom(seed: number): () => number {
  let a = seed >>> 0;
  return () => { a |= 0; a = (a + 0x6D2B79F5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}
const pick = <T,>(r: () => number, arr: T[]): T => arr[Math.floor(r() * arr.length) % arr.length];

interface Look {
  skin: string; skinS: string; hair: string; robe: string; robeS: string; robeL: string;
  trim: string; cloth2: string; gm: Garment; gf: Gown; head: Head; motif: Motif;
  beard: boolean; cloak: boolean; longHair: boolean; build: number; jewel: boolean; staff: boolean;
  trousers: boolean;      // wears trousers/short tunic (vs a long draped garment)
  sash: boolean;          // waist sash drawn (dropped for plain everyday wear)
  // ── extra individuation (more variation, so a new head reads as a different
  //    person at a glance rather than a re-tinted twin) ──
  tilt: number;           // small whole-figure rotation, degrees (-4..4)
  mirror: boolean;        // mirrored stance — faces/leans the other way
  pin: boolean;           // a brooch/clasp at the chest, in a THIRD accent colour
  pinColor: string;
}

function rollLook(base: Kit, o: FigureOpts, r: () => number): Look {
  const female = o.sex === "f";
  const occ: Occasion = o.occasion ?? "national";
  const ramp = SKIN[base.env];
  // weight toward the middle of the ramp so extremes are rarer
  const si = Math.min(ramp.length - 1, Math.floor((r() * 0.6 + r() * 0.6) / 1.2 * ramp.length));
  const skin = shade(ramp[si], (r() - 0.5) * 0.10);
  const hair = pick(r, base.hairs);
  const robe0 = pick(r, base.robes);
  // Everyday clothes are plainer/earthier; ceremonial keeps the bright dyes.
  const robe = shade(robe0, (r() - 0.5) * 0.1 + (occ === "everyday" ? -0.1 : 0));
  const heads = female ? base.headsF : base.heads;
  const head: Head = heads.length ? pick(r, heads) : "none";
  // National dress keeps the kit's default garment. Everyday men wear trousers +
  // a short tunic almost everywhere (working dress); ceremonial puts everyone in a
  // long robe. `tunicLike` cultures (Norse/Celtic…) already wore trousers.
  const kitTunic = base.gm === "tunic" && !female;
  const trousers = occ === "everyday" ? !female
    : occ === "ceremonial" ? false
    : kitTunic;
  // Cloaks: rare in everyday, near-certain in ceremonial finery.
  const cloak = occ === "everyday" ? false
    : occ === "ceremonial" ? r() < 0.9
    : r() < base.cloak;
  const jewelBase = female ? 0.7 : 0.2;
  const jewel = occ === "everyday" ? r() < jewelBase * 0.3
    : occ === "ceremonial" ? r() < Math.min(1, jewelBase + 0.4)
    : r() < jewelBase;
  return {
    skin, skinS: shade(skin, -0.16), hair,
    robe, robeS: shade(robe, -0.16), robeL: shade(robe, 0.13),
    trim: pick(r, base.trims), cloth2: pick(r, base.cloth2),
    gm: base.gm, gf: base.gf, head, motif: occ === "everyday" ? "plain" : base.motif,
    beard: !female && base.beardable && r() < 0.55,
    cloak,
    longHair: female ? r() < 0.8 : r() < 0.25,
    build: 0.88 + r() * 0.26,
    jewel,
    staff: !female && r() < (occ === "ceremonial" ? 0.4 : occ === "everyday" ? 0.06 : 0.18),
    trousers,
    sash: occ !== "everyday" || r() < 0.4,
    tilt: (r() - 0.5) * 8,
    mirror: r() < 0.5,
    pin: r() < 0.3,
    pinColor: pick(r, base.trims.length > 1 ? base.trims : [...base.trims, ...base.cloth2]),
  };
}

// ── motif band (small repeated ornament along a hem) ────────────────────────
function motifBand(m: Motif, x0: number, x1: number, y: number, c: string): string {
  if (m === "plain") return "";
  const w = x1 - x0, out: string[] = [];
  if (m === "key") { for (let x = x0; x < x1 - 6; x += 9) out.push(`<path d="M${x} ${y} h6 v-5 h-4" fill="none" stroke="${c}" stroke-width="1.4"/>`); }
  else if (m === "check") { for (let x = x0; x < x1; x += 6) out.push(`<rect x="${x}" y="${y - 5}" width="3" height="5" fill="${c}" opacity="0.6"/>`); }
  else if (m === "geo") { for (let x = x0; x < x1 - 4; x += 8) out.push(`<path d="M${x} ${y} l4 -5 l4 5 z" fill="${c}" opacity="0.8"/>`); }
  else { for (let x = x0; x < x1 - 2; x += 7) out.push(`<circle cx="${x + 2}" cy="${y - 2.5}" r="1.5" fill="${c}"/>`); }
  void w; return out.join("");
}

// ── the figure (viewBox 0 0 120 340) ────────────────────────────────────────
function figure(L: Look, sex: "m" | "f"): string {
  const female = sex === "f";
  const { skin, skinS, hair, robe, robeS, robeL, trim, cloth2 } = L;
  const P: string[] = [];
  const cx = 60, sh = 22 * L.build;         // shoulder half-width
  const shY = 70, waistY = 156, hemY = 316, tunicHem = 212, footY = 322;
  const shL = cx - sh, shR = cx + sh;

  P.push(`<ellipse cx="60" cy="330" rx="30" ry="5.5" fill="rgba(0,0,0,0.15)"/>`);

  // cloak behind
  if (L.cloak) P.push(`<path d="M${shL - 2} ${shY} Q${cx} ${shY + 8} ${shR + 2} ${shY} L${shR + 10} ${hemY - 6} Q${cx} ${hemY + 8} ${shL - 10} ${hemY - 6} Z" fill="${shade(cloth2, -0.15)}" opacity="0.9"/>`);

  // ── lower body ──
  const tunicLike = L.trousers && !female;
  if (tunicLike) {
    // trousers + boots
    const lw = 12 * L.build;
    P.push(`<path d="M${cx - lw - 1} ${tunicHem} h${lw} v${footY - tunicHem} h-${lw} z" fill="${cloth2}"/>`);
    P.push(`<path d="M${cx + 1} ${tunicHem} h${lw} v${footY - tunicHem} h-${lw} z" fill="${cloth2}"/>`);
    P.push(`<path d="M${cx - lw - 3} ${footY - 4} h${lw + 3} v8 a2 2 0 0 1 -2 2 h-${lw + 1} z" fill="${shade(cloth2, -0.4)}"/>`);
    P.push(`<path d="M${cx} ${footY - 4} h${lw + 3} v10 h-${lw + 1} a2 2 0 0 1 -2 -2 z" fill="${shade(cloth2, -0.4)}"/>`);
  } else {
    // long garment — draped skirt widening to the hem
    const hemHalf = (female ? 34 : 30) * L.build;
    P.push(`<path d="M${cx - 20} ${waistY - 4} Q${cx} ${waistY + 6} ${cx + 20} ${waistY - 4} L${cx + hemHalf} ${hemY} Q${cx} ${hemY + 10} ${cx - hemHalf} ${hemY} Z" fill="${robe}"/>`);
    // vertical drape folds
    P.push(`<path d="M${cx - 8} ${waistY + 6} L${cx - 12} ${hemY - 2} M${cx + 8} ${waistY + 6} L${cx + 12} ${hemY - 2} M${cx} ${waistY + 4} L${cx} ${hemY}" stroke="${robeS}" stroke-width="1.3" opacity="0.5" fill="none"/>`);
    // hem trim + motif
    P.push(`<path d="M${cx - hemHalf} ${hemY} Q${cx} ${hemY + 10} ${cx + hemHalf} ${hemY}" fill="none" stroke="${trim}" stroke-width="4" opacity="0.85"/>`);
    P.push(motifBand(L.motif, cx - hemHalf + 4, cx + hemHalf - 4, hemY - 2, trim));
  }

  // ── torso garment ──
  P.push(`<path d="M${shL} ${shY} Q${cx} ${shY - 8} ${shR} ${shY} L${cx + 18} ${waistY} Q${cx} ${waistY + 8} ${cx - 18} ${waistY} Z" fill="${robe}"/>`);
  // garment-specific drape
  if (L.gf === "sari" && female) {
    P.push(`<path d="M${shL + 2} ${shY + 2} L${shR - 4} ${waistY - 10} L${shR - 10} ${waistY} L${shL} ${shY + 12} Z" fill="${cloth2}" opacity="0.92"/>`);
    P.push(`<rect x="${cx - 16}" y="${shY + 20}" width="32" height="14" rx="4" fill="${skin}" opacity="0.0"/>`);
  } else if (L.gm === "toga" && !female) {
    P.push(`<path d="M${shL + 3} ${shY + 2} L${shR - 6} ${waistY - 8} L${shR - 12} ${waistY + 2} L${shL} ${shY + 14} Z" fill="${shade(robe, 0.09)}" opacity="0.92"/>`);
    P.push(`<path d="M${cx - 20} ${shY + 4} q-6 40 4 ${waistY - shY}" fill="none" stroke="${trim}" stroke-width="3" opacity="0.7"/>`);
  } else if (L.gm === "kaftan" && !female) {
    // open front: two panels + a centre gap showing the under colour
    P.push(`<path d="M${cx} ${shY - 2} L${cx} ${waistY}" stroke="${shade(cloth2, -0.2)}" stroke-width="6" opacity="0.9"/>`);
    P.push(`<path d="M${cx} ${shY} L${cx} ${waistY}" stroke="${trim}" stroke-width="2" opacity="0.8"/>`);
  } else {
    P.push(`<path d="M${cx} ${shY - 2} L${cx} ${waistY}" stroke="${trim}" stroke-width="2.4" opacity="0.75"/>`);
  }
  // collar / shoulders
  P.push(`<path d="M${shL} ${shY} Q${cx} ${shY - 8} ${shR} ${shY}" fill="none" stroke="${trim}" stroke-width="2.6" opacity="0.7"/>`);
  // waist sash (dropped for plain everyday wear)
  if (L.sash) P.push(`<path d="M${cx - 19} ${waistY - 3} Q${cx} ${waistY + 5} ${cx + 19} ${waistY - 3} L${cx + 19} ${waistY + 7} Q${cx} ${waistY + 15} ${cx - 19} ${waistY + 7} Z" fill="${cloth2}"/>`);
  // a chest brooch/clasp in a third accent colour — cheap, high-signal individuation
  // so two heads of the same house read as different people, not re-tinted twins.
  if (L.pin) P.push(`<path d="M${cx - 4} ${shY + 18} l4 -4 l4 4 l-4 4 z" fill="${L.pinColor}" stroke="${shade(L.pinColor, -0.3)}" stroke-width="0.6"/>`);

  // ── arms (wide sleeves for robe/hanfu, narrow for tunic) ──
  const wide = (L.gm === "robe" || L.gm === "kaftan" || L.gm === "thobe");
  const armW = wide ? 12 : 8;
  const wristY = female ? 168 : 172;
  P.push(`<path d="M${shL + 2} ${shY + 2} Q${shL - armW} ${shY + 6} ${shL - armW + 2} ${shY + 40} L${shL - 2} ${wristY} l${armW - 1} 0 L${shL + 8} ${shY + 44} Q${cx - 14} ${shY + 8} ${shL + 4} ${shY + 4} Z" fill="${robeS}"/>`);
  P.push(`<path d="M${shR - 2} ${shY + 2} Q${shR + armW} ${shY + 6} ${shR + armW - 2} ${shY + 40} L${shR + 2} ${wristY} l-${armW - 1} 0 L${shR - 8} ${shY + 44} Q${cx + 14} ${shY + 8} ${shR - 4} ${shY + 4} Z" fill="${robeS}"/>`);
  P.push(`<circle cx="${shL - 1}" cy="${wristY + 3}" r="5" fill="${skin}"/>`);
  P.push(`<circle cx="${shR + 1}" cy="${wristY + 3}" r="5" fill="${skin}"/>`);
  if (L.staff) P.push(`<rect x="${shR + 3}" y="${shY}" width="2.5" height="${footY - shY}" rx="1" fill="${shade(cloth2, -0.3)}"/>`);

  // ── neck + head ──
  const hY = 42, hR = 16;
  P.push(`<rect x="${cx - 6}" y="${hY + hR - 4}" width="12" height="12" rx="3" fill="${skinS}"/>`);
  if (L.jewel) P.push(`<path d="M${cx - 8} ${hY + hR + 6} Q${cx} ${hY + hR + 12} ${cx + 8} ${hY + hR + 6}" fill="none" stroke="${trim}" stroke-width="1.6"/>`);
  // hair behind (long)
  if (L.longHair) P.push(`<path d="M${cx - hR} ${hY - 2} Q${cx - hR - 4} ${hY + 40} ${cx - 8} ${hY + 56} L${cx - 3} ${hY + 52} Q${cx - hR + 2} ${hY + 24} ${cx - hR + 3} ${hY} Z M${cx + hR} ${hY - 2} Q${cx + hR + 4} ${hY + 40} ${cx + 8} ${hY + 56} L${cx + 3} ${hY + 52} Q${cx + hR - 2} ${hY + 24} ${cx + hR - 3} ${hY} Z" fill="${hair}"/>`);
  // ears
  P.push(`<circle cx="${cx - hR + 1}" cy="${hY + 2}" r="2.4" fill="${skinS}"/><circle cx="${cx + hR - 1}" cy="${hY + 2}" r="2.4" fill="${skinS}"/>`);
  // head
  P.push(`<ellipse cx="${cx}" cy="${hY}" rx="${hR}" ry="${hR + 1}" fill="${skin}"/>`);
  // face
  P.push(`<circle cx="${cx - 5.5}" cy="${hY}" r="1.7" fill="#2a211c"/><circle cx="${cx + 5.5}" cy="${hY}" r="1.7" fill="#2a211c"/>`);
  P.push(`<path d="M${cx - 8} ${hY - 4} q3 -2 6 0 M${cx + 2} ${hY - 4} q3 -2 6 0" fill="none" stroke="${shade(hair, 0.05)}" stroke-width="1.1" stroke-linecap="round"/>`);
  P.push(`<path d="M${cx} ${hY + 1} l0 4" stroke="${shade(skin, -0.2)}" stroke-width="1" stroke-linecap="round"/>`);
  P.push(`<path d="M${cx - 3.5} ${hY + 9} q3.5 2.5 7 0" fill="none" stroke="${shade(skin, -0.3)}" stroke-width="1.3" stroke-linecap="round"/>`);
  if (L.beard) P.push(`<path d="M${cx - hR + 1} ${hY + 2} Q${cx - hR + 3} ${hY + 22} ${cx} ${hY + 26} Q${cx + hR - 3} ${hY + 22} ${cx + hR - 1} ${hY + 2} Q${cx + 8} ${hY + 16} ${cx} ${hY + 17} Q${cx - 8} ${hY + 16} ${cx - hR + 1} ${hY + 2} Z" fill="${hair}"/>`);
  // hair fringe on top
  P.push(`<path d="M${cx - hR - 1} ${hY - 3} Q${cx - hR + 2} ${hY - hR - 4} ${cx} ${hY - hR - 4} Q${cx + hR - 2} ${hY - hR - 4} ${cx + hR + 1} ${hY - 3} Q${cx + 6} ${hY - hR + 3} ${cx} ${hY - hR + 4} Q${cx - 6} ${hY - hR + 3} ${cx - hR - 1} ${hY - 3} Z" fill="${hair}"/>`);

  // ── headwear ──
  P.push(headwear(L.head, hY, hR, cx, { hair, robe, robeL, trim, cloth2 }));

  return `<g>${P.join("")}</g>`;
}

function headwear(h: Head, hY: number, hR: number, cx: number, c: { hair: string; robe: string; robeL: string; trim: string; cloth2: string }): string {
  const top = hY - hR;
  switch (h) {
    case "veil":
      return `<path d="M${cx - hR - 3} ${hY - 2} Q${cx} ${top - 10} ${cx + hR + 3} ${hY - 2} Q${cx + hR + 6} ${hY + 34} ${cx + 6} ${hY + 52} L${cx + 2} ${hY + 50} Q${cx + hR} ${hY + 20} ${cx + hR - 1} ${hY - 2} Q${cx} ${top - 2} ${cx - hR + 1} ${hY - 2} Q${cx - hR} ${hY + 20} ${cx - 2} ${hY + 50} L${cx - 6} ${hY + 52} Q${cx - hR - 6} ${hY + 34} ${cx - hR - 3} ${hY - 2} Z" fill="${c.robeL}" opacity="0.96"/>`;
    case "laurel":
      return `<path d="M${cx - hR + 1} ${top + 6} Q${cx - hR - 6} ${top} ${cx - 5} ${top - 4} M${cx + hR - 1} ${top + 6} Q${cx + hR + 6} ${top} ${cx + 5} ${top - 4}" fill="none" stroke="${c.trim}" stroke-width="2.6" stroke-linecap="round"/>`;
    case "band":
      return `<path d="M${cx - hR} ${top + 6} Q${cx} ${top + 1} ${cx + hR} ${top + 6}" fill="none" stroke="${c.trim}" stroke-width="3" stroke-linecap="round"/>`;
    case "helm":
      return `<path d="M${cx - hR - 1} ${hY + 1} Q${cx} ${top - 8} ${cx + hR + 1} ${hY + 1} L${cx + hR + 1} ${top + 3} Q${cx} ${top - 3} ${cx - hR - 1} ${top + 3} Z" fill="${c.trim}"/><rect x="${cx - 1.5}" y="${top - 16}" width="3" height="14" fill="${shade(c.trim, -0.2)}"/>`;
    case "fur":
      return `<path d="M${cx - hR - 3} ${hY - 1} Q${cx} ${top - 10} ${cx + hR + 3} ${hY - 1} Q${cx} ${top + 2} ${cx - hR - 3} ${hY - 1} Z" fill="${shade(c.robe, -0.15)}"/><path d="M${cx - hR - 3} ${hY - 1} Q${cx} ${top + 4} ${cx + hR + 3} ${hY - 1}" fill="none" stroke="${c.robeL}" stroke-width="4" stroke-linecap="round"/>`;
    case "keffiyeh":
      return `<path d="M${cx - hR - 4} ${hY} Q${cx} ${top - 10} ${cx + hR + 4} ${hY} L${cx + hR + 5} ${hY + 26} L${cx + hR} ${hY + 26} Q${cx + hR + 1} ${hY + 2} ${cx} ${top + 2} Q${cx - hR - 1} ${hY + 2} ${cx - hR} ${hY + 26} L${cx - hR - 5} ${hY + 26} Z" fill="${c.robeL}"/><path d="M${cx - hR - 3} ${hY - 1} Q${cx} ${top - 2} ${cx + hR + 3} ${hY - 1}" fill="none" stroke="${c.cloth2}" stroke-width="2.6"/>`;
    case "turban":
      return `<path d="M${cx - hR - 1} ${hY - 1} Q${cx} ${top - 8} ${cx + hR + 1} ${hY - 1} Q${cx} ${top + 3} ${cx - hR - 1} ${hY - 1} Z" fill="${c.robe}"/><path d="M${cx - hR} ${hY - 2} Q${cx} ${top - 1} ${cx + hR} ${hY - 2}" fill="none" stroke="${c.trim}" stroke-width="2.4"/><path d="M${cx - hR + 3} ${hY - 5} Q${cx} ${top + 2} ${cx + hR - 3} ${hY - 5}" fill="none" stroke="${shade(c.robe, -0.2)}" stroke-width="2"/>`;
    case "conical":
      return `<path d="M${cx - hR + 3} ${hY - 2} L${cx} ${top - 18} L${cx + hR - 3} ${hY - 2} Z" fill="${c.cloth2}"/><path d="M${cx - hR + 3} ${hY - 2} h${2 * (hR - 3)}" stroke="${c.trim}" stroke-width="2.4"/>`;
    case "furhat":
      return `<path d="M${cx - hR} ${top + 8} Q${cx} ${top - 10} ${cx + hR} ${top + 8} L${cx + hR} ${hY - 1} Q${cx} ${top + 2} ${cx - hR} ${hY - 1} Z" fill="${c.robe}"/><path d="M${cx - hR - 1} ${hY - 1} Q${cx} ${hY - 6} ${cx + hR + 1} ${hY - 1} L${cx + hR + 1} ${hY + 3} Q${cx} ${hY - 2} ${cx - hR - 1} ${hY + 3} Z" fill="${shade(c.robe, -0.25)}"/>`;
    case "feather":
      return `<g fill="none" stroke="${c.trim}" stroke-width="2.6" stroke-linecap="round"><path d="M${cx} ${top} L${cx} ${top - 20}"/><path d="M${cx - 6} ${top + 2} L${cx - 12} ${top - 16}"/><path d="M${cx + 6} ${top + 2} L${cx + 12} ${top - 16}"/><path d="M${cx - 11} ${top + 5} L${cx - 20} ${top - 8}"/><path d="M${cx + 11} ${top + 5} L${cx + 20} ${top - 8}"/></g><path d="M${cx - hR} ${top + 7} Q${cx} ${top - 1} ${cx + hR} ${top + 7}" fill="none" stroke="${c.cloth2}" stroke-width="4"/>`;
    case "cap":
      return `<path d="M${cx - hR + 1} ${top + 8} Q${cx} ${top - 6} ${cx + hR - 1} ${top + 8} Q${cx} ${top + 2} ${cx - hR + 1} ${top + 8} Z" fill="${c.cloth2}"/><rect x="${cx - 1.5}" y="${top - 4}" width="3" height="5" fill="${c.trim}"/>`;
    default:
      return "";
  }
}

/** Full SVG string for one culture figure (male or female) in national dress. */
export function cultureFigureSVG(o: FigureOpts): string {
  let base = o.kit >= 0 && o.kit < KITS.length ? KITS[o.kit] : KITS[0];
  if (o.kit2 != null && o.kit2 >= 0 && o.kit2 < KITS.length && o.kit2 !== o.kit) {
    const b = KITS[o.kit2]; // creole: blend palettes, take some of both dress + headwear
    base = {
      ...base,
      hairs: [...base.hairs, ...b.hairs],
      robes: base.robes.map((r, i) => mix(r, b.robes[i % b.robes.length], 0.5)),
      trims: [...base.trims, ...b.trims],
      cloth2: [...base.cloth2, ...b.cloth2],
      heads: o.sex === "f" ? base.heads : [...base.heads, ...b.heads],
      headsF: [...base.headsF, ...b.headsF],
      veil: base.veil || b.veil,
      env: base.env,
    };
  }
  const seed = (Math.imul((o.seed >>> 0) ^ 0x9e3779b9, 2654435761) ^ ((o.variant ?? 0) * 0x85ebca6b) ^ (o.sex === "f" ? 0x1337 : 0)) >>> 0;
  const L = rollLook(base, o, rngFrom(seed));
  // Pose variation: a small tilt + an occasional mirrored stance, around the
  // figure's own vertical axis — cheap (one wrapping transform), but it's what
  // makes a re-roll read as a different PERSON rather than the same pose re-tinted.
  const mirrorX = L.mirror ? -1 : 1;
  return `<svg viewBox="0 0 120 340" width="100%" height="100%" preserveAspectRatio="xMidYMax meet" role="img" xmlns="http://www.w3.org/2000/svg"><g transform="translate(60,170) scale(${mirrorX},1) rotate(${L.tilt.toFixed(2)}) translate(-60,-170)">${figure(L, o.sex)}</g></svg>`;
}

/** FNV-1a hash of a culture name → stable per-culture seed. */
export function cultureSeed(name: string): number {
  let h = 2166136261;
  for (let i = 0; i < name.length; i++) { h ^= name.charCodeAt(i); h = Math.imul(h, 16777619); }
  return h >>> 0;
}

export const CULTURE_KIT_COUNT = KITS.length;
