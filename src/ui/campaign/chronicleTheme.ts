import type { CSSProperties } from "react";

/** Chronicle (campaign) design tokens — the ONE visual language for the campaign
 *  shell: the top HUD, the World Ledger rail and (progressively) the floating
 *  windows. Change a colour here, not inline. Mirrored in
 *  docs/mockups/_archive/chronicle-shell-redesign.html. */
/** The GAME FRAME palette (2026-10-16 redesign, owner: "game frame, ledger inside"):
 *  dark leather surfaces, bronze hairlines, parchment ink, gold titles. */
const FRAME = {
  // Surfaces (darkest → most raised)
  bg: "#14110d",
  panel: "#1b1712",
  raised: "#262017",
  card: "#16130f",
  // Hairlines
  line: "#5d4a2a",
  lineSoft: "#3a2f20",
  lineGold: "rgba(201,162,74,0.45)",
  // Ink (text) hierarchy
  ink: "#ecdcb5",
  inkMid: "#c9b48a",
  inkDim: "#8f7a55",
  inkFaint: "#6b5b40",
  // Accents
  accent: "#4f8fc0",
  accentSoft: "#1f3346",
  gold: "#e0b85c",     // the Chronicle's signature — dates, titles, seals
  goldDim: "#8a6d3b",
  parchment: "#e8d9b0",
  // Semantics
  good: "#4cae7a",
  warn: "#d9a441",
  bad: "#c0573a",
  badInk: "#e08080",
  goodInk: "#80c890",
};
type TokenKey = keyof typeof FRAME;

/** The LEDGER palette: a paper page inside the frame — dark ink on paper, headings
 *  in red ink (the rubrication of an account book). Applied by `<Ledger>` in kit.tsx. */
const LEDGER: Record<TokenKey, string> = {
  bg: "#ece3cf", panel: "#f5efe2", raised: "#efe6d2", card: "#f8f3e8",
  line: "#c9b993", lineSoft: "#ddd0b2", lineGold: "rgba(125,31,31,0.35)",
  ink: "#2b2419", inkMid: "#4d4030", inkDim: "#6b5b3d", inkFaint: "#8a7a5a",
  accent: "#2f6e93", accentSoft: "#dbe6ec",
  gold: "#7d1f1f", goldDim: "#9b6b4b", parchment: "#3e3528",
  good: "#2f7a4f", warn: "#a8741a", bad: "#a3262a", badInk: "#a3262a", goodInk: "#2f6b4f",
};

/** Chronicle design tokens — the ONE visual language for every window. Plain hex, so
 *  they work anywhere (SVG presentation attributes and canvas cannot read CSS vars). */
export const T: Readonly<Record<TokenKey, string>> = FRAME;

/** The same tokens as CSS variables (frame colour as fallback). The shared kit's HTML
 *  primitives use these, so a `<Ledger>` region repaints them by redefining the
 *  variables. Use only in `style`, never in an SVG attribute or on a canvas. */
export const TV = Object.fromEntries(
  (Object.keys(FRAME) as TokenKey[]).map((k) => [k, `var(--wf-${k}, ${FRAME[k]})`]),
) as Record<TokenKey, string>;

/** CSS custom properties that turn a region into a ledger page. */
export const ledgerVars = Object.fromEntries(
  (Object.keys(LEDGER) as TokenKey[]).map((k) => [`--wf-${k}`, LEDGER[k]]),
) as unknown as CSSProperties;

/** A token at an alpha, valid for var() tokens too. */
export const alpha = (token: string, a: number) =>
  `color-mix(in srgb, ${token} ${Math.round(a * 100)}%, transparent)`;

/** Serif display stack for campaign headings, dates and titles — the "chronicle"
 *  voice. Body text stays in the app's default sans. */
export const SERIF =
  "'Palatino Linotype','Book Antiqua',Palatino,'Iowan Old Style',Georgia,'Times New Roman',serif";

/** A campaign display heading (panel titles, the year). */
export const displayTitle: CSSProperties = {
  fontFamily: SERIF,
  color: T.gold,
  fontWeight: 700,
  letterSpacing: 0.4,
};

/** Small-caps section header inside a card. */
export const sectionHdr: CSSProperties = {
  color: T.inkDim,
  fontSize: 10,
  fontWeight: 600,
  textTransform: "uppercase",
  letterSpacing: 0.6,
  marginBottom: 4,
};

/** A rail/HUD card. */
export const cardStyle: CSSProperties = {
  background: T.card,
  border: `1px solid ${T.lineSoft}`,
  borderRadius: 6,
  padding: "7px 9px",
};

// ── Additive layout tokens ───────────────────────────────────────────────────
// Radius / shadow / spacing / font-size scales so the shared UI kit
// (`src/ui/kit.tsx`) and every panel speak ONE visual language instead of each
// hand-rolling its own hex + pixel values. Additive-only: existing `T`, `SERIF`
// and the style helpers above are unchanged, so nothing that already imports
// them breaks.

/** Corner radii. `pill` = fully rounded (badges/chips). */
export const RADIUS = { sm: 3, md: 4, lg: 5, pill: 999 } as const;

/** Elevation shadows. `panel` = a floating window, `card` = a raised inset. */
export const SHADOW = {
  panel: "inset 0 0 0 1px rgba(0,0,0,0.55), inset 0 1px 0 rgba(255,220,150,0.08), 0 10px 30px rgba(0,0,0,0.55)",
  card: "0 1px 0 rgba(255,255,255,0.02)",
} as const;

/** Spacing scale (px) — gaps, padding, margins. */
export const SPACE = { xs: 3, sm: 5, md: 8, lg: 12, xl: 16 } as const;

/** Font-size scale (px), from footnote to panel title. */
export const FZ = {
  micro: 8, tiny: 9, small: 10, body: 11, base: 12, head: 13, title: 15,
} as const;

/** Semantic → ink/fill token pairs, so a Badge/Meter picks a consistent colour
 *  by MEANING rather than a panel guessing a hex. */
export const TONE = {
  neutral: { ink: T.inkMid, fill: T.raised, line: T.line },
  good: { ink: T.goodInk, fill: "rgba(76,174,122,0.16)", line: "rgba(76,174,122,0.4)" },
  warn: { ink: "#e6c06a", fill: "rgba(217,164,65,0.16)", line: "rgba(217,164,65,0.4)" },
  bad: { ink: T.badInk, fill: "rgba(192,87,58,0.16)", line: "rgba(192,87,58,0.4)" },
  gold: { ink: T.gold, fill: "rgba(216,178,74,0.14)", line: T.lineGold },
  accent: { ink: "#8ec2ee", fill: T.accentSoft, line: "rgba(58,128,192,0.5)" },
} as const;

export type Tone = keyof typeof TONE;
