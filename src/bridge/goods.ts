// Split from the former monolithic src/bridge/tauri.ts (invoke wrappers, one per Rust command).
import { invoke } from "@tauri-apps/api/core";
import type { GoodRegion, GoodSpec } from "@types";

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

/** Cluster every trade-good belt into labelled regions. */
export async function computeGoodRegions(): Promise<GoodRegion[]> {
  return invoke("compute_good_regions");
}
