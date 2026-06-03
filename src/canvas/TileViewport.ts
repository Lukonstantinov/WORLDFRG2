const TILE_SIZE = 128;

export class TileViewport {
  private _x = 0;
  private _y = 0;
  // Independent X/Y scale so the world can be stretched/contracted to exactly
  // fill the canvas (equirectangular template fitted edge-to-edge, no letterbox
  // bars). Zoom/pan keep the two locked together so stretching only happens at
  // the initial fit.
  private _scaleX = 1;
  private _scaleY = 1;
  private isDragging = false;
  private lastPointer = { x: 0, y: 0 };

  get x() { return this._x; }
  get y() { return this._y; }
  get scaleX() { return this._scaleX; }
  get scaleY() { return this._scaleY; }
  /** Representative scalar scale (for line widths / brush sizing). */
  get scale() { return Math.sqrt(this._scaleX * this._scaleY); }

  onWheel(e: WheelEvent) {
    e.preventDefault();
    const zoomFactor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
    const clamp = (s: number) => Math.max(0.05, Math.min(30, s));
    const newScaleX = clamp(this._scaleX * zoomFactor);
    const newScaleY = clamp(this._scaleY * zoomFactor);

    const rect = (e.target as HTMLElement).getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;

    this._x = cx - (cx - this._x) * (newScaleX / this._scaleX);
    this._y = cy - (cy - this._y) * (newScaleY / this._scaleY);
    this._scaleX = newScaleX;
    this._scaleY = newScaleY;
  }

  startPan(x: number, y: number) {
    this.isDragging = true;
    this.lastPointer = { x, y };
  }

  updatePan(x: number, y: number) {
    if (!this.isDragging) return;
    const dx = x - this.lastPointer.x;
    const dy = y - this.lastPointer.y;
    this.lastPointer = { x, y };
    this._x += dx;
    this._y += dy;
  }

  endPan() {
    this.isDragging = false;
  }

  screenToWorld(sx: number, sy: number): { wx: number; wy: number } {
    const wx = Math.floor((sx - this._x) / this._scaleX);
    const wy = Math.floor((sy - this._y) / this._scaleY);
    return { wx, wy };
  }

  /** Center the view on a world cell, zooming in to at least `minScale`. */
  centerOn(wx: number, wy: number, screenWidth: number, screenHeight: number, minScale = 4) {
    if (this._scaleX < minScale) this._scaleX = minScale;
    if (this._scaleY < minScale) this._scaleY = minScale;
    this._x = screenWidth / 2 - (wx + 0.5) * this._scaleX;
    this._y = screenHeight / 2 - (wy + 0.5) * this._scaleY;
  }

  /**
   * Fit the world to the canvas.
   *  - `stretch` false (default): proportional fit (undistorted). Scales both
   *    axes by the limiting dimension and centers; small letterbox bars appear
   *    on the other axis when the pane aspect ≠ world aspect.
   *  - `stretch` true: scale X and Y independently so the map fills the pane
   *    edge-to-edge with no bars (distorts aspect).
   */
  fitWorld(
    gridWidth: number, gridHeight: number,
    screenWidth: number, screenHeight: number,
    stretch = false,
  ) {
    if (stretch) {
      this._scaleX = screenWidth / gridWidth;
      this._scaleY = screenHeight / gridHeight;
      this._x = 0;
      this._y = 0;
      return;
    }
    const s = Math.min(screenWidth / gridWidth, screenHeight / gridHeight);
    this._scaleX = s;
    this._scaleY = s;
    this._x = (screenWidth - gridWidth * s) / 2;
    this._y = (screenHeight - gridHeight * s) / 2;
  }

  getVisibleTileRange(screenWidth: number, screenHeight: number): {
    txMin: number; txMax: number; tyMin: number; tyMax: number;
  } {
    const topLeft = this.screenToWorld(0, 0);
    const bottomRight = this.screenToWorld(screenWidth, screenHeight);

    const txMin = Math.floor(topLeft.wx / TILE_SIZE) - 1;
    const tyMin = Math.floor(topLeft.wy / TILE_SIZE) - 1;
    const txMax = Math.floor(bottomRight.wx / TILE_SIZE) + 1;
    const tyMax = Math.floor(bottomRight.wy / TILE_SIZE) + 1;

    return { txMin, txMax, tyMin, tyMax };
  }
}
