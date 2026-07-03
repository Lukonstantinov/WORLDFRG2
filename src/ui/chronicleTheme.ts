import type { CSSProperties } from "react";

/** Chronicle (campaign) design tokens — the ONE visual language for the campaign
 *  shell: the top HUD, the World Ledger rail and (progressively) the floating
 *  windows. Change a colour here, not inline. Mirrored in
 *  docs/mockups/chronicle-shell-redesign.html. */
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
