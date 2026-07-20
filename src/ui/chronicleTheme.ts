import type { CSSProperties } from "react";

/** Chronicle (campaign) design tokens — the ONE visual language for the campaign
 *  shell: the top HUD, the World Ledger rail and (progressively) the floating
 *  windows. Change a colour here, not inline. Mirrored in
 *  docs/mockups/_archive/chronicle-shell-redesign.html. */
export const T = {
  // Surfaces (darkest → most raised)
  bg: "#0b1420",
  panel: "#0d1521",
  raised: "#111b2a",
  card: "#0a1018",
  // Hairlines
  line: "#1e2e42",
  lineSoft: "#16202e",
  lineGold: "rgba(216,178,74,0.35)",
  // Ink (text) hierarchy
  ink: "#cfe2f6",
  inkMid: "#9fb4cc",
  inkDim: "#5a7390",
  inkFaint: "#46586e",
  // Accents
  accent: "#3a80c0",
  accentSoft: "#19324a",
  gold: "#d8b24a",     // the Chronicle's signature — dates, titles, seals
  goldDim: "#8a7434",
  parchment: "#e8d9b0",
  // Semantics
  good: "#4cae7a",
  warn: "#d9a441",
  bad: "#c0573a",
  badInk: "#e08080",
  goodInk: "#80c890",
} as const;

/** Serif display stack for campaign headings, dates and titles — the "chronicle"
 *  voice. Body text stays in the app's default sans. */
export const SERIF =
  "'Iowan Old Style','Palatino Linotype',Palatino,Georgia,'Times New Roman',serif";

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
export const RADIUS = { sm: 4, md: 6, lg: 8, pill: 999 } as const;

/** Elevation shadows. `panel` = a floating window, `card` = a raised inset. */
export const SHADOW = {
  panel: "0 8px 28px rgba(0,0,0,0.5)",
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
