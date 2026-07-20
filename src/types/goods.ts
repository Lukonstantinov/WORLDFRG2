// Split from the former monolithic src/types.ts. Mirrors Rust serde structs.

// ── Editable trade-good specs (mirror sim/goods_spec.rs) ──
export type GoodDomain = "marine" | "coastal" | "continental" | "island";
export type GoodDistribution = "global" | "local" | "deposits" | "manufactured";

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
