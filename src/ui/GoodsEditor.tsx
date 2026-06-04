import { useEffect, useRef, useState } from "react";
import { useGoodsStore } from "../state/goodsStore";
import { previewGoodScore } from "../bridge/tauri";
import type { GoodSpec, GoodDomain, GoodDistribution, GoodEnvelope } from "../types";

const DOMAINS: GoodDomain[] = ["marine", "coastal", "continental", "island"];
const DISTRIBUTIONS: GoodDistribution[] = ["global", "local", "deposits"];

// Climate presets → Köppen zone codes (see sim/koppen.rs).
const CLIMATE_PRESETS: { label: string; codes: number[] }[] = [
  { label: "Tropical", codes: [1, 2, 3, 23, 24] },
  { label: "Arid", codes: [4, 5, 6, 7] },
  { label: "Mediterranean", codes: [8, 9, 10] },
  { label: "Temperate", codes: [11, 12, 13, 25] },
  { label: "Continental", codes: [14, 15, 16, 17, 27, 28, 29] },
  { label: "Polar", codes: [21, 22] },
];

const panel: React.CSSProperties = {
  position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)", zIndex: 1000,
  display: "flex", alignItems: "center", justifyContent: "center",
};
const sheet: React.CSSProperties = {
  width: "min(880px, 94vw)", maxHeight: "88vh", overflow: "auto",
  background: "#161b22", border: "1px solid #2c3442", borderRadius: 8,
  padding: 14, color: "#cfd8e3", fontSize: 12,
};
const btn: React.CSSProperties = {
  background: "#222b38", color: "#cfd8e3", border: "1px solid #36425a",
  borderRadius: 4, padding: "4px 9px", cursor: "pointer", fontSize: 11,
};
const primaryBtn: React.CSSProperties = { ...btn, background: "#2d5fa8", borderColor: "#3f74c0", color: "#fff" };
const row: React.CSSProperties = {
  display: "grid", gridTemplateColumns: "20px 22px 1.4fr 0.9fr 0.9fr 1fr 1fr 60px 56px",
  gap: 6, alignItems: "center", padding: "4px 0", borderBottom: "1px solid #232b36",
};
const input: React.CSSProperties = {
  background: "#0e1218", color: "#cfd8e3", border: "1px solid #2c3442", borderRadius: 3,
  padding: "2px 4px", fontSize: 11, width: "100%",
};

function num(v: number) { return Math.round(v * 100) / 100; }

export function GoodsEditor() {
  const {
    specs, editorOpen, dirty, setOpen, update, addCustom, duplicate, remove,
    applyToWorld, saveToLibrary, loadFromLibrary, resetDefaults, loadFromWorld,
  } = useGoodsStore();
  const [expanded, setExpanded] = useState<number | null>(null);
  const [status, setStatus] = useState("");

  useEffect(() => { if (editorOpen && specs.length === 0) void loadFromWorld(); }, [editorOpen, specs.length, loadFromWorld]);

  if (!editorOpen) return null;

  const act = (label: string, fn: () => Promise<void>) => async () => {
    try { await fn(); setStatus(label); } catch (e) { setStatus(`Error: ${e}`); }
  };

  const enabledCount = specs.filter((s) => s.enabled).length;

  return (
    <div style={panel} onClick={() => setOpen(false)}>
      <div style={sheet} onClick={(e) => e.stopPropagation()}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
          <strong style={{ fontSize: 14 }}>Trade Goods Library</strong>
          <span style={{ color: "#7a8aa0" }}>
            {enabledCount} enabled · {specs.length} total{dirty ? " · unsaved" : ""}
          </span>
        </div>

        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>
          <button style={btn} onClick={addCustom}>+ Add custom</button>
          <button style={btn} onClick={act("Loaded global library", loadFromLibrary)}>Load library</button>
          <button style={btn} onClick={act("Reset to defaults", resetDefaults)}>Reset to defaults</button>
          <div style={{ flex: 1 }} />
          <button style={btn} onClick={act("Saved to global library", saveToLibrary)}>Save to library</button>
          <button style={primaryBtn} onClick={act("Applied to this world", applyToWorld)}>Apply to world</button>
          <button style={btn} onClick={() => setOpen(false)}>Close</button>
        </div>

        <div style={{ ...row, fontWeight: 600, color: "#8aa0b8", borderBottom: "1px solid #36425a" }}>
          <span>On</span><span></span><span>Name</span><span>Domain</span><span>Dist.</span>
          <span>Rarity</span><span>Desire</span><span>Lux</span><span></span>
        </div>

        {specs.map((g, i) => (
          <GoodRow
            key={`${g.id}_${i}`} g={g} i={i} expanded={expanded === i}
            onToggleExpand={() => setExpanded(expanded === i ? null : i)}
            update={update} duplicate={duplicate} remove={remove}
          />
        ))}

        <div style={{ marginTop: 8, color: "#7a8aa0", minHeight: 16 }}>{status}</div>
        <div style={{ marginTop: 4, color: "#5a6a80", fontSize: 10 }}>
          Apply to world, then re-run the Biological-Trade step to regenerate goods with these settings.
          Built-in goods keep their original scoring unless duplicated into a custom good.
        </div>
      </div>
    </div>
  );
}

function GoodRow({ g, i, expanded, onToggleExpand, update, duplicate, remove }: {
  g: GoodSpec; i: number; expanded: boolean; onToggleExpand: () => void;
  update: (i: number, patch: Partial<GoodSpec>) => void;
  duplicate: (i: number) => void; remove: (i: number) => void;
}) {
  return (
    <>
      <div style={row}>
        <input type="checkbox" checked={g.enabled} onChange={(e) => update(i, { enabled: e.target.checked })} />
        <span title={g.id} style={{ fontSize: 14 }}>{g.icon}</span>
        <input style={input} value={g.name} onChange={(e) => update(i, { name: e.target.value })} />
        <select style={input} value={g.domain} onChange={(e) => update(i, { domain: e.target.value as GoodDomain })}>
          {DOMAINS.map((d) => <option key={d} value={d}>{d}</option>)}
        </select>
        <select style={input} value={g.distribution} onChange={(e) => update(i, { distribution: e.target.value as GoodDistribution })}>
          {DISTRIBUTIONS.map((d) => <option key={d} value={d}>{d}</option>)}
        </select>
        <Slider value={g.rarity} onChange={(v) => update(i, { rarity: v })} />
        <Slider value={g.desire} onChange={(v) => update(i, { desire: v })} />
        <input type="checkbox" checked={g.network_luxury} onChange={(e) => update(i, { network_luxury: e.target.checked })} />
        <div style={{ display: "flex", gap: 3 }}>
          <button style={{ ...btn, padding: "2px 5px" }} title="Icon / color / envelope" onClick={onToggleExpand}>⋯</button>
          {!g.builtin && <button style={{ ...btn, padding: "2px 5px", color: "#e08080" }} title="Delete" onClick={() => remove(i)}>✕</button>}
        </div>
      </div>
      {expanded && (
        <div style={{ padding: "8px 6px 12px 28px", borderBottom: "1px solid #232b36", display: "flex", flexWrap: "wrap", gap: 14 }}>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
            Icon <input style={{ ...input, width: 50 }} value={g.icon} onChange={(e) => update(i, { icon: e.target.value })} />
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
            Color <input type="color" value={g.color} onChange={(e) => update(i, { color: e.target.value })} />
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
            <input type="checkbox" checked={g.network_luxury} onChange={(e) => update(i, { network_luxury: e.target.checked })} /> network luxury
          </label>
          <button style={btn} onClick={() => duplicate(i)}>Duplicate as custom</button>
          {g.builtin
            ? <span style={{ color: "#7a8aa0", flexBasis: "100%" }}>Built-in scoring (duplicate to customize its climate/temperature envelope).</span>
            : <EnvelopeEditor i={i} env={g.scoring ?? null} update={update} />}
          <PreviewCanvas spec={g} />
        </div>
      )}
    </>
  );
}

function EnvelopeEditor({ i, env, update }: {
  i: number; env: GoodEnvelope | null;
  update: (i: number, patch: Partial<GoodSpec>) => void;
}) {
  const e: GoodEnvelope = env ?? { climate: [], fertility: 0.4, coast_bonus: 0 };
  const setEnv = (patch: Partial<GoodEnvelope>) => update(i, { scoring: { ...e, ...patch } });
  const activeCodes = new Set(e.climate.map(([c]) => c));
  const presetOn = (codes: number[]) => codes.every((c) => activeCodes.has(c));
  const togglePreset = (codes: number[]) => {
    const on = presetOn(codes);
    const next = new Set(activeCodes);
    for (const c of codes) { if (on) next.delete(c); else next.add(c); }
    setEnv({ climate: [...next].map((c) => [c, 1.0] as [number, number]) });
  };

  return (
    <div style={{ flexBasis: "100%", display: "flex", flexDirection: "column", gap: 6 }}>
      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
        <span style={{ color: "#8aa0b8", alignSelf: "center" }}>Climate:</span>
        {CLIMATE_PRESETS.map((p) => (
          <button key={p.label}
            style={{ ...btn, background: presetOn(p.codes) ? "#2d5fa8" : "#222b38", padding: "2px 7px" }}
            onClick={() => togglePreset(p.codes)}>{p.label}</button>
        ))}
        <span style={{ color: "#5a6a80", alignSelf: "center" }}>(none = any climate)</span>
      </div>
      <Range label="Temp °C" lo={e.temp?.[0] ?? 18} width={e.temp?.[1] ?? 8}
        onLo={(v) => setEnv({ temp: [v, e.temp?.[1] ?? 8] })} onWidth={(v) => setEnv({ temp: [e.temp?.[0] ?? 18, v] })}
        loMin={-20} loMax={35} wMin={2} wMax={25} />
      <BandRow label="Precip mm" band={e.precip ?? [400, 1600, 500]} onChange={(b) => setEnv({ precip: b })} max={3000} step={50} />
      <BandRow label="Elevation" band={e.elevation ?? [0, 0.5, 0.25]} onChange={(b) => setEnv({ elevation: b })} max={1} step={0.02} />
      <div style={{ display: "flex", gap: 16 }}>
        <Labeled label={`Fertility ${num(e.fertility)}`}>
          <input type="range" min={0} max={1} step={0.05} value={e.fertility} onChange={(ev) => setEnv({ fertility: parseFloat(ev.target.value) })} />
        </Labeled>
        <Labeled label={`Coast bonus ${num(e.coast_bonus)}`}>
          <input type="range" min={0} max={1} step={0.05} value={e.coast_bonus} onChange={(ev) => setEnv({ coast_bonus: parseFloat(ev.target.value) })} />
        </Labeled>
      </div>
    </div>
  );
}

function Slider({ value, onChange }: { value: number; onChange: (v: number) => void }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
      <input type="range" min={0} max={1} step={0.05} value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))} style={{ width: "100%" }} />
      <span style={{ color: "#8aa0b8", width: 24, fontSize: 10 }}>{num(value)}</span>
    </div>
  );
}

function Labeled({ label, children }: { label: string; children: React.ReactNode }) {
  return <label style={{ display: "flex", flexDirection: "column", gap: 2, color: "#8aa0b8" }}>{label}{children}</label>;
}

function Range({ label, lo, width, onLo, onWidth, loMin, loMax, wMin, wMax }: {
  label: string; lo: number; width: number; onLo: (v: number) => void; onWidth: (v: number) => void;
  loMin: number; loMax: number; wMin: number; wMax: number;
}) {
  return (
    <div style={{ display: "flex", gap: 16 }}>
      <Labeled label={`${label} center ${num(lo)}`}>
        <input type="range" min={loMin} max={loMax} step={1} value={lo} onChange={(e) => onLo(parseFloat(e.target.value))} />
      </Labeled>
      <Labeled label={`width ${num(width)}`}>
        <input type="range" min={wMin} max={wMax} step={1} value={width} onChange={(e) => onWidth(parseFloat(e.target.value))} />
      </Labeled>
    </div>
  );
}

function BandRow({ label, band, onChange, max, step }: {
  label: string; band: [number, number, number]; onChange: (b: [number, number, number]) => void;
  max: number; step: number;
}) {
  const [lo, hi, edge] = band;
  return (
    <div style={{ display: "flex", gap: 16 }}>
      <Labeled label={`${label} lo ${num(lo)}`}>
        <input type="range" min={0} max={max} step={step} value={lo} onChange={(e) => onChange([parseFloat(e.target.value), hi, edge])} />
      </Labeled>
      <Labeled label={`hi ${num(hi)}`}>
        <input type="range" min={0} max={max} step={step} value={hi} onChange={(e) => onChange([lo, parseFloat(e.target.value), edge])} />
      </Labeled>
      <Labeled label={`falloff ${num(edge)}`}>
        <input type="range" min={0} max={max} step={step} value={edge} onChange={(e) => onChange([lo, hi, parseFloat(e.target.value)])} />
      </Labeled>
    </div>
  );
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const h = hex.replace("#", "");
  const n = h.length === 3
    ? h.split("").map((c) => c + c).join("")
    : h.padEnd(6, "0").slice(0, 6);
  const v = parseInt(n, 16);
  return { r: (v >> 16) & 255, g: (v >> 8) & 255, b: v & 255 };
}

/** Live suitability heatmap for a good spec (debounced; brighter = better fit). */
function PreviewCanvas({ spec }: { spec: GoodSpec }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const key = JSON.stringify(spec);
  useEffect(() => {
    let cancelled = false;
    const t = setTimeout(() => {
      previewGoodScore(spec).then((grid) => {
        if (cancelled) return;
        const cv = ref.current;
        if (!cv || grid.width === 0) return;
        cv.width = grid.width;
        cv.height = grid.height;
        const ctx = cv.getContext("2d");
        if (!ctx) return;
        const img = ctx.createImageData(grid.width, grid.height);
        const col = hexToRgb(spec.color);
        for (let p = 0; p < grid.data.length; p++) {
          const v = grid.data[p] / 255;
          const o = p * 4;
          img.data[o] = Math.round(8 + col.r * v);
          img.data[o + 1] = Math.round(10 + col.g * v);
          img.data[o + 2] = Math.round(14 + col.b * v);
          img.data[o + 3] = 255;
        }
        ctx.putImageData(img, 0, 0);
      }).catch(() => {});
    }, 350);
    return () => { cancelled = true; clearTimeout(t); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return (
    <div style={{ flexBasis: "100%" }}>
      <div style={{ fontSize: 10, color: "#7a8aa0", marginBottom: 3 }}>
        Suitability preview (brighter = better fit) — updates as you edit
      </div>
      <canvas ref={ref} style={{
        width: 280, height: 140, imageRendering: "pixelated",
        border: "1px solid #2a3442", borderRadius: 4, background: "#06090d",
      }} />
    </div>
  );
}
