// Physical units for goods (owner decision 2026-10-17: metric standard, imperial option).
//
// The simulation counts goods in abstract units. They are anchored to one physical
// fact: a person eats about 0.6 kg of grain a day. The campaign reports how many
// grain units one person eats a day (`campaign_unit_scale`), which fixes kilograms
// per grain unit; every other good's unit weighs what its value buys at its value
// per weight (gold is worth ~20,000× grain by weight, silver ~2,000×).
import { create } from "zustand";
import type { GoodSpec } from "@types";

export type UnitSystem = "metric" | "imperial";

const LS_KEY = "wf2.units";
function initial(): UnitSystem {
  try { return localStorage.getItem(LS_KEY) === "imperial" ? "imperial" : "metric"; } catch { return "metric"; }
}
export const useUnits = create<{ system: UnitSystem; setSystem: (s: UnitSystem) => void }>((set) => ({
  system: initial(),
  setSystem: (system) => { try { localStorage.setItem(LS_KEY, system); } catch { /* per-viewer convenience only */ } set({ system }); },
}));

/** Grain eaten per person per day, kg (Allen's bare-bones basket ≈ 220 kg a year). */
export const GRAIN_KG_PER_PERSON_DAY = 0.6;

/** Value per weight against grain, by DNA value tier (Bulk … Treasure). */
const TIER_RATIO = [1, 4, 40, 400, 20000];
const RATIO_OVERRIDE: Record<string, number> = { gold: 20000, silver: 2000, pearls: 30000, diamond: 60000, jewelry: 20000 };
const LIQUID = new Set(["wine", "beer", "oliveoil", "mead", "brandy", "citrus_liqueur", "whaling"]);
const HEAD_KG: Record<string, number> = { horses: 400 };
const CLOTH_KG_PER_M = 0.4;
const CLOTH = new Set(["cloth", "linen", "cotton_cloth", "silk_brocade", "sailcloth", "carpets"]);

/** Kilograms in one abstract unit of `spec`, given kilograms per grain unit. */
export function kgPerUnit(spec: Pick<GoodSpec, "id" | "base_value" | "dna">, kgPerGrainUnit: number): number {
  const ratio = RATIO_OVERRIDE[spec.id] ?? TIER_RATIO[spec.dna?.tier ?? 1] ?? 4;
  return kgPerGrainUnit * (spec.base_value > 0 ? spec.base_value : 1) / ratio;
}

const num = (v: number) => v >= 100 ? Math.round(v).toLocaleString("en") : v >= 10 ? v.toFixed(1) : v.toFixed(2);

/** A quantity of good `id` (kilograms) in the chosen system, in the unit people used for it. */
export function fmtKg(kg: number, id: string, system: UnitSystem, treasure = false): string {
  if (HEAD_KG[id]) return `${num(kg / HEAD_KG[id])} head`;
  if (CLOTH.has(id)) return system === "metric" ? `${num(kg / CLOTH_KG_PER_M)} m` : `${num(kg / CLOTH_KG_PER_M * 1.0936)} yd`;
  if (LIQUID.has(id)) {
    const l = kg; // ≈ 1 kg per litre
    if (system === "imperial") return `${num(l / 4.546)} gal`;
    return l >= 100 ? `${num(l / 100)} hL` : `${num(l)} L`;
  }
  if (system === "imperial") {
    if (treasure) return `${num(kg * 32.1507)} oz t`;
    const lb = kg * 2.20462;
    return lb >= 2240 ? `${num(lb / 2240)} tons` : `${num(lb)} lb`;
  }
  return kg >= 1000 ? `${num(kg / 1000)} t` : kg >= 1 ? `${num(kg)} kg` : `${num(kg * 1000)} g`;
}

// ── The campaign anchor ───────────────────────────────────────────────────────
let kgPerGrainUnit = 0;
/** Feed `campaign_unit_scale` in; 0 (no campaign) turns quantities back to plain units. */
export function setUnitScale(grainUnitsPerPersonDay: number) {
  kgPerGrainUnit = grainUnitsPerPersonDay > 0 ? GRAIN_KG_PER_PERSON_DAY / grainUnitsPerPersonDay : 0;
}
/** `amount` abstract units of `good` as a physical quantity, or null before the anchor is known. */
export function physicalQty(good: string, amount: number, specs: GoodSpec[], system: UnitSystem): string | null {
  if (!kgPerGrainUnit) return null;
  const spec = specs.find((s) => s.id === good);
  if (!spec) return null;
  return fmtKg(amount * kgPerUnit(spec, kgPerGrainUnit), good, system, spec.dna?.role === "treasure");
}
