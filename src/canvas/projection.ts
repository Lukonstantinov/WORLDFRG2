// Latitude-LINE placement only. The map raster is never touched — uploaded
// templates are already drawn in whatever projection the user chose, so we only
// decide where to draw the reference latitude lines (0/±30/±60/±90) on top.
//
// `ratio` is the spacing between consecutive line bands: gap(30→60) ÷ gap(0→30).
//   ratio = 1.0  → even spacing (equirectangular)
//   ratio = 2.4  → Mercator-like (the bands grow toward the poles)
//   any value    → match an already-distorted template by eye
// The first band (equator→30°) keeps the equirect size (set by the Expansion /
// `latScale` control), so changing the ratio fans the OUTER lines out/in while
// the equator and the 30° lines stay put.

const BAND_DEG = 30; // latitude lines sit every 30°

/** Canvas y (world units, 0..gridH) for a latitude line.
 *  `equatorOffset` / `latScale` mirror the Rust `lat_from_y` framing. */
export function latLineY(
  latDeg: number,
  gridH: number,
  equatorOffset = 0.5,
  latScale = 1,
  ratio = 1,
): number {
  const scale = latScale <= 1e-4 ? 1 : latScale;
  const equatorRow = equatorOffset * gridH;
  // Pixels for the first 30° band — identical to the linear spacing so ratio=1
  // reproduces the even map exactly.
  const band = (gridH * scale) / (180 / BAND_DEG);
  const n = Math.abs(latDeg) / BAND_DEG; // 0,1,2,3 for 0/30/60/90
  // Cumulative offset = band·(1 + r + r² + … + r^(n-1)) (geometric sum).
  const r = Math.max(0.01, ratio);
  const cum = Math.abs(r - 1) < 1e-6 ? band * n : band * (Math.pow(r, n) - 1) / (r - 1);
  return equatorRow - Math.sign(latDeg) * cum;
}
