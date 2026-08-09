import { useEffect } from "react";
import { useUIStore } from "@state/uiStore";
import { usePaletteStore, rampGradient, bandGradient } from "@state/paletteStore";
import { koppenName } from "./climate";
import type { ActiveLayer } from "@types";

/** THE MAP KEY, for every layer that has an exact one.
 *
 *  Before this, `ElevationLegend` early-returned unless the layer was `elevation`
 *  or `terrain` — so 23 of 25 layers rendered with no key and no units anywhere on
 *  screen. Selecting "Precipitation" gave a tan-to-blue field with nothing saying
 *  what any of it meant.
 *
 *  Every colour here comes from `get_render_palettes`, i.e. the renderer's own
 *  constants (§8.18). Layers whose ramps are still written inline in Rust are
 *  deliberately NOT given an invented key — they are served by the StatusBar hover
 *  readout instead, which reports the real value under the cursor. A legend that
 *  guesses is worse than no legend; that is precisely how the old one broke. */

type LegendKind =
  | { kind: "elevation" }
  | { kind: "temperature" }
  | { kind: "precipitation" }
  | { kind: "koppen" };

const LAYER_LEGENDS: Partial<Record<ActiveLayer, LegendKind>> = {
  elevation: { kind: "elevation" },
  terrain: { kind: "elevation" },
  temperature: { kind: "temperature" },
  sst: { kind: "temperature" },
  precipitation: { kind: "precipitation" },
  climate: { kind: "koppen" },
};

const box: React.CSSProperties = {
  position: "absolute", left: 8, bottom: 8, zIndex: 12,
  background: "rgba(10,16,24,0.90)", border: "1px solid #1e2e42",
  borderRadius: 8, padding: "8px 10px", color: "#cfe2f6",
  fontSize: 11, pointerEvents: "none", maxWidth: 340,
  boxShadow: "0 6px 18px rgba(0,0,0,0.45)",
};
const title: React.CSSProperties = {
  fontSize: 10, textTransform: "uppercase", letterSpacing: 1,
  color: "#6a86a6", marginBottom: 5, fontWeight: 600,
};
const barBase: React.CSSProperties = {
  height: 12, borderRadius: 3, border: "1px solid #24354a",
};
const ticks: React.CSSProperties = {
  display: "flex", justifyContent: "space-between",
  fontSize: 9, color: "#7d97b0", marginTop: 3,
  fontVariantNumeric: "tabular-nums",
};

export function LayerLegend() {
  const activeLayer = useUIStore((s) => s.activeLayer);
  const palettes = usePaletteStore((s) => s.palettes);
  const load = usePaletteStore((s) => s.load);

  useEffect(() => { void load(); }, [load]);

  const spec = LAYER_LEGENDS[activeLayer];
  if (!spec || !palettes) return null;

  if (spec.kind === "elevation") {
    const land = palettes.elevation;
    const sea = palettes.bathymetry;
    return (
      <div style={box}>
        <div style={title}>Elevation</div>
        <div style={{ ...barBase, background: rampGradient(land) }} />
        {/* Labels sit at each stop's TRUE metric position, so a colour read off
            the map resolves to the height the renderer actually used. */}
        <div style={{ position: "relative", height: 12, marginTop: 2 }}>
          {land.map((s, i) => {
            const pct = (s.at / (land[land.length - 1].at || 1)) * 100;
            const edge = i === 0 ? "flex-start" : i === land.length - 1 ? "flex-end" : "center";
            return (
              <span key={s.at} style={{
                position: "absolute", left: `${pct}%`, fontSize: 9, color: "#7d97b0",
                transform: edge === "flex-start" ? "none" : edge === "flex-end" ? "translateX(-100%)" : "translateX(-50%)",
                fontVariantNumeric: "tabular-nums", whiteSpace: "nowrap",
              }}>{s.at >= 1000 ? `${(s.at / 1000).toFixed(s.at >= 5000 ? 0 : 1)}k` : s.at}</span>
            );
          })}
        </div>
        <div style={{ ...title, marginTop: 10 }}>Sea depth</div>
        <div style={{ ...barBase, background: rampGradient(sea) }} />
        <div style={ticks}><span>Shore</span><span>Shelf</span><span>Abyss</span></div>
        <div style={{ fontSize: 9, color: "#5a7390", marginTop: 6, lineHeight: 1.4 }}>
          Metres above sea level. Breaks follow atlas convention, not even steps.
        </div>
      </div>
    );
  }

  if (spec.kind === "temperature") {
    const t = palettes.temperature;
    return (
      <div style={box}>
        <div style={title}>{activeLayer === "sst" ? "Sea-surface temperature" : "Temperature"}</div>
        <div style={{ ...barBase, background: rampGradient(t) }} />
        <div style={ticks}>
          {t.filter((_, i) => i % 2 === 0 || t[i].at === 0).map((s) => (
            <span key={s.at}>{s.at > 0 ? `+${s.at}` : s.at}°</span>
          ))}
        </div>
        <div style={{ fontSize: 9, color: "#5a7390", marginTop: 6, lineHeight: 1.4 }}>
          The pale band is 0 °C — freezing, and the same scale on both the land and
          sea plates, so one colour means one temperature.
        </div>
      </div>
    );
  }

  if (spec.kind === "precipitation") {
    const p = palettes.precipitation;
    return (
      <div style={box}>
        <div style={title}>Precipitation</div>
        <div style={{ ...barBase, background: bandGradient(p) }} />
        <div style={{ display: "flex", fontSize: 9, color: "#7d97b0", marginTop: 3, fontVariantNumeric: "tabular-nums" }}>
          {p.map((b, i) => (
            <span key={b.at} style={{ flex: 1, textAlign: "center" }}>
              {i === p.length - 1 ? "4k+" : b.at >= 1000 ? `${b.at / 1000}k` : b.at}
            </span>
          ))}
        </div>
        <div style={{ fontSize: 9, color: "#5a7390", marginTop: 6, lineHeight: 1.4 }}>
          mm/yr, in classed bands — rainfall is log-distributed, so atlases class it
          rather than blending.
        </div>
      </div>
    );
  }

  // Köppen: a class list, not a ramp. Two columns keep 31 zones on screen.
  const k = palettes.koppen;
  return (
    <div style={{ ...box, maxWidth: 300, maxHeight: "46vh", overflow: "hidden" }}>
      <div style={title}>Climate — Köppen</div>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "1px 8px" }}>
        {k.map((c) => (
          <div key={c.code} style={{ display: "flex", alignItems: "center", gap: 5, minWidth: 0 }}>
            <span style={{
              width: 10, height: 10, flex: "0 0 auto", borderRadius: 2,
              background: c.color, border: "1px solid rgba(0,0,0,0.4)",
            }} />
            <span style={{
              fontSize: 9.5, color: "#a8bed4", whiteSpace: "nowrap",
              overflow: "hidden", textOverflow: "ellipsis",
            }}>{koppenName(c.code)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
