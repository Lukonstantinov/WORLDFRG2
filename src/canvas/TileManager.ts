import { getTileRange } from "../bridge/tauri";

const TILE_SIZE = 128;

interface CachedTile {
  img: HTMLCanvasElement;
  tx: number;
  ty: number;
  version: number;
  lastUsed: number;
}

/** Decode base64 string to Uint8Array */
function decodeBase64(b64: string): Uint8Array {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) {
    bytes[i] = bin.charCodeAt(i);
  }
  return bytes;
}

/** Create an offscreen canvas from base64-encoded RGBA pixel data */
function canvasFromBase64RGBA(b64: string, width: number, height: number): HTMLCanvasElement {
  const rgba = decodeBase64(b64);
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d")!;
  const clamped = new Uint8ClampedArray(rgba.length);
  clamped.set(rgba);
  const imageData = new ImageData(clamped, width, height);
  ctx.putImageData(imageData, 0, 0);
  return canvas;
}

export class TileManager {
  private cache = new Map<string, CachedTile>();
  private maxCached = 2000;
  private loading = new Set<string>();
  /** Layer of the most recently requested tiles — what `draw` blits. */
  private currentLayer = "";

  /** Cache key includes the layer, so switching layers (and back) reuses the
   *  already-rendered tiles instead of refetching + re-rendering everything. */
  private key(layer: string, tx: number, ty: number): string {
    return `${layer}|${tx},${ty}`;
  }

  /** Draw the cached tiles of the current layer overlapping the visible range.
   *  Only the on-screen tiles are blitted (the cache may hold up to `maxCached`),
   *  so pan/zoom cost scales with the viewport, not the whole cache. */
  draw(
    ctx: CanvasRenderingContext2D,
    range: { txMin: number; txMax: number; tyMin: number; tyMax: number },
  ) {
    ctx.imageSmoothingEnabled = false;
    const now = Date.now();
    const layer = this.currentLayer;
    for (let ty = range.tyMin; ty <= range.tyMax; ty++) {
      for (let tx = range.txMin; tx <= range.txMax; tx++) {
        const tile = this.cache.get(this.key(layer, tx, ty));
        if (!tile) continue;
        ctx.drawImage(tile.img, tx * TILE_SIZE, ty * TILE_SIZE);
        tile.lastUsed = now; // keep visible tiles hot for LRU eviction
      }
    }
  }

  /** Request tiles for the visible range from the Rust backend */
  async loadVisibleTiles(
    txMin: number, txMax: number,
    tyMin: number, tyMax: number,
    layer: string,
    gridWidth: number, gridHeight: number,
  ) {
    const maxTx = Math.ceil(gridWidth / TILE_SIZE) - 1;
    const maxTy = Math.ceil(gridHeight / TILE_SIZE) - 1;
    const cTxMin = Math.max(0, txMin);
    const cTyMin = Math.max(0, tyMin);
    const cTxMax = Math.min(maxTx, txMax);
    const cTyMax = Math.min(maxTy, tyMax);

    if (cTxMin > cTxMax || cTyMin > cTyMax) return;

    // Tiles are cached per layer; record which layer `draw` should now show.
    this.currentLayer = layer;

    const needed: [number, number][] = [];
    for (let ty = cTyMin; ty <= cTyMax; ty++) {
      for (let tx = cTxMin; tx <= cTxMax; tx++) {
        const k = this.key(layer, tx, ty);
        if (!this.cache.has(k) && !this.loading.has(k)) {
          needed.push([tx, ty]);
          this.loading.add(k);
        } else if (this.cache.has(k)) {
          this.cache.get(k)!.lastUsed = Date.now();
        }
      }
    }

    if (needed.length === 0) return;

    try {
      const responses = await getTileRange(
        cTxMin, cTxMax, cTyMin, cTyMax,
        [layer], 0
      );

      for (const resp of responses) {
        const k = this.key(layer, resp.tx, resp.ty);
        this.loading.delete(k);
        this.cache.delete(k);

        const img = canvasFromBase64RGBA(resp.rgba, TILE_SIZE, TILE_SIZE);
        this.cache.set(k, {
          img,
          tx: resp.tx,
          ty: resp.ty,
          version: resp.version,
          lastUsed: Date.now(),
        });
      }
    } catch (err) {
      for (const [tx, ty] of needed) {
        this.loading.delete(this.key(layer, tx, ty));
      }
      console.error("Failed to load tiles:", err);
    }

    this.evict();
  }

  /** Invalidate specific tiles (e.g., after painting) across ALL cached layers,
   *  since the underlying cell data — not just the active layer's render —
   *  changed. */
  invalidate(tiles: [number, number][]) {
    const suffixes = new Set(tiles.map(([tx, ty]) => `|${tx},${ty}`));
    for (const k of this.cache.keys()) {
      const bar = k.indexOf("|");
      if (bar >= 0 && suffixes.has(k.slice(bar))) this.cache.delete(k);
    }
  }

  /** Remove all cached tiles */
  clear() {
    this.cache.clear();
    this.loading.clear();
  }

  private evict() {
    if (this.cache.size <= this.maxCached) return;
    const entries = [...this.cache.entries()].sort(
      (a, b) => a[1].lastUsed - b[1].lastUsed
    );
    const toRemove = entries.slice(0, this.cache.size - this.maxCached);
    for (const [key] of toRemove) {
      this.cache.delete(key);
    }
  }
}
