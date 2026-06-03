/**
 * Canvas 2D renderer — replaces PixiJS to avoid WebGL init issues in Tauri WebView2.
 */

export interface MapApp {
  canvas: HTMLCanvasElement;
  ctx: CanvasRenderingContext2D;
  destroy: () => void;
}

let app: MapApp | null = null;

export function initPixiApp(container: HTMLElement): MapApp {
  if (app) {
    app.destroy();
    app = null;
  }

  const w = container.clientWidth || 800;
  const h = container.clientHeight || 600;
  const dpr = window.devicePixelRatio || 1;

  console.log(`[renderer] creating Canvas2D, container=${w}x${h}, dpr=${dpr}`);

  const canvas = document.createElement("canvas");
  canvas.width = w * dpr;
  canvas.height = h * dpr;
  canvas.style.width = "100%";
  canvas.style.height = "100%";
  canvas.style.display = "block";

  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Canvas 2D context not available");

  container.appendChild(canvas);

  // Draw immediate test pattern so we know the canvas works
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = "#0a1020";
  ctx.fillRect(0, 0, w, h);
  ctx.fillStyle = "#1a3050";
  ctx.font = "14px sans-serif";
  ctx.fillText("Canvas ready — create or open a world", 20, 30);

  app = {
    canvas,
    ctx,
    destroy: () => { canvas.remove(); },
  };

  console.log(`[renderer] canvas appended, ${canvas.width}x${canvas.height}`);
  return app;
}

export function getApp(): MapApp | null {
  return app;
}
