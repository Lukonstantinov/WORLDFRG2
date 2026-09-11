/**
 * GENERATION_UX_REDESIGN_PLAN.md Slice 8 — the resolution-multiplier export
 * hook. `MapCanvas` owns the live TileManager/OverlayManager/viewport (they
 * are internal refs, not props), so rather than lifting all of that state out
 * to a parent component, MapCanvas registers ONE function here on mount —
 * the same module-singleton pattern `PixiApp.ts`'s `getApp()` already uses —
 * and the Export dialog calls it without needing to know anything about
 * MapCanvas's internals.
 *
 * The registered function re-runs the EXACT same scene-drawing code the
 * screen uses (`drawScene` in MapCanvas.tsx), onto a fresh offscreen canvas
 * sized `cssSize × devicePixelRatio × multiplier`, so a multiplier > 1 is
 * real additional pixel density over the current view — not a cropped
 * re-scale of an already-rasterized screenshot.
 */
export type ExportSnapshotFn = (multiplier: number) => string | null;
export type ExportSnapshotRawFn = (multiplier: number) => ImageData | null;

let fn: ExportSnapshotFn | null = null;
let rawFn: ExportSnapshotRawFn | null = null;

export function setExportSnapshotFn(f: ExportSnapshotFn | null) {
  fn = f;
}

export function exportMapSnapshot(multiplier: number): string | null {
  return fn ? fn(multiplier) : null;
}

/** GENERATION_UX_REDESIGN_PLAN.md Slice 8 (PDF atlas) — the RAW-pixel sibling
 *  of `exportMapSnapshot`: returns `ImageData` straight off the offscreen
 *  render instead of a PNG data URL, so the PDF writer can embed it as an
 *  uncompressed RGB stream without a decode round-trip. */
export function setExportSnapshotRawFn(f: ExportSnapshotRawFn | null) {
  rawFn = f;
}

export function exportMapSnapshotRaw(multiplier: number): ImageData | null {
  return rawFn ? rawFn(multiplier) : null;
}
