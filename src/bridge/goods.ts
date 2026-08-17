// Split from the former monolithic src/bridge/tauri.ts (invoke wrappers, one per Rust command).
import { invoke } from "@tauri-apps/api/core";
import type { GoodRegion, GoodSpec, GoodsPlacementReport } from "@types";

// ── Trade-good library (editable specs; per-world + global) ──
/** The shipped 30-good defaults (for "reset to default"). */
export async function defaultGoods(): Promise<GoodSpec[]> {
  return invoke("default_goods");
}

/** The current world's active good specs (per-world snapshot or defaults). */
export async function getGoodsSpec(): Promise<GoodSpec[]> {
  return invoke("get_goods_spec");
}

/** Snapshot a good-spec list into the current world (used before generation). */
export async function setGoodsSpec(specs: GoodSpec[]): Promise<void> {
  return invoke("set_goods_spec", { specs });
}

/** Live suitability heatmap for a good spec (Goods Editor preview). */
export async function previewGoodScore(spec: GoodSpec): Promise<{ width: number; height: number; data: number[]; land: number[] }> {
  return invoke("preview_good_score", { spec });
}

/** Lightweight 220×110 land/sea mask for minimaps — no heavy column loading. */
export async function previewLandGrid(): Promise<{ width: number; height: number; land: number[] }> {
  return invoke("preview_land_grid");
}

/** The global good library (editing template for new worlds). */
export async function getGoodsLibrary(): Promise<GoodSpec[]> {
  return invoke("get_goods_library");
}

/** Persist the global good library. */
export async function saveGoodsLibrary(specs: GoodSpec[]): Promise<void> {
  return invoke("save_goods_library", { specs });
}

/** What happened importing a `.txt` goods file — always populated, even when
 *  nothing was added, so the caller never has to guess at a silent failure. */
export interface GoodsImportReport {
  added: string[];
  defaulted: [string, string][];
  rejected: [string, string][];
}

/** Import a `.txt` goods file (INI-ish `[id]` sections) into the global
 *  library. Add-only — an id already in the library is rejected, never
 *  overwritten (see docs/DEPOSITS_AND_MINING_PLAN.md slice 3, decision D8). */
export async function importGoodsTxt(path: string): Promise<GoodsImportReport> {
  return invoke("import_goods_txt", { path });
}

/** Cluster every trade-good belt into labelled regions. */
export async function computeGoodRegions(): Promise<GoodRegion[]> {
  return invoke("compute_good_regions");
}

/** The post-generation goods placement report (§8.20). Read-only; a world
 *  generated before the report existed returns an empty one rather than erroring. */
export async function getGoodsReport(): Promise<GoodsPlacementReport> {
  return invoke("get_goods_report");
}
