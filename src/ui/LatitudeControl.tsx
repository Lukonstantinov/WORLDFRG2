import { useState, useEffect } from "react";
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

const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

/**
 * Controls the dynamic latitude framing. Every control has both a stepped slider
 * and a typed number box (commit on Enter / blur):
 *  - **Equator** (%) — moves the 0° line up/down; every band follows.
 *  - **Expansion** (%) — stretches the 0/30/60 bands apart; past 100% the poles
 *    fall off-canvas and are cropped.
 *  - **Line proportion** (×) — the spacing ratio between consecutive latitude
 *    lines (gap 30→60 ÷ gap 0→30): 1.0 = even, 2.4 = Mercator, tunable to match
 *    an already-distorted template. This one is display-only (lines move; the map
 *    raster and the simulation are untouched).
 *
 * Equator/Expansion update the store live so the overlay tracks, and persist to
 * world metadata so the next sim run generates against the new latitudes.
 */
export function LatitudeControl() {
  const meta = useWorldStore((s) => s.meta);
  const latConfig = useWorldStore((s) => s.latConfig);
  const setLatConfig = useWorldStore((s) => s.setLatConfig);
  const setMeta = useWorldStore((s) => s.setMeta);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);

  const equatorOffset = latConfig.equatorOffset;
  const latScale = latConfig.latScale;
  const lineRatio = latConfig.lineRatio;

  // Local text for the number boxes so typing is free; the displayed value
  // resyncs whenever the underlying frame changes (slider drag, reset, commit).
  const [eqStr, setEqStr] = useState("");
  const [expStr, setExpStr] = useState("");
  const [ratioStr, setRatioStr] = useState("");
  useEffect(() => { setEqStr(String(Math.round(equatorOffset * 100))); }, [equatorOffset]);
  useEffect(() => { setExpStr(String(Math.round(latScale * 100))); }, [latScale]);
  useEffect(() => { setRatioStr(lineRatio.toFixed(2)); }, [lineRatio]);

  if (!meta) return null;

  // Live overlay update (no IPC) — used while dragging. All three latitude
  // params now live in latConfig, since the line proportion is shared with the
  // simulation (persisted as lat_ratio).
  const apply = (eq: number, scale: number, ratio: number) => {
    setLatConfig(eq, scale, ratio);
    setOverlayVisible("latLines", true);
  };
  // Persist to the backend (slider release / typed value), reading the latest
  // live values so a release after a drag commits the final position.
  const commitLive = () => {
    const { equatorOffset: eq, latScale: scale, lineRatio: r } = useWorldStore.getState().latConfig;
    setLatitudeConfig(eq, scale, r).then(setMeta).catch(() => {});
  };

  const setEquatorPct = (pct: number, persist: boolean) => {
    const eq = clamp(pct, 0, 100) / 100;
    apply(eq, latScale, lineRatio);
    if (persist) setLatitudeConfig(eq, latScale, lineRatio).then(setMeta).catch(() => {});
  };
  const setExpansionPct = (pct: number, persist: boolean) => {
    const scale = clamp(pct, 25, 400) / 100;
    apply(equatorOffset, scale, lineRatio);
    if (persist) setLatitudeConfig(equatorOffset, scale, lineRatio).then(setMeta).catch(() => {});
  };
  const setRatio = (r: number, persist: boolean) => {
    const ratio = clamp(r, 0.5, 5);
    apply(equatorOffset, latScale, ratio);
    if (persist) setLatitudeConfig(equatorOffset, latScale, ratio).then(setMeta).catch(() => {});
  };

  const commitEq = () => {
    const v = parseFloat(eqStr);
    if (Number.isFinite(v)) setEquatorPct(v, true);
    else setEqStr(String(Math.round(equatorOffset * 100)));
  };
  const commitExp = () => {
    const v = parseFloat(expStr);
    if (Number.isFinite(v)) setExpansionPct(v, true);
    else setExpStr(String(Math.round(latScale * 100)));
  };
  const commitRatio = () => {
    const v = parseFloat(ratioStr);
    if (Number.isFinite(v)) setRatio(v, true);
    else setRatioStr(lineRatio.toFixed(2));
  };
  const onEnter = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") (e.target as HTMLInputElement).blur();
  };

  const reset = () => {
    apply(0.5, 1, 1);
    setLatitudeConfig(0.5, 1, 1).then(setMeta).catch(() => {});
  };

  const topLat = latAt(0, equatorOffset, latScale);
  const botLat = latAt(1, equatorOffset, latScale);

  return (
    <div style={{ padding: "6px 8px" }}>
      <div style={header}>Latitude Frame</div>

      <div style={row}>
        <span style={lbl}>Equator</span>
        <span style={val}>{Math.round(equatorOffset * 100)}%</span>
      </div>
      <div style={ctrlRow}>
        <input
          type="range" min={0} max={100} step={1} value={Math.round(equatorOffset * 100)}
          onChange={(e) => setEquatorPct(Number(e.target.value), false)}
          onPointerUp={commitLive} onKeyUp={commitLive}
          style={range}
        />
        <input
          value={eqStr} onChange={(e) => setEqStr(e.target.value)}
          onBlur={commitEq} onKeyDown={onEnter} inputMode="numeric" style={numInput}
        />
      </div>

      <div style={{ ...row, marginTop: 6 }}>
        <span style={lbl}>Expansion</span>
        <span style={val}>{Math.round(latScale * 100)}%</span>
      </div>
      <div style={ctrlRow}>
        <input
          type="range" min={25} max={400} step={5} value={Math.round(latScale * 100)}
          onChange={(e) => setExpansionPct(Number(e.target.value), false)}
          onPointerUp={commitLive} onKeyUp={commitLive}
          style={range}
        />
        <input
          value={expStr} onChange={(e) => setExpStr(e.target.value)}
          onBlur={commitExp} onKeyDown={onEnter} inputMode="numeric" style={numInput}
        />
      </div>

      <div style={{ ...row, marginTop: 8 }}>
        <span style={lbl}>Line proportion</span>
        <span style={val}>{lineRatio.toFixed(2)}×</span>
      </div>
      <div style={ctrlRow}>
        <input
          type="range" min={0.5} max={4} step={0.05} value={lineRatio}
          onChange={(e) => setRatio(Number(e.target.value), false)}
          onPointerUp={commitLive} onKeyUp={commitLive}
          style={range}
        />
        <input
          value={ratioStr} onChange={(e) => setRatioStr(e.target.value)}
          onBlur={commitRatio} onKeyDown={onEnter} inputMode="decimal" style={numInput}
        />
      </div>
      <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
        <button style={chip(Math.abs(lineRatio - 1) < 1e-3)} onClick={() => setRatio(1, true)}>Even 1.0×</button>
        <button style={chip(Math.abs(lineRatio - 2.4) < 1e-3)} onClick={() => setRatio(2.4, true)}>Mercator 2.4×</button>
      </div>
      <div style={hint}>
        Line proportion = gap(30→60) ÷ gap(0→30). The map image is not changed,
        but the SIMULATION now uses this too — re-run from Ocean &amp; Atmosphere
        so currents/climate land on the lines.
      </div>

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
const ctrlRow: React.CSSProperties = { display: "flex", gap: 6, alignItems: "center" };
const range: React.CSSProperties = { flex: 1, minWidth: 0, height: 4, cursor: "pointer" };
const numInput: React.CSSProperties = {
  width: 50, flexShrink: 0, padding: "2px 4px", fontSize: 11, fontFamily: "monospace",
  background: "#0d1219", color: "#c0d8f0", border: "1px solid #1e2a38",
  borderRadius: 4, textAlign: "right",
};
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
function chip(active: boolean): React.CSSProperties {
  return {
    flex: 1, padding: "3px 0", fontSize: 10, cursor: "pointer", borderRadius: 4,
    border: active ? "1px solid #3a7ac0" : "1px solid #1e2a38",
    background: active ? "#1a3a5a" : "#0d1219",
    color: active ? "#c0ddf0" : "#5a7090", fontWeight: active ? 600 : 400,
  };
}
