import { getTilesPacked } from "@bridge";

const TILE_SIZE = 128;
/** Max supertiles per `get_tiles` invoke — keeps each IPC payload bounded
 *  (~64 × 64KB ≈ 4MB of base64 RGBA) instead of one giant response. */
const CHUNK_SIZE = 64;
/** Highest LOD the backend supports (one image covers 2^4 = 16×16 base tiles). */
const MAX_LOD = 4;

interface CachedTile {
  img: HTMLCanvasElement;
  tx: number;
  ty: number;
  version: number;
  lastUsed: number;
}

/** LOD for a viewport scale: at scale 1 every world cell is ≥1px (full res);
 *  each halving of the scale steps one LOD up. Min zoom 0.05 → LOD 4. */
function lodForScale(scale: number): number {
  if (!(scale > 0)) return MAX_LOD;
  return Math.max(0, Math.min(MAX_LOD, Math.floor(Math.log2(1 / scale))));
}

/** Free a cached tile's canvas memory promptly (don't wait for GC). */
function releaseTile(tile: CachedTile) {
  tile.img.width = 0;
  tile.img.height = 0;
}

/** Create an offscreen canvas from RGBA pixel bytes. */
function canvasFromRGBA(rgba: Uint8ClampedArray<ArrayBuffer>, width: number, height: number): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d")!;
  ctx.putImageData(new ImageData(rgba, width, height), 0, 0);
  return canvas;
}

interface PackedTile {
  tx: number;
  ty: number;
  version: number;
  layerIdx: number;
  lod: number;
  sizePx: number;
  rgba: Uint8ClampedArray<ArrayBuffer>;
}

/** Parse a `get_tiles_packed` response. Layout (little-endian, mirrors the
 *  Rust `pack_tiles` — keep the two in sync): `[u32 count]` then per record
 *  `[i32 tx][i32 ty][i64 version][u8 layerIdx][u8 lod][u16 sizePx]
 *   [u32 byteLen][RGBA bytes]`. */
function parsePackedTiles(buf: ArrayBuffer): PackedTile[] {
  const view = new DataView(buf);
  let off = 0;
  const count = view.getUint32(off, true);
  off += 4;
  const out: PackedTile[] = [];
  for (let i = 0; i < count; i++) {
    const tx = view.getInt32(off, true);
    const ty = view.getInt32(off + 4, true);
    const version = Number(view.getBigInt64(off + 8, true));
    const layerIdx = view.getUint8(off + 16);
    const lod = view.getUint8(off + 17);
    const sizePx = view.getUint16(off + 18, true);
    const byteLen = view.getUint32(off + 20, true);
    off += 24;
    // Copy out of the IPC buffer so each tile's pixels are independently
    // collectable (a view would pin the whole response alive).
    const rgba = new Uint8ClampedArray(buf.slice(off, off + byteLen));
    off += byteLen;
    out.push({ tx, ty, version, layerIdx, lod, sizePx, rgba });
  }
  return out;
}

export class TileManager {
  private cache = new Map<string, CachedTile>();
  private maxCached = 2000;
  /** Keys of the chunk currently in flight (dedupes against re-requests). */
  private loading = new Set<string>();
  /** Layer of the most recently requested tiles — what `draw` blits. */
  private currentLayer = "";
  /** LOD of the most recently requested tiles — what `draw` blits. */
  private currentLod = 0;
  /** Supertile coords still to fetch for currentLayer/currentLod. REPLACED
   *  wholesale by each loadVisibleTiles call, so a stale request's unfetched
   *  remainder is abandoned the moment the viewport/layer/LOD changes. */
  private queue: [number, number][] = [];
  /** Single-flight pump draining `queue` one chunk at a time. */
  private pumping: Promise<void> | null = null;
  /** Bumped by clear() so in-flight responses for a wiped world are discarded. */
  private epoch = 0;
  /** Called after each chunk lands so the canvas can repaint progressively. */
  onTilesLoaded?: () => void;

  /** Cache key includes layer AND lod, so switching layers or zoom bands (and
   *  back) reuses the already-rendered tiles instead of refetching everything. */
  private key(layer: string, lod: number, tx: number, ty: number): string {
    return `${layer}|${lod}|${tx},${ty}`;
  }

  /** Draw the cached tiles of the current layer/LOD overlapping the visible
   *  range. `range` arrives in BASE-tile coords; at LOD L each cached image is
   *  a supertile covering S=2^L base tiles per side, blitted scaled up. */
  draw(
    ctx: CanvasRenderingContext2D,
    range: { txMin: number; txMax: number; tyMin: number; tyMax: number },
    layerOverride?: string,
  ) {
    ctx.imageSmoothingEnabled = false;
    const now = Date.now();
    // The COMPARE layer draws through the same cache and LOD as the base layer,
    // so a swipe costs one extra blit per visible tile and nothing else.
    const layer = layerOverride ?? this.currentLayer;
    const lod = this.currentLod;
    const S = 1 << lod;
    const size = TILE_SIZE * S;
    const stxMin = Math.floor(range.txMin / S);
    const stxMax = Math.floor(range.txMax / S);
    const styMin = Math.floor(range.tyMin / S);
    const styMax = Math.floor(range.tyMax / S);
    for (let sty = styMin; sty <= styMax; sty++) {
      for (let stx = stxMin; stx <= stxMax; stx++) {
        const tile = this.cache.get(this.key(layer, lod, stx, sty));
        if (!tile) continue;
        ctx.drawImage(tile.img, stx * size, sty * size, size, size);
        tile.lastUsed = now; // keep visible tiles hot for LRU eviction
      }
    }
  }

  /** Request the visible tiles from the Rust backend. The range arrives in
   *  BASE-tile coords; `scale` (viewport zoom) picks the LOD, the range is
   *  converted to supertile coords, and ONLY the missing supertiles are
   *  fetched, in chunks of at most CHUNK_SIZE per invoke. */
  async loadVisibleTiles(
    txMin: number, txMax: number,
    tyMin: number, tyMax: number,
    layer: string,
    gridWidth: number, gridHeight: number,
    scale = 1,
  ): Promise<void> {
    const maxTx = Math.ceil(gridWidth / TILE_SIZE) - 1;
    const maxTy = Math.ceil(gridHeight / TILE_SIZE) - 1;
    const cTxMin = Math.max(0, txMin);
    const cTyMin = Math.max(0, tyMin);
    const cTxMax = Math.min(maxTx, txMax);
    const cTyMax = Math.min(maxTy, tyMax);

    if (cTxMin > cTxMax || cTyMin > cTyMax) return;

    const lod = lodForScale(scale);
    const S = 1 << lod;

    // Record which layer/LOD `draw` should now show (cache keys include both).
    this.currentLayer = layer;
    this.currentLod = lod;

    // Convert the base-tile range to supertile coords.
    const sTxMin = Math.floor(cTxMin / S);
    const sTxMax = Math.floor(cTxMax / S);
    const sTyMin = Math.floor(cTyMin / S);
    const sTyMax = Math.floor(cTyMax / S);

    // Only the supertiles that are neither cached nor currently in flight.
    const needed: [number, number][] = [];
    const now = Date.now();
    for (let sty = sTyMin; sty <= sTyMax; sty++) {
      for (let stx = sTxMin; stx <= sTxMax; stx++) {
        const k = this.key(layer, lod, stx, sty);
        const cached = this.cache.get(k);
        if (cached) {
          cached.lastUsed = now;
        } else if (!this.loading.has(k)) {
          needed.push([stx, sty]);
        }
      }
    }

    // Replace (not append) the fetch queue: anything a previous request still
    // wanted but is no longer visible / at the current layer+LOD is dropped.
    this.queue = needed;

    if (this.queue.length === 0 && !this.pumping) return;
    if (!this.pumping) {
      this.pumping = this.pump().finally(() => { this.pumping = null; });
    }
    return this.pumping;
  }

  /** Drain the queue with at most one `get_tiles` invoke in flight. Each chunk
   *  is requested at the layer/LOD current at dequeue time (the queue is always
   *  rebuilt alongside currentLayer/currentLod), and cached as it arrives so
   *  tiles appear progressively. */
  private async pump(): Promise<void> {
    while (this.queue.length > 0) {
      const layer = this.currentLayer;
      const lod = this.currentLod;
      const epoch = this.epoch;
      // Each supertile makes the backend decompress S×S base tiles, so shrink
      // the chunk as the LOD grows to bound the per-invoke working set
      // (~256 base tiles ≈ a few hundred MB transient at most).
      const S = 1 << lod;
      const chunkSize = Math.max(1, Math.min(CHUNK_SIZE, Math.floor(256 / (S * S))));
      const chunk = this.queue.splice(0, chunkSize);
      const keys = chunk.map(([stx, sty]) => this.key(layer, lod, stx, sty));
      for (const k of keys) this.loading.add(k);

      let responses: PackedTile[] = [];
      try {
        responses = parsePackedTiles(await getTilesPacked(chunk, [layer], lod));
      } catch (err) {
        console.error("Failed to load tiles:", err);
      } finally {
        for (const k of keys) this.loading.delete(k);
      }

      // A clear() (world/data change) while this chunk was in flight makes the
      // rendered images stale — drop them; the follow-up refresh refetches.
      if (epoch !== this.epoch) continue;

      for (const resp of responses) {
        const k = this.key(layer, lod, resp.tx, resp.ty);
        const old = this.cache.get(k);
        if (old) {
          this.cache.delete(k);
          releaseTile(old);
        }
        const img = canvasFromRGBA(resp.rgba, resp.sizePx, resp.sizePx);
        this.cache.set(k, {
          img,
          tx: resp.tx,
          ty: resp.ty,
          version: resp.version,
          lastUsed: Date.now(),
        });
      }

      this.evict();
      if (responses.length > 0) this.onTilesLoaded?.();
    }
  }

  /** Invalidate specific BASE tiles (e.g., after painting) across ALL cached
   *  layers, since the underlying cell data — not just the active layer's
   *  render — changed. LOD>0 entries aggregate many base tiles, so every
   *  downsampled entry is dropped (cheap full sweep; they're few and small). */
  invalidate(tiles: [number, number][]) {
    const coords = new Set(tiles.map(([tx, ty]) => `${tx},${ty}`));
    for (const [k, tile] of this.cache) {
      const [, lodStr, coord] = k.split("|");
      if (lodStr !== "0" || coords.has(coord)) {
        this.cache.delete(k);
        releaseTile(tile);
      }
    }
  }

  /** Remove all cached tiles and abandon any in-flight fetches' results. */
  clear() {
    for (const tile of this.cache.values()) releaseTile(tile);
    this.cache.clear();
    this.loading.clear();
    this.queue = [];
    this.epoch++;
  }

  private evict() {
    if (this.cache.size <= this.maxCached) return;
    const entries = [...this.cache.entries()].sort(
      (a, b) => a[1].lastUsed - b[1].lastUsed
    );
    const toRemove = entries.slice(0, this.cache.size - this.maxCached);
    for (const [key, tile] of toRemove) {
      this.cache.delete(key);
      releaseTile(tile);
    }
  }
}
