import { useState, useEffect } from "react";
import { useWorldStore } from "@state/worldStore";
import { useUIStore } from "@state/uiStore";
import { setLatitudeConfig } from "@bridge";

/**
 * Shared controls for the **World Characteristics** step (left WorkflowPanel).
 *
 * These were previously split across two columns: the planetary knobs existed
 * BOTH here and in a duplicate block inside the right-side Toolbar, and the
 * latitude framing existed only on the right — even though every one of them
 * feeds the simulation and must be set before generating. They now live in one
 * place, on the left, beside the generation steps that consume them.
 *
 * `PlanetSlider` is the shared control shape (stepped slider that commits on
 * release + preset chips + a hint); `LatitudeFrame` is the latitude-framing
 * block moved over from the Toolbar.
 */

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
 * Dynamic latitude framing. Every control has a stepped slider and a typed
 * number box (commit on Enter / blur):
 *  - **Equator** (%) — moves the 0° line up/down; every band follows.
 *  - **Expansion** (%) — stretches the 0/30/60 bands apart; past 100% the poles
 *    fall off-canvas and are cropped.
 *  - **Line proportion** (×) — spacing ratio between consecutive latitude lines
 *    (gap 30→60 ÷ gap 0→30): 1.0 = even, 2.4 = Mercator. Shared with the
 *    SIMULATION (persisted as `lat_ratio`), not merely a drawing option.
 *
 * All three persist to world metadata, so the next sim run generates against
 * the new latitudes.
 */
export function LatitudeFrame() {
  const meta = useWorldStore((s) => s.meta);
  const latConfig = useWorldStore((s) => s.latConfig);
  const setLatConfig = useWorldStore((s) => s.setLatConfig);
  const setMeta = useWorldStore((s) => s.setMeta);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);

  const equatorOffset = latConfig.equatorOffset;
  const latScale = latConfig.latScale;
  const lineRatio = latConfig.lineRatio;
  const obliquity = meta?.obliquity ?? 23.44;

  const [eqStr, setEqStr] = useState("");
  const [expStr, setExpStr] = useState("");
  const [ratioStr, setRatioStr] = useState("");
  useEffect(() => { setEqStr(String(Math.round(equatorOffset * 100))); }, [equatorOffset]);
  useEffect(() => { setExpStr(String(Math.round(latScale * 100))); }, [latScale]);
  useEffect(() => { setRatioStr(lineRatio.toFixed(2)); }, [lineRatio]);

  if (!meta) return null;

  // Live overlay update (no IPC) — used while dragging.
  const apply = (eq: number, scale: number, ratio: number) => {
    setLatConfig(eq, scale, ratio);
    setOverlayVisible("latLines", true);
  };
  // Persist on slider release / typed value, reading the latest live values so a
  // release after a drag commits the final position.
  const commitLive = () => {
    const { equatorOffset: eq, latScale: scale, lineRatio: r } = useWorldStore.getState().latConfig;
    setLatitudeConfig(eq, scale, r, obliquity).then(setMeta).catch(() => {});
  };

  const setEquatorPct = (pct: number, persist: boolean) => {
    const eq = clamp(pct, 0, 100) / 100;
    apply(eq, latScale, lineRatio);
    if (persist) setLatitudeConfig(eq, latScale, lineRatio, obliquity).then(setMeta).catch(() => {});
  };
  const setExpansionPct = (pct: number, persist: boolean) => {
    const scale = clamp(pct, 25, 400) / 100;
    apply(equatorOffset, scale, lineRatio);
    if (persist) setLatitudeConfig(equatorOffset, scale, lineRatio, obliquity).then(setMeta).catch(() => {});
  };
  const setRatio = (r: number, persist: boolean) => {
    const ratio = clamp(r, 0.5, 5);
    apply(equatorOffset, latScale, ratio);
    if (persist) setLatitudeConfig(equatorOffset, latScale, ratio, obliquity).then(setMeta).catch(() => {});
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
    setLatitudeConfig(0.5, 1, 1, obliquity).then(setMeta).catch(() => {});
  };

  const topLat = latAt(0, equatorOffset, latScale);
  const botLat = latAt(1, equatorOffset, latScale);

  return (
    <div>
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

      <div style={readout}>Visible: {fmtLat(topLat)} &rarr; {fmtLat(botLat)}</div>
      <div style={hint}>
        Line proportion = gap(30→60) ÷ gap(0→30). The SIMULATION uses it too, so
        currents and climate land on the lines. Bands past the poles are cropped.
        Re-run Ocean &amp; Atmosphere onward to generate against these latitudes.
      </div>
      <button onClick={reset} style={resetBtn}>Reset to centered</button>
    </div>
  );
}

// ── Shared control styles ────────────────────────────────────────────────────

export const row: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 2,
};
export const lbl: React.CSSProperties = { fontSize: 10, color: "#5a7090" };
export const val: React.CSSProperties = { fontSize: 10, color: "#8aa0c0", fontFamily: "monospace" };
export const ctrlRow: React.CSSProperties = { display: "flex", gap: 6, alignItems: "center" };
export const range: React.CSSProperties = { flex: 1, minWidth: 0, height: 4, cursor: "pointer" };
export const numInput: React.CSSProperties = {
  width: 50, flexShrink: 0, padding: "2px 4px", fontSize: 11, fontFamily: "monospace",
  background: "#0d1219", color: "#c0d8f0", border: "1px solid #1e2a38",
  borderRadius: 4, textAlign: "right",
};
const readout: React.CSSProperties = {
  fontSize: 10, color: "#8aa0c0", fontFamily: "monospace", marginTop: 6,
};
export const hint: React.CSSProperties = {
  fontSize: 9, color: "#405060", marginTop: 4, lineHeight: 1.35,
};
const resetBtn: React.CSSProperties = {
  marginTop: 6, width: "100%", padding: "3px 0", fontSize: 10, cursor: "pointer",
  background: "#151d28", color: "#7088a0", border: "1px solid #1e2a38", borderRadius: 4,
};
export function chip(active: boolean): React.CSSProperties {
  return {
    flex: 1, padding: "3px 0", fontSize: 10, cursor: "pointer", borderRadius: 4,
    border: active ? "1px solid #3a7ac0" : "1px solid #1e2a38",
    background: active ? "#1a3a5a" : "#0d1219",
    color: active ? "#c0ddf0" : "#5a7090", fontWeight: active ? 600 : 400,
  };
}

/** One planetary-state control: a stepped slider (commits on release) with a live
 *  value readout and preset chips. Drives the energy-balance climate + circulation. */
export function PlanetSlider(props: {
  label: string; unit: string; value: number;
  min: number; max: number; step: number; digits: number;
  onChange: (v: number) => void;
  presets: [string, number][];
  hint: string;
}) {
  const { label, unit, value, min, max, step, digits, onChange, presets, hint: h } = props;
  // Local slider position so dragging is smooth; commit only on release.
  const [live, setLive] = useState(value);
  useEffect(() => { setLive(value); }, [value]);
  return (
    <div style={{ marginTop: 6 }}>
      <div style={row}>
        <span style={lbl}>{label}</span>
        <span style={val}>{live.toFixed(digits)}{unit ? ` ${unit}` : ""}</span>
      </div>
      <div style={ctrlRow}>
        <input
          type="range" min={min} max={max} step={step} value={live}
          onChange={(e) => setLive(Number(e.target.value))}
          onPointerUp={(e) => onChange(Number((e.target as HTMLInputElement).value))}
          onKeyUp={(e) => onChange(Number((e.target as HTMLInputElement).value))}
          style={range}
        />
      </div>
      <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
        {presets.map(([plabel, pval]) => (
          <button key={plabel} style={chip(Math.abs(value - pval) < step / 2)} onClick={() => onChange(pval)}>
            {plabel}
          </button>
        ))}
      </div>
      <div style={hint}>{h}</div>
    </div>
  );
}
