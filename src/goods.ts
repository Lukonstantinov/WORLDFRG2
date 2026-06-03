// Trade-good definitions. Order MUST match the backend GOOD_NAMES /
// TileData.goods ordering (sim/biological.rs).
export interface GoodDef {
  name: string;   // backend identifier (matches GOOD_NAMES)
  label: string;  // UI label
  emoji: string;  // glyph drawn inside the region on the map
  color: string;  // region tint / matrix accent
}

export const GOOD_DEFS: GoodDef[] = [
  { name: "silk", label: "Silk", emoji: "\u{1F9F5}", color: "#d97fb0" },
  { name: "wine", label: "Wine", emoji: "\u{1F377}", color: "#9b2d4f" },
  { name: "oliveoil", label: "Olive Oil", emoji: "\u{1FAD2}", color: "#8ea33a" },
  { name: "sugar", label: "Sugar", emoji: "\u{1F36C}", color: "#e8d8a0" },
  { name: "frankincense", label: "Frankincense", emoji: "\u{1FA94}", color: "#c79a4b" },
  { name: "stockfish", label: "Stockfish & Salt-cod", emoji: "\u{1F41F}", color: "#6fb0c8" },
  { name: "spices", label: "Spices", emoji: "\u{1F336}\u{FE0F}", color: "#d2622a" },
  { name: "tea", label: "Tea", emoji: "\u{1F375}", color: "#5fae6f" },
  { name: "coffee", label: "Coffee", emoji: "☕", color: "#7a4a2a" },
  { name: "furs", label: "Furs", emoji: "\u{1F98A}", color: "#a9763d" },
  { name: "timber", label: "Timber", emoji: "\u{1FAB5}", color: "#6b8f4e" },
  { name: "amber", label: "Amber", emoji: "\u{1F7E0}", color: "#e0962a" },
  { name: "salt", label: "Salt", emoji: "\u{1F9C2}", color: "#cfd6dc" },
  { name: "dyes", label: "Dyes", emoji: "\u{1F41A}", color: "#8a52c0" },
  { name: "incense", label: "Incense", emoji: "\u{1F4A8}", color: "#b0a0c0" },
  { name: "pearls", label: "Pearls", emoji: "\u{1F9AA}", color: "#d8e4ec" },
  { name: "whaling", label: "Whaling Grounds", emoji: "\u{1F40B}", color: "#5878a0" },
  { name: "wheat", label: "Wheat", emoji: "\u{1F33E}", color: "#d9b94a" },
  { name: "iron", label: "Iron / Ore", emoji: "⛏\u{FE0F}", color: "#9aa0a6" },
  { name: "cotton", label: "Cotton", emoji: "\u{1F9F6}", color: "#eef0e8" },
  { name: "gemstones", label: "Gemstones", emoji: "\u{1F48E}", color: "#56c8d8" },
];

/** Overlay-visibility key for a good's region toggle. */
export function goodOverlayKey(name: string): string {
  return `good_${name}`;
}
