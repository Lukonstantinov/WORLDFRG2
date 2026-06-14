// Trade-good definitions. Order MUST match the backend GOOD_NAMES /
// TileData.goods ordering (sim/biological.rs).
export interface GoodDef {
  name: string;   // backend identifier (matches GOOD_NAMES)
  label: string;  // UI label
  emoji: string;  // glyph drawn inside the region on the map
  color: string;  // region tint / matrix accent
}

export const GOOD_DEFS: GoodDef[] = [
  { name: "silk", label: "Silk", emoji: "\u{1F41B}", color: "#d97fb0" },
  { name: "wine", label: "Wine", emoji: "\u{1F377}", color: "#9b2d4f" },
  { name: "oliveoil", label: "Olive Oil", emoji: "\u{1FAD2}", color: "#8ea33a" },
  { name: "sugar", label: "Sugar Cane", emoji: "\u{1F36C}", color: "#e8d8a0" },
  { name: "frankincense", label: "Frankincense", emoji: "\u{1FA94}", color: "#c79a4b" },
  { name: "stockfish", label: "Stockfish & Salt-cod", emoji: "\u{1F41F}", color: "#6fb0c8" },
  { name: "spices", label: "Spices", emoji: "\u{1F336}\u{FE0F}", color: "#d2622a" },
  { name: "tea", label: "Tea", emoji: "\u{1F375}", color: "#5fae6f" },
  { name: "coffee", label: "Coffee", emoji: "☕", color: "#7a4a2a" },
  { name: "furs", label: "Furs", emoji: "\u{1F98A}", color: "#a9763d" },
  { name: "timber", label: "Timber", emoji: "\u{1FAB5}", color: "#6b8f4e" },
  { name: "amber", label: "Amber", emoji: "\u{1F7E0}", color: "#e0962a" },
  { name: "salt", label: "Rock Salt", emoji: "\u{1F9C2}", color: "#cfd6dc" },
  { name: "dyes", label: "Dyes", emoji: "\u{1F41A}", color: "#8a52c0" },
  { name: "incense", label: "Incense", emoji: "\u{1F4A8}", color: "#b0a0c0" },
  { name: "pearls", label: "Pearls", emoji: "\u{1F9AA}", color: "#d8e4ec" },
  { name: "whaling", label: "Whaling Grounds", emoji: "\u{1F40B}", color: "#5878a0" },
  { name: "wheat", label: "Grain", emoji: "\u{1F33E}", color: "#d9b94a" },
  { name: "iron", label: "Iron / Ore", emoji: "⛏\u{FE0F}", color: "#9aa0a6" },
  { name: "cotton", label: "Cotton", emoji: "\u{1F9F6}", color: "#eef0e8" },
  { name: "gemstones", label: "Gemstones", emoji: "\u{1F48E}", color: "#56c8d8" },
  { name: "hardwoods", label: "Tropical Hardwoods", emoji: "\u{1F333}", color: "#5b3a1e" },
  { name: "horses", label: "Horses", emoji: "\u{1F40E}", color: "#b5793a" },
  { name: "wool_fleece", label: "Fleece Wool", emoji: "\u{1F411}", color: "#e8e3d8" },
  { name: "wool_llama", label: "Highland Wool", emoji: "\u{1F999}", color: "#c8a06a" },
  { name: "ivory", label: "Ivory", emoji: "\u{1F418}", color: "#efe6d0" },
  { name: "cacao", label: "Cacao", emoji: "\u{1F36B}", color: "#6b4226" },
  { name: "copper", label: "Copper", emoji: "\u{1F7E4}", color: "#b06a3a" },
  { name: "tin", label: "Tin", emoji: "\u{26AA}", color: "#b8bcc0" },
  { name: "gold", label: "Gold", emoji: "\u{1F7E1}", color: "#d4af37" },
  { name: "cloves", label: "Cloves", emoji: "\u{1F33F}", color: "#7a3b1e" },
  { name: "pepper", label: "Pepper", emoji: "\u{26AB}", color: "#2f2f33" },
  { name: "paper", label: "Paper", emoji: "\u{1F4DC}", color: "#e8e0c8" },
  { name: "ceramics", label: "Ceramics", emoji: "\u{1F3FA}", color: "#5a86c8" },
  { name: "glassware", label: "Glassware", emoji: "\u{1FA9F}", color: "#9fd8d0" },
  { name: "tobacco", label: "Tobacco", emoji: "\u{1F6AC}", color: "#8a6a3a" },
  { name: "indigo", label: "Indigo", emoji: "\u{1F7E6}", color: "#3a4fb0" },
  { name: "dates", label: "Dates", emoji: "\u{1F33D}", color: "#c08a3a" },
  // ── Market builtins (mirror backend GOOD_NAMES 38..44) ──
  { name: "rice", label: "Rice", emoji: "\u{1F35A}", color: "#e6e2c8" },
  { name: "barley", label: "Barley & Rye", emoji: "\u{1F35E}", color: "#c8a85a" },
  { name: "millet", label: "Millet", emoji: "\u{1F963}", color: "#d8c070" },
  { name: "herring", label: "Herring", emoji: "\u{1F420}", color: "#7ab8d0" },
  { name: "honey", label: "Honey & Wax", emoji: "\u{1F36F}", color: "#e0a020" },
  { name: "hides", label: "Hides & Leather", emoji: "\u{1F404}", color: "#9a7a50" },
  { name: "beer", label: "Beer & Ale", emoji: "\u{1F37A}", color: "#d09030" },
  // ── Custom shipped goods (mirror sim/goods_spec.rs default_custom_goods) ──
  { name: "bay_salt", label: "Bay Salt", emoji: "\u{1F9C2}", color: "#e8e0d0" },
  { name: "citrus", label: "Citrus", emoji: "\u{1F34A}", color: "#f4a33a" },
  { name: "flax", label: "Flax / Linen", emoji: "\u{1F9F5}", color: "#cfe0e8" },
  { name: "coral", label: "Red Coral", emoji: "\u{1FAB8}", color: "#ff6f61" },
  { name: "cinnamon", label: "Cinnamon", emoji: "\u{1F33F}", color: "#a9603a" },
  { name: "saffron", label: "Saffron", emoji: "\u{1F33C}", color: "#f4c430" },
  { name: "tyrian_purple", label: "Tyrian Purple", emoji: "\u{1F7E3}", color: "#6a0dad" },
  { name: "ambergris", label: "Ambergris", emoji: "\u{1F40B}", color: "#cfc0a0" },
  { name: "jade", label: "Jade", emoji: "\u{1F7E2}", color: "#00a86b" },
  { name: "silver", label: "Silver", emoji: "\u{1FA99}", color: "#c8ccd6" },
  { name: "marble", label: "Marble", emoji: "\u{1F3DB}\u{FE0F}", color: "#e8e6e0" },
  { name: "lead", label: "Lead", emoji: "\u{1F529}", color: "#8a8e96" },
  // ── Clay (raw input) + manufactured chain goods (mirror goods_spec.rs) ──
  { name: "clay", label: "Clay", emoji: "\u{1F9F1}", color: "#b07a52" },
  { name: "cloth", label: "Woolen Cloth", emoji: "\u{1F9F6}", color: "#d8c8b0" },
  { name: "salted_herring", label: "Salted Herring", emoji: "\u{1F9C2}", color: "#88b8c0" },
  { name: "metalware", label: "Metalware & Arms", emoji: "\u{2694}\u{FE0F}", color: "#9099a8" },
  { name: "refined_sugar", label: "Refined Sugar", emoji: "\u{1F367}", color: "#f0e8d8" },
  { name: "citrus_liqueur", label: "Citrus Liqueur", emoji: "\u{1F378}", color: "#e8b24a" },
  { name: "linen", label: "Linen", emoji: "\u{1F9FA}", color: "#cfe0e8" },
  { name: "cotton_cloth", label: "Cotton Cloth", emoji: "\u{1F455}", color: "#eef0e8" },
  { name: "silk_brocade", label: "Fine Silk Brocade", emoji: "\u{1F9E3}", color: "#d96fb0" },
  { name: "carpets", label: "Carpets & Tapestry", emoji: "\u{1F7EB}", color: "#9b4f2f" },
  { name: "leather_goods", label: "Leather Goods", emoji: "\u{1F45E}", color: "#7a4a2a" },
  { name: "bronzeware", label: "Bronzeware", emoji: "\u{1F514}", color: "#b06a3a" },
  { name: "jewelry", label: "Fine Jewelry", emoji: "\u{1F48D}", color: "#d4af37" },
  { name: "brandy", label: "Brandy & Spirits", emoji: "\u{1F943}", color: "#a9603a" },
  { name: "mead", label: "Mead", emoji: "\u{1F36F}", color: "#e0a020" },
  { name: "perfume", label: "Perfume & Attar", emoji: "\u{1F9F4}", color: "#c79a4b" },
  { name: "soap", label: "Soap", emoji: "\u{1F9FC}", color: "#cfe0d8" },
  { name: "candles", label: "Candles & Wax", emoji: "\u{1F56F}\u{FE0F}", color: "#e8d8a0" },
  { name: "books", label: "Books & Manuscripts", emoji: "\u{1F4DA}", color: "#8a6a3a" },
  { name: "furniture", label: "Fine Furniture", emoji: "\u{1FA91}", color: "#6b4226" },
  { name: "ivory_carvings", label: "Ivory Carvings", emoji: "\u{265F}\u{FE0F}", color: "#efe6d0" },
  { name: "statuary", label: "Statuary", emoji: "\u{1F5FF}", color: "#e8e6e0" },
];

/** Overlay-visibility key for a good's region toggle. */
export function goodOverlayKey(name: string): string {
  return `good_${name}`;
}

/** A multi-type good's per-subtype display (label / colour / optional icon / a
 *  relative trade `value` 0..1 used for InfoPanel and as a prestige hint).
 *  Index = the backend subtype id returned in GoodRegion.subtypes. */
export interface SubtypeDef { label: string; color: string; icon?: string; value: number; }

/** Grain species — the "wheat" overlay is the world's staple-grain land, tinted
 *  by which cereal wins each cell's climate. Because each species owns a distinct
 *  temperature/moisture/altitude niche, they naturally fall on different
 *  continents (cold north → rye/barley; hot wet equator → rice; hot dry →
 *  millet/sorghum; temperate cores → wheat/maize/oats). `value` ≈ market price. */
export const GRAIN_SUBTYPES: SubtypeDef[] = [
  { label: "Wheat",   color: "#d9b94a", value: 0.85 }, // 0 temperate / Mediterranean (prime bread grain)
  { label: "Rice",    color: "#7fcf6a", value: 0.80 }, // 1 hot & wet paddy (high-yield staple)
  { label: "Maize",   color: "#e8c24a", value: 0.55 }, // 2 warm & moist
  { label: "Millet",  color: "#c98a3a", value: 0.40 }, // 3 hot semi-arid
  { label: "Barley",  color: "#bcae6a", value: 0.45 }, // 4 cool / high (also brewing)
  { label: "Rye",     color: "#9c8a5a", value: 0.45 }, // 5 cold humid-continental
  { label: "Oats",    color: "#cfc27a", value: 0.40 }, // 6 cool & wet oceanic (fodder)
  { label: "Sorghum", color: "#b86a3a", value: 0.38 }, // 7 very hot & arid
];

/** Paper sources (distinct icons so each is distinguishable). `value` rises with
 *  craft refinement — manufactured mill paper and parchment are dearer than reed
 *  papyrus. */
export const PAPER_SUBTYPES: SubtypeDef[] = [
  { label: "Papyrus",      color: "#c9c089", icon: "\u{1F4DC}", value: 0.40 }, // 0 reed / delta
  { label: "Bamboo paper", color: "#9bbf7a", icon: "\u{1F38B}", value: 0.55 }, // 1 subtropical pulp
  { label: "Mill paper",   color: "#e8e0c8", icon: "\u{1F4C4}", value: 0.75 }, // 2 manufactured at cities
  { label: "Parchment",    color: "#e2cfa6", icon: "\u{1F411}", value: 0.85 }, // 3 cold/high pastoral hide (prestige)
  { label: "Rice paper",   color: "#cfe6b0", icon: "\u{1F35A}", value: 0.50 }, // 4 hot wet east-monsoon fibre
];

/** Per-good subtype palette, keyed by good name. */
export function goodSubtypes(name: string): SubtypeDef[] | null {
  if (name === "wheat") return GRAIN_SUBTYPES;
  if (name === "paper") return PAPER_SUBTYPES;
  return null;
}

// ── Categories ──────────────────────────────────────────────────────────────
// Broad trade-good categories so the overlay list (and the goods browser) can
// GROUP similar goods together — e.g. rock salt + bay salt and all the metals
// sit side by side, the marine goods cluster, etc.
// Every good belongs to a THEMATIC category that holds both its raw and its
// manufactured members (e.g. Textiles = wool/cotton/flax raws AND cloth/linen/
// carpets). The review screen then splits each category by production type
// (Planted / Extracted / Manufactured). There is no "Other" or lump
// "Manufactures" bucket — each finished good sits with its raw family.
export const CATEGORY_ORDER = [
  "Staples", "Wine, Oil & Vine", "Cash Crops", "Spices & Aromatics",
  "Textiles & Animal", "Forestry & Craft", "Minerals & Metals", "Marine", "Other",
] as const;

const GOOD_CATEGORY: Record<string, string> = {
  // Staples — grains + the everyday sweeteners/larder
  wheat: "Staples", rice: "Staples", barley: "Staples", millet: "Staples",
  dates: "Staples", honey: "Staples",
  // Wine, oil & vine — pressed/fermented drinks & oils (raw + distilled/brewed)
  wine: "Wine, Oil & Vine", oliveoil: "Wine, Oil & Vine", citrus: "Wine, Oil & Vine",
  brandy: "Wine, Oil & Vine", mead: "Wine, Oil & Vine", beer: "Wine, Oil & Vine",
  citrus_liqueur: "Wine, Oil & Vine",
  // Cash crops — colonial/plantation crops + their first refinement
  sugar: "Cash Crops", tobacco: "Cash Crops", indigo: "Cash Crops",
  coffee: "Cash Crops", tea: "Cash Crops", cacao: "Cash Crops",
  refined_sugar: "Cash Crops",
  // Spices & aromatics — seasonings, resins, and the perfumes distilled from them
  spices: "Spices & Aromatics", cloves: "Spices & Aromatics", pepper: "Spices & Aromatics",
  cinnamon: "Spices & Aromatics", frankincense: "Spices & Aromatics", incense: "Spices & Aromatics",
  saffron: "Spices & Aromatics", perfume: "Spices & Aromatics",
  // Textiles & animal — fibres/hides/animals AND the cloth/leather woven from them
  silk: "Textiles & Animal", cotton: "Textiles & Animal", flax: "Textiles & Animal",
  wool_fleece: "Textiles & Animal", wool_llama: "Textiles & Animal", furs: "Textiles & Animal",
  hides: "Textiles & Animal", horses: "Textiles & Animal", ivory: "Textiles & Animal",
  cloth: "Textiles & Animal", linen: "Textiles & Animal", cotton_cloth: "Textiles & Animal",
  silk_brocade: "Textiles & Animal", carpets: "Textiles & Animal", leather_goods: "Textiles & Animal",
  // Forestry & craft — wood/clay/paper raws AND the workshop crafts made from them
  timber: "Forestry & Craft", hardwoods: "Forestry & Craft", paper: "Forestry & Craft",
  clay: "Forestry & Craft", ceramics: "Forestry & Craft", glassware: "Forestry & Craft",
  books: "Forestry & Craft", furniture: "Forestry & Craft", candles: "Forestry & Craft",
  soap: "Forestry & Craft", statuary: "Forestry & Craft", ivory_carvings: "Forestry & Craft",
  // Minerals & metals — both salts, ores, gems, stone AND the metalwork forged from them
  salt: "Minerals & Metals", bay_salt: "Minerals & Metals", iron: "Minerals & Metals",
  copper: "Minerals & Metals", tin: "Minerals & Metals", gold: "Minerals & Metals",
  gemstones: "Minerals & Metals", jade: "Minerals & Metals", silver: "Minerals & Metals",
  marble: "Minerals & Metals", lead: "Minerals & Metals",
  metalware: "Minerals & Metals", bronzeware: "Minerals & Metals", jewelry: "Minerals & Metals",
  // Marine — sea catch/harvest AND the salted fish preserved for transport
  stockfish: "Marine", herring: "Marine", salted_herring: "Marine", pearls: "Marine",
  whaling: "Marine", amber: "Marine", dyes: "Marine", tyrian_purple: "Marine",
  coral: "Marine", ambergris: "Marine",
};

export function goodCategory(name: string): string {
  // Every shipped good is mapped above; only user-added custom goods fall through
  // to "Other" (the trailing catch-all group).
  return GOOD_CATEGORY[name] ?? "Other";
}
