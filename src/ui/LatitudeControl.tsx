import { useWorldStore } from "../state/worldStore";
import { useUIStore } from "../state/uiStore";
import { setLatitudeConfig } from "../bridge/tauri";

/** Format a signed latitude (degrees) as e.g. "45°N", "0°", "30°S". */
function fmtLat(lat: number): string {
  const r = Math.round(lat);
  if (r === 0) return "0°";
  return `${Math.abs(r)}°${r > 0 ? "N" : "S"}`;
}

/** Latitude visible at a given normalized row, mirroring Rust `lat_from_y`. */
function latAt(yFrac: number, equatorOffset: number, latScale: number): number {
  const scale = latScale <= 1e-4 ? 1 : latScale;
  return Math.max(-90, Math.min(90, ((equatorOffset - yFrac) * 180) / scale));
}

/**
 * Controls the dynamic latitude framing:
 *  - **Equator** slider moves the 0° line up/down; every latitude band follows.
 *  - **Expansion** slider stretches the 0/30/60 bands apart (like dragging
 *    Fibonacci levels); past 100% the poles fall off-canvas and are cropped.
 *
 * Edits update the store optimistically so the Lat-Lines overlay tracks live,
 * and are persisted to world metadata so the next simulation run generates
 * against the new latitudes.
 */
export function LatitudeControl() {
  const meta = useWorldStore((s) => s.meta);
  const latConfig = useWorldStore((s) => s.latConfig);
  const setLatConfig = useWorldStore((s) => s.setLatConfig);
  const setMeta = useWorldStore((s) => s.setMeta);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);

  if (!meta) return null;

  const { equatorOffset, latScale } = latConfig;

  // Live, drag-time update: only the lat-line overlay tracks this, no IPC.
  const apply = (eq: number, scale: number) => {
    setLatConfig(eq, scale);             // live overlay update
    setOverlayVisible("latLines", true); // make the effect visible
  };

  // Commit to the backend once on release (so the next sim run generates against
  // the new latitudes). Reads the latest live values straight from the store.
  const commit = () => {
    const { equatorOffset: eq, latScale: scale } = useWorldStore.getState().latConfig;
    setLatitudeConfig(eq, scale).then(setMeta).catch(() => {});
  };

  const onEquator = (v: number) => apply(v / 100, latScale);
  const onScale = (v: number) => apply(equatorOffset, v / 100);
  const reset = () => { apply(0.5, 1); setLatitudeConfig(0.5, 1).then(setMeta).catch(() => {}); };

  const topLat = latAt(0, equatorOffset, latScale);
  const botLat = latAt(1, equatorOffset, latScale);

  return (
    <div style={{ padding: "6px 8px" }}>
      <div style={header}>Latitude Frame</div>

      <div style={row}>
        <span style={lbl}>Equator</span>
        <span style={val}>{Math.round(equatorOffset * 100)}%</span>
      </div>
      <input
        type="range" min={0} max={100} value={Math.round(equatorOffset * 100)}
        onChange={(e) => onEquator(Number(e.target.value))}
        onPointerUp={commit}
        onKeyUp={commit}
        style={range}
      />

      <div style={{ ...row, marginTop: 6 }}>
        <span style={lbl}>Expansion</span>
        <span style={val}>{Math.round(latScale * 100)}%</span>
      </div>
      <input
        type="range" min={25} max={400} value={Math.round(latScale * 100)}
        onChange={(e) => onScale(Number(e.target.value))}
        onPointerUp={commit}
        onKeyUp={commit}
        style={range}
      />

      <div style={readout}>
        Visible: {fmtLat(topLat)} &rarr; {fmtLat(botLat)}
      </div>
      <div style={hint}>
        Re-run Ocean &amp; Atmosphere onward to generate against these latitudes.
        Bands past the poles are cropped.
      </div>
      <button onClick={reset} style={resetBtn}>Reset to centered</button>
    </div>
  );
}

const header: React.CSSProperties = {
  fontSize: 10, color: "#4a6a8a", textTransform: "uppercase", letterSpacing: 1.2,
  marginBottom: 5, fontWeight: 600,
};
const row: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 2,
};
const lbl: React.CSSProperties = { fontSize: 10, color: "#5a7090" };
const val: React.CSSProperties = { fontSize: 10, color: "#8aa0c0", fontFamily: "monospace" };
const range: React.CSSProperties = { width: "100%", height: 4, cursor: "pointer" };
const readout: React.CSSProperties = {
  fontSize: 10, color: "#8aa0c0", fontFamily: "monospace", marginTop: 6,
};
const hint: React.CSSProperties = {
  fontSize: 9, color: "#405060", marginTop: 4, lineHeight: 1.35,
};
const resetBtn: React.CSSProperties = {
  marginTop: 6, width: "100%", padding: "3px 0", fontSize: 10, cursor: "pointer",
  background: "#151d28", color: "#7088a0", border: "1px solid #1e2a38", borderRadius: 4,
};
