import { useEffect, useRef } from "react";
import type { CoarsePreview } from "@types";

/**
 * The TIER-2 preview: the real Ocean & Atmosphere → Köppen chain, run on a
 * downsampled copy of YOUR landmass, showing where the climates actually land
 * and what share of the world each one takes.
 *
 * This is the piece the zonal strip cannot do. The strip knows the desert BELT
 * is 12° wide; only this knows that your continent sits under it and comes out
 * 60% arid — because that depends on continentality, ocean currents and rain
 * shadows, none of which exist without land.
 */

const CLASSES = [
  { key: "tropicalPct", label: "Tropical", color: "#187a3c" },
  { key: "aridPct", label: "Arid", color: "#d6a84a" },
  { key: "temperatePct", label: "Temperate", color: "#7ebe5c" },
  { key: "continentalPct", label: "Continental", color: "#5c94b8" },
  { key: "polarPct", label: "Polar", color: "#e2e9f0" },
] as const;

// Ocean-current colours — the same warm/cold/neutral convention the main map's
// current overlay uses (OverlayManager WARM/COLD/NEUTRAL_CURRENT).
const CURRENT_COLOR: Record<number, string> = {
  0: "rgba(200, 214, 228, 0.55)", // neutral — equatorial / gyre limbs / drift
  1: "#ee5533", // warm boundary current
  2: "#3399ee", // cold boundary current / ACC
};

export function ClimateMixPreview({ preview }: { preview: CoarsePreview | null }) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const cv = canvasRef.current;
    if (!cv || !preview) return;
    cv.width = preview.width;
    cv.height = preview.height;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    // `rgba` arrives base64-encoded (a raw number array would be ~3 MB of JSON
    // for a 600×300 preview). Decode to bytes, then blit.
    const bin = atob(preview.rgba);
    const bytes = new Uint8ClampedArray(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    const img = ctx.createImageData(preview.width, preview.height);
    img.data.set(bytes);
    ctx.putImageData(img, 0, 0);

    // Draw the rough major ocean currents ON TOP of the Köppen thumbnail. These
    // are the currents this settings preview implies (traced from the same field
    // that shaped the class mix); the real generation stage refines them. Points
    // are already in thumbnail-pixel coords, so no transform is needed. Warm last
    // so the red boundary currents read over the cold/neutral limbs.
    const lines = preview.streamlines ?? [];
    if (lines.length) {
      ctx.lineJoin = "round";
      ctx.lineCap = "round";
      // Line width is in canvas (coarse) pixels; the canvas is CSS-scaled up, so
      // keep it sub-pixel-thin here to land ~1px on screen.
      ctx.lineWidth = 0.5;
      for (const type of [0, 2, 1]) {
        ctx.strokeStyle = CURRENT_COLOR[type] ?? CURRENT_COLOR[0];
        for (const line of lines) {
          if (line.ctype !== type || line.points.length < 2) continue;
          ctx.beginPath();
          for (let i = 0; i < line.points.length; i++) {
            const [x, y] = line.points[i];
            // A wrapped streamline can jump the antimeridian; break the path so
            // it doesn't draw a stripe straight across the map.
            if (i > 0) {
              const dx = Math.abs(x - line.points[i - 1][0]);
              if (dx > preview.width / 2) { ctx.moveTo(x, y); continue; }
            }
            if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
          }
          ctx.stroke();
        }
      }
    }
  }, [preview]);

  if (!preview) return null;

  return (
    <div>
      {/* The thumbnail. Rendered at its native coarse size then scaled to the
          panel width; pixelated keeps the class boundaries crisp rather than
          smearing five discrete colours into mud. */}
      <canvas
        ref={canvasRef}
        style={{
          width: "100%", height: "auto", display: "block",
          imageRendering: "pixelated",
          border: "1px solid #1e2e42", borderRadius: 3, background: "#0a1018",
        }}
      />

      {/* Ocean-current legend — only when the preview actually traced some. */}
      {(preview.streamlines?.length ?? 0) > 0 && (
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 4, fontSize: 9, color: "#6a819a" }}>
          <span>Currents (est.):</span>
          <LegendSwatch color="#ee5533" label="warm" />
          <LegendSwatch color="#3399ee" label="cold" />
          <LegendSwatch color="#9bb0c0" label="drift" />
        </div>
      )}

      {/* Stacked share bar */}
      <div style={{ display: "flex", height: 12, marginTop: 5, borderRadius: 2, overflow: "hidden" }}>
        {CLASSES.map((c) => {
          const pct = preview[c.key];
          if (pct < 0.5) return null;
          return (
            <div key={c.key} title={`${c.label} ${pct.toFixed(1)}%`}
              style={{ width: `${pct}%`, background: c.color }} />
          );
        })}
      </div>

      {/* Numeric mix */}
      <div style={{ marginTop: 4 }}>
        {CLASSES.map((c) => {
          const pct = preview[c.key];
          return (
            <div key={c.key} style={{ display: "flex", alignItems: "center", gap: 5, padding: "1px 0" }}>
              <div style={{ width: 9, height: 9, borderRadius: 2, background: c.color, flexShrink: 0,
                border: "1px solid #00000050" }} />
              <span style={{ color: "#7f95ad", fontSize: 10, flex: 1 }}>{c.label}</span>
              <span style={{ color: pct >= 1 ? "#a8c0d8" : "#4a6076", fontSize: 10, fontFamily: "monospace" }}>
                {pct.toFixed(1)}%
              </span>
            </div>
          );
        })}
      </div>

      <div style={{ color: "#405060", fontSize: 9, marginTop: 4, lineHeight: 1.4 }}>
        Land mean <b>{preview.meanTemp.toFixed(1)}°C</b>,{" "}
        <b>{Math.round(preview.meanPrecip)}</b> mm/yr over {preview.landCells.toLocaleString()} sampled
        cells. Run at {preview.width}×{preview.height} — the same physics as the
        real pass, at lower resolution, so small islands and narrow ranges are
        smoothed away. The overlaid currents are a rough estimate of the major
        ocean currents — the real generation stage refines them under the full
        rules.
      </div>
    </div>
  );
}

function LegendSwatch({ color, label }: { color: string; label: string }) {
  return (
    <span style={{ display: "flex", alignItems: "center", gap: 3 }}>
      <span style={{ width: 12, height: 2, background: color, display: "inline-block", borderRadius: 1 }} />
      {label}
    </span>
  );
}
