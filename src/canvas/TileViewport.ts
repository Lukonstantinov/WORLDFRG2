const TILE_SIZE = 128;

/** Time constant (seconds) of the wheel-zoom ease — how quickly the DISPLAYED
 *  scale/position catches up to the TARGET one after a wheel tick. Small
 *  enough to feel responsive, large enough to read as a glide rather than a
 *  jump cut — the EU4/CK3/Vic-style "smooth zoom" feel, as opposed to this
 *  viewport's old behaviour of setting scale straight to its new value every
 *  wheel event with no interpolation at all. */
const ZOOM_EASE_TAU = 0.10;
/** Below this delta the eased value snaps to its target instead of forever
 *  approaching it — an exponential ease never exactly reaches its target, and
 *  without a snap the last fractional pixel/scale delta would keep `update`
 *  reporting "still animating" forever, keeping the render loop and the tile
 *  fetcher spinning on a frame that is visually already settled. */
const ZOOM_EASE_SCALE_EPS = 1e-4;
const ZOOM_EASE_POS_EPS = 0.02;

export class TileViewport {
  private _x = 0;
  private _y = 0;
  // Independent X/Y scale so the world can be stretched/contracted to exactly
  // fill the canvas (equirectangular template fitted edge-to-edge, no letterbox
  // bars). Zoom/pan keep the two locked together so stretching only happens at
  // the initial fit.
  private _scaleX = 1;
  private _scaleY = 1;
  // Where a wheel-driven zoom is EASING toward. Every other mutator (drag pan,
  // `centerOn`, `fitWorld`) sets both the displayed value and its target in
  // the same call, so those stay instantaneous exactly as before — only the
  // wheel takes the eased path.
  private _targetX = 0;
  private _targetY = 0;
  private _targetScaleX = 1;
  private _targetScaleY = 1;
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
    // Anchored on the current TARGET, not the (still catching-up) displayed
    // value, so several wheel ticks in quick succession compound smoothly
    // toward one another instead of each one re-anchoring on a stale
    // mid-glide position.
    const newScaleX = clamp(this._targetScaleX * zoomFactor);
    const newScaleY = clamp(this._targetScaleY * zoomFactor);

    const rect = (e.target as HTMLElement).getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;

    this._targetX = cx - (cx - this._targetX) * (newScaleX / this._targetScaleX);
    this._targetY = cy - (cy - this._targetY) * (newScaleY / this._targetScaleY);
    this._targetScaleX = newScaleX;
    this._targetScaleY = newScaleY;
  }

  /** Ease the displayed x/y/scale a step toward the wheel-zoom target.
   *  `dt` is the frame's elapsed time in seconds. Returns whether another
   *  frame of easing is still needed, so the caller's render loop knows
   *  whether to keep re-rendering and re-fetching tiles or let the view sit
   *  idle once it has visually settled. A no-op (returns false immediately)
   *  whenever nothing set a target different from the current value — panning
   *  and programmatic jumps never trigger this path since they keep target
   *  and displayed value equal. */
  update(dt: number): boolean {
    const k = 1 - Math.exp(-dt / ZOOM_EASE_TAU);
    let moving = false;
    if (Math.abs(this._targetScaleX - this._scaleX) > ZOOM_EASE_SCALE_EPS
      || Math.abs(this._targetScaleY - this._scaleY) > ZOOM_EASE_SCALE_EPS) {
      this._scaleX += (this._targetScaleX - this._scaleX) * k;
      this._scaleY += (this._targetScaleY - this._scaleY) * k;
      moving = true;
    } else {
      this._scaleX = this._targetScaleX;
      this._scaleY = this._targetScaleY;
    }
    if (Math.abs(this._targetX - this._x) > ZOOM_EASE_POS_EPS
      || Math.abs(this._targetY - this._y) > ZOOM_EASE_POS_EPS) {
      this._x += (this._targetX - this._x) * k;
      this._y += (this._targetY - this._y) * k;
      moving = true;
    } else {
      this._x = this._targetX;
      this._y = this._targetY;
    }
    return moving;
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
    // A drag pan is instantaneous (1:1 with the pointer, as before) — keep the
    // wheel-zoom target following along so a wheel tick right after a drag
    // eases from where the view actually is, not from a stale pre-drag spot.
    this._targetX = this._x;
    this._targetY = this._y;
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
    // An explicit jump, not a wheel glide — land there instantly and drop any
    // wheel-zoom target that would otherwise pull the view back off it.
    this._targetScaleX = this._scaleX;
    this._targetScaleY = this._scaleY;
    this._targetX = this._x;
    this._targetY = this._y;
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
    } else {
      const s = Math.min(screenWidth / gridWidth, screenHeight / gridHeight);
      this._scaleX = s;
      this._scaleY = s;
      this._x = (screenWidth - gridWidth * s) / 2;
      this._y = (screenHeight - gridHeight * s) / 2;
    }
    // An explicit re-fit, not a wheel glide — land there instantly (see
    // `centerOn`'s own note).
    this._targetScaleX = this._scaleX;
    this._targetScaleY = this._scaleY;
    this._targetX = this._x;
    this._targetY = this._y;
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
