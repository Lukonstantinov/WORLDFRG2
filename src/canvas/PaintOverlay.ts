/**
 * Native 2D canvas overlay for brush cursor preview.
 * Sits on top of the PixiJS canvas for zero-framework overhead on mousemove.
 */

const COLORS: Record<string, string> = {
  land: "rgba(120, 210, 80, 0.4)",
  sea: "rgba(40, 110, 220, 0.4)",
  elevation: "rgba(245, 166, 35, 0.4)",
  "elevation-erase": "rgba(80, 60, 30, 0.4)",
  shelf: "rgba(80, 180, 220, 0.4)",
  "shelf-erase": "rgba(40, 80, 100, 0.4)",
  volcano: "rgba(255, 80, 40, 0.5)",
};

const RING_COLORS: Record<string, string> = {
  land: "rgba(120, 210, 80, 0.8)",
  sea: "rgba(40, 110, 220, 0.8)",
  elevation: "rgba(245, 166, 35, 0.8)",
  "elevation-erase": "rgba(120, 80, 40, 0.8)",
  shelf: "rgba(80, 180, 220, 0.8)",
  "shelf-erase": "rgba(40, 80, 100, 0.8)",
  volcano: "rgba(255, 80, 40, 0.9)",
};

let overlayCanvas: HTMLCanvasElement | null = null;
let overlayCtx: CanvasRenderingContext2D | null = null;

export function createPaintOverlay(container: HTMLElement): HTMLCanvasElement {
  if (overlayCanvas) {
    overlayCanvas.remove();
  }
  overlayCanvas = document.createElement("canvas");
  overlayCanvas.style.cssText =
    "position:absolute;inset:0;width:100%;height:100%;pointer-events:none;z-index:10;";
  container.appendChild(overlayCanvas);

  overlayCtx = overlayCanvas.getContext("2d")!;

  const ro = new ResizeObserver(() => {
    if (!overlayCanvas || !container) return;
    overlayCanvas.width = container.clientWidth;
    overlayCanvas.height = container.clientHeight;
  });
  ro.observe(container);

  overlayCanvas.width = container.clientWidth;
  overlayCanvas.height = container.clientHeight;

  return overlayCanvas;
}

/** Draw cursor ring at screen position */
export function drawCursorRing(
  screenX: number,
  screenY: number,
  radiusPx: number,
  mode: string
) {
  if (!overlayCtx || !overlayCanvas) return;
  overlayCtx.clearRect(0, 0, overlayCanvas.width, overlayCanvas.height);

  overlayCtx.beginPath();
  overlayCtx.arc(screenX, screenY, radiusPx, 0, Math.PI * 2);
  overlayCtx.strokeStyle = RING_COLORS[mode] || RING_COLORS.land;
  overlayCtx.lineWidth = 1.5;
  overlayCtx.stroke();

  // Fill with faint color
  overlayCtx.fillStyle = COLORS[mode] || COLORS.land;
  overlayCtx.fill();
}

/** Draw a paint stamp (accumulated stroke preview) */
export function paintStamp(
  screenX: number,
  screenY: number,
  radiusPx: number,
  mode: string
) {
  if (!overlayCtx) return;
  overlayCtx.beginPath();
  overlayCtx.arc(screenX, screenY, radiusPx, 0, Math.PI * 2);
  overlayCtx.fillStyle = COLORS[mode] || COLORS.land;
  overlayCtx.fill();
}

export function clearPaintOverlay() {
  if (!overlayCtx || !overlayCanvas) return;
  overlayCtx.clearRect(0, 0, overlayCanvas.width, overlayCanvas.height);
}
