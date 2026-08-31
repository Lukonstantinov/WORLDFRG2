// Split from the former monolithic src/types.ts. Mirrors Rust serde structs.

// ── Editable trade-good specs (mirror sim/goods_spec.rs) ──
export type GoodDomain = "marine" | "coastal" | "continental" | "island";
export type GoodDistribution = "global" | "local" | "deposits" | "manufactured";
/** CLAUDE.md §8.19 (goods localities, shipped) Slice 2 — which part of the shelf a marine good may
 *  occupy. "either" reproduces the old undifferentiated `sea_coastal` gate. */
export type MarineBand = "either" | "inshore" | "bank";

/** One recipe input of a Manufactured good: `qty` units of `good` per 1 output. */
export interface RecipeInput {
  good: string;
  qty: number;
}

export interface GoodEnvelope {
  climate: [number, number][];        // (koppen code, weight)
  temp?: [number, number] | null;     // bell center, width (°C)
  precip?: [number, number, number] | null;     // band lo, hi, edge (mm/yr)
  elevation?: [number, number, number] | null;  // band lo, hi, edge (0..1)
  abs_lat?: [number, number, number] | null;    // band lo, hi, edge (deg)
  fertility: number;
  coast_bonus: number;
  /** CLAUDE.md §8.19 (goods localities, shipped) Slice 1 (F6) — river placement factors, 0..1 weight
   *  each (0 = no effect, the default for every pre-existing custom good). */
  floodplain?: number;
  irrigation?: number;
  riverbank?: number;
  float_out?: number;
}

export interface GoodDepositSpec {
  min_elev: number;
  count_num: number;
  count_den: number;
  /** Per-mineral ore-province noise frequency (each metal lights up its own ranges). */
  province_scale?: number;
}

export interface GoodSpec {
  id: string;
  name: string;
  icon: string;
  color: string;
  enabled: boolean;
  domain: GoodDomain;
  distribution: GoodDistribution;
  rarity: number;
  desire: number;
  network_luxury: boolean;
  builtin: boolean;
  deposit?: GoodDepositSpec | null;
  scoring?: GoodEnvelope | null;
  /** CLAUDE.md §8.19 (goods localities, shipped) Slice 2 — defaults to "either" (unrestricted). */
  marine_band?: MarineBand;
  /** Need category — alternatives within a category substitute for each other. */
  category: string;
  /** Needs ladder tier: 0 basic, 1 comfort, 2 luxury. */
  need_tier: number;
  /** World-standard value per unit in grain-equivalent (wheat = 1). */
  base_value: number;
  /** Freight weight/volume multiplier (1 = silk-light; 3-4 = bulky staple). */
  bulk?: number;
  /** Extra freight per travel-day from spoilage (0 = durable). */
  perishable?: number;
  /** Recipe inputs for a Manufactured good (empty/absent = raw/extracted). */
  inputs?: RecipeInput[];
  /** Output-rate factor for manufacture (∝ population × this). */
  labor?: number;
  /** Demand cadence in days — how often a person consumes a unit (food ~7,
   *  comfort ~45, durables/luxuries ~180). Long = weak local pull → wholesale. */
  consumption_interval?: number;
}

/** One good's line in the post-generation placement report (§8.20). Mirrors
 *  `biological::GoodPlacementRow`. */
export interface GoodPlacementRow {
  id: string;
  name: string;
  icon: string;
  /** "global" | "local" | "endemic" | "deposits" | "manufactured". */
  distribution: string;
  category: string;
  cells: number;
  land_share: number;
  /** Independent homelands actually seeded (the spec's `origins` is the request). */
  origins: number;
  localities: number;
  /** Named (notable-grade) localities — the "subcategories" reading. */
  notable: string[];
  /** Mean belt value where present, 0..1. */
  mean_grade: number;
  /** "absent" | "fallback_seed" | "ubiquitous" | "single_cell"; empty when healthy. */
  flags: string[];
}

/** Mirrors `biological::GoodsPlacementReport`. */
export interface GoodsPlacementReport {
  rows: GoodPlacementRow[];
  enabled: number;
  placed: number;
  absent: number;
  flagged: number;
}
