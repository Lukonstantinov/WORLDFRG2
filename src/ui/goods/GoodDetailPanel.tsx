import { useEffect, useMemo, useRef, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useGoodsStore } from "@state/goodsStore";
import { previewGoodScore } from "@bridge";
import { koppenName, koppenHex, koppenRgb } from "@ui/world/climate";
import { goodCategory } from "@goods";
import { commodityHistory } from "@app/commodityHistory";
import type { GoodSpec } from "@types";

const panel: React.CSSProperties = {
  position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)", zIndex: 1200,
  display: "flex", alignItems: "center", justifyContent: "center",
};
const sheet: React.CSSProperties = {
  width: "min(620px, 94vw)", maxHeight: "90vh", overflow: "auto",
  background: "#13181f", border: "1px solid #2c3442", borderRadius: 8,
  padding: 16, color: "#cfd8e3", fontSize: 12,
};
const tag: React.CSSProperties = {
  display: "inline-block", padding: "2px 7px", borderRadius: 10, fontSize: 11,
  background: "#1b2230", border: "1px solid #2c3442", marginRight: 5,
};

function prodType(d: GoodSpec["distribution"]): { label: string; emoji: string } {
  switch (d) {
    case "deposits": return { label: "Extracted (deposit)", emoji: "🪨" };
    case "manufactured": return { label: "Manufactured (in cities)", emoji: "🏭" };
    default: return { label: "Planted / Grown", emoji: "🌾" };
  }
}

export function GoodDetailPanel() {
  const id = useUIStore((s) => s.goodDetailId);
  const close = useUIStore((s) => s.setGoodDetail);
  const specs = useGoodsStore((s) => s.specs);
  const spec = useMemo(() => specs.find((s) => s.id === id) ?? null, [specs, id]);
  const byId = useMemo(() => new Map(specs.map((s) => [s.id, s])), [specs]);

  const [grid, setGrid] = useState<{ width: number; height: number; data: number[]; land: number[] } | null>(null);
  const [err, setErr] = useState<string>("");
  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Dominant Köppen zone = the highest-weighted climate in the good's envelope;
  // its colour tints both the climate swatches and the suitability heatmap.
  const climates = useMemo(
    () => [...(spec?.scoring?.climate ?? [])].sort((a, b) => b[1] - a[1]),
    [spec],
  );
  const tintRgb: [number, number, number] = useMemo(() => {
    if (climates.length > 0) return koppenRgb(climates[0][0]);
    // No climate envelope (built-in scorer / deposit) → fall back to good colour.
    const c = spec?.color ?? "#7aa0c8";
    const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(c);
    return m ? [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)] : [122, 160, 200];
  }, [climates, spec]);

  const isManufactured = spec?.distribution === "manufactured";

  // Fetch the suitability preview grid for non-manufactured goods.
  useEffect(() => {
    setGrid(null); setErr("");
    if (!spec || isManufactured) return;
    let live = true;
    previewGoodScore(spec)
      .then((g) => { if (live) setGrid(g); })
      .catch((e) => { if (live) setErr(String(e)); });
    return () => { live = false; };
  }, [spec, isManufactured]);

  // Paint the heatmap: each cell darkens→tint by suitability over a near-black sea.
  useEffect(() => {
    const cv = canvasRef.current;
    if (!cv || !grid) return;
    const { width: w, height: h, data } = grid;
    cv.width = w; cv.height = h;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    const img = ctx.createImageData(w, h);
    const [tr, tg, tb] = tintRgb;
    const land = grid.land ?? [];
    // A land cell that touches sea (or the map edge) is a coastline pixel — we
    // brighten it so the world's landmass shape reads behind the heatmap.
    const isLand = (x: number, y: number) =>
      x >= 0 && x < w && y >= 0 && y < h ? land[y * w + x] === 1 : false;
    for (let i = 0; i < w * h; i++) {
      const v = Math.min(255, data[i]) / 255;
      const k = Math.pow(v, 0.7); // gamma so faint suitability still reads
      const o = i * 4;
      let r = 11 + (tr - 11) * k;
      let g = 14 + (tg - 14) * k;
      let b = 19 + (tb - 19) * k;
      const x = i % w, y = (i / w) | 0;
      if (land[i] === 1) {
        // Faint land fill so even zero-suitability land is distinguishable from sea.
        r = Math.max(r, 26); g = Math.max(g, 31); b = Math.max(b, 38);
        // Thin, DIM coastline (one pixel where land meets sea) — just enough to
        // read the landmass without competing with the suitability heatmap.
        const coast = !isLand(x - 1, y) || !isLand(x + 1, y) || !isLand(x, y - 1) || !isLand(x, y + 1);
        if (coast && k < 0.25) { r = 70; g = 84; b = 98; }
      }
      img.data[o] = Math.round(r);
      img.data[o + 1] = Math.round(g);
      img.data[o + 2] = Math.round(b);
      img.data[o + 3] = 255;
    }
    ctx.putImageData(img, 0, 0);
  }, [grid, tintRgb]);

  if (!id) return null;
  if (!spec) return null;

  const pt = prodType(spec.distribution);
  const sc = spec.scoring;
  const hist = commodityHistory(spec.id);

  return (
    <div style={panel} onClick={() => close(null)}>
      <div style={sheet} onClick={(e) => e.stopPropagation()}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
          <strong style={{ fontSize: 16 }}>
            <span style={{ marginRight: 6 }}>{spec.icon}</span>{spec.name}
          </strong>
          <button onClick={() => close(null)} style={{ ...tag, cursor: "pointer", marginRight: 0 }}>✕ Close</button>
        </div>

        <div style={{ marginBottom: 10 }}>
          <span style={{ ...tag, borderColor: "#3a5070" }}>{pt.emoji} {pt.label}</span>
          <span style={tag}>{goodCategory(spec.id)}</span>
          <span style={tag}>domain: {spec.domain}</span>
          {spec.network_luxury && <span style={{ ...tag, color: "#d9b06a", borderColor: "#6a5a30" }}>luxury</span>}
        </div>
        <div style={{ marginBottom: 12, color: "#8aa0b8" }}>
          <span style={tag}>value ≈ {spec.base_value}× grain</span>
          <span style={tag}>bulk {spec.bulk ?? 1}</span>
          {(spec.perishable ?? 0) > 0 && (
            <span style={{ ...tag, color: (spec.perishable ?? 0) >= 0.3 ? "#e0a080" : undefined }}>
              perishable {spec.perishable}{(spec.perishable ?? 0) >= 0.3 ? " · stays at origin" : ""}
            </span>
          )}
        </div>

        {hist && (
          <div style={{ background: "#0e1a14", border: "1px solid #1c3326", borderRadius: 6, padding: 12, marginBottom: 12 }}>
            <div style={{ color: "#d8c878", fontSize: 11, fontWeight: 700, letterSpacing: 0.3 }}>📜 {hist.era}</div>
            <div style={{ color: "#cfe2f6", fontSize: 12, lineHeight: 1.6, marginTop: 5 }}>{hist.note}</div>
          </div>
        )}

        {isManufactured ? (
          <div style={{ background: "#0e1218", border: "1px solid #232b36", borderRadius: 6, padding: 12 }}>
            <div style={{ color: "#8aa0b8", marginBottom: 6 }}>Made in cities from a recipe — no map belt of its own.</div>
            <div style={{ display: "flex", alignItems: "center", gap: 4, flexWrap: "wrap", marginBottom: 8 }}>
              {(spec.inputs ?? []).map((inp, k) => {
                const s = byId.get(inp.good);
                return (
                  <span key={inp.good} style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
                    {k > 0 && <span style={{ color: "#5a6a80" }}>+</span>}
                    <span style={{ ...tag, marginRight: 0, cursor: s ? "pointer" : "default" }}
                      onClick={() => s && close(s.id)}>
                      {s?.icon ?? "❔"} {s?.name ?? inp.good}{inp.qty !== 1 ? ` ×${inp.qty}` : ""}
                    </span>
                  </span>
                );
              })}
              <span style={{ color: "#7a8aa0", margin: "0 4px", fontWeight: 700 }}>→</span>
              <span style={{ ...tag, marginRight: 0, borderColor: spec.color }}>{spec.icon} {spec.name}</span>
            </div>
            <div style={{ color: "#5f7390", fontSize: 11 }}>
              A city's manufacture guild produces it wherever the inputs arrive — so it appears
              in big trading hubs that import {(spec.inputs ?? []).map((i) => byId.get(i.good)?.name ?? i.good).join(", ")}.
            </div>
          </div>
        ) : (
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div>
              <div style={{ color: "#8aa0b8", marginBottom: 4 }}>Appears in climates</div>
              {climates.length === 0 && (
                <div style={{ color: "#5f7390" }}>
                  Built-in climate scorer — no explicit zone list. The heatmap shows where it suits this world.
                </div>
              )}
              {climates.map(([code, wt]) => (
                <div key={code} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
                  <span style={{ width: 13, height: 13, borderRadius: 3, background: koppenHex(code), border: "1px solid #00000055", flex: "0 0 auto" }} />
                  <span style={{ flex: 1, color: "#c0ccda" }}>{koppenName(code)}</span>
                  <span style={{ width: 46, height: 5, background: "#1b2230", borderRadius: 3, overflow: "hidden" }}>
                    <span style={{ display: "block", height: "100%", width: `${Math.round(wt * 100)}%`, background: koppenHex(code) }} />
                  </span>
                </div>
              ))}
              {sc && (
                <div style={{ marginTop: 8, color: "#7a8aa0", fontSize: 11, lineHeight: 1.6 }}>
                  {sc.temp && <div>🌡 temp ≈ {sc.temp[0]}°C (±{sc.temp[1]})</div>}
                  {sc.precip && <div>🌧 rain {sc.precip[0]}–{sc.precip[1]} mm</div>}
                  {sc.abs_lat && <div>🧭 latitude {sc.abs_lat[0]}–{sc.abs_lat[1]}°</div>}
                  {sc.elevation && <div>⛰ elevation {Math.round(sc.elevation[0] * 8848)}–{Math.round(sc.elevation[1] * 8848)} m</div>}
                  {sc.coast_bonus > 0 && <div>🌊 favours the coast</div>}
                  {sc.fertility > 0 && <div>🌱 favours fertile soil</div>}
                </div>
              )}
            </div>
            <div>
              <div style={{ color: "#8aa0b8", marginBottom: 4 }}>Where it could appear — this world</div>
              <canvas
                ref={canvasRef}
                style={{
                  width: "100%", imageRendering: "pixelated", borderRadius: 4,
                  border: "1px solid #232b36", background: "#0b0e13",
                  display: grid ? "block" : "none",
                }}
              />
              {!grid && !err && <div style={{ color: "#5f7390" }}>Generate a world first to preview placement…</div>}
              {err && <div style={{ color: "#c08080" }}>Preview needs a generated world.</div>}
              {grid && (
                <div style={{ color: "#5f7390", fontSize: 10, marginTop: 4 }}>
                  brightness = suitability · tinted with{" "}
                  {climates.length > 0 ? `the ${koppenName(climates[0][0])} climate colour` : "the good's colour"}
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
