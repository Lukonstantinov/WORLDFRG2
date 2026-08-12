import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { usePaletteStore, rampGradient, bandGradient, rampAt, mixHex } from "@state/paletteStore";
import { koppenName } from "./climate";
import { getBiomeStats, getElevationDistribution, type ElevationBand } from "@bridge";
import type { BiomeStat } from "@types";
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
  | { kind: "koppen" }
  | { kind: "biomes" };

const LAYER_LEGENDS: Partial<Record<ActiveLayer, LegendKind>> = {
  elevation: { kind: "elevation" },
  terrain: { kind: "elevation" },
  temperature: { kind: "temperature" },
  sst: { kind: "temperature" },
  precipitation: { kind: "precipitation" },
  climate: { kind: "koppen" },
  biomes: { kind: "biomes" },
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
  const isolateClass = useUIStore((s) => s.isolateClass);
  const setIsolateClass = useUIStore((s) => s.setIsolateClass);
  const [biomes, setBiomes] = useState<BiomeStat[]>([]);
  const [dist, setDist] = useState<ElevationBand[]>([]);

  useEffect(() => { void load(); }, [load]);

  const spec = LAYER_LEGENDS[activeLayer];

  // Biome NAMES come from the world itself, so the list shows only the biomes this
  // world actually has — a key listing 41 classes when 20 are present is noise.
  useEffect(() => {
    if (spec?.kind !== "biomes" || biomes.length > 0) return;
    getBiomeStats().then(setBiomes).catch(() => {});
  }, [spec?.kind, biomes.length]);

  // The elevation DISTRIBUTION, folded into the key. A ramp without it cannot tell
  // you that most of the world sits in its bottom band — which is precisely the
  // fact that motivated re-spacing the ramp onto atlas breaks.
  useEffect(() => {
    if (spec?.kind !== "elevation" || dist.length > 0) return;
    getElevationDistribution().then(setDist).catch(() => {});
  }, [spec?.kind, dist.length]);

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
        {dist.length > 0 && (() => {
          const maxPct = Math.max(...dist.map((d) => d.percentage), 1);
          const top = palettes.elevation[palettes.elevation.length - 1].at;
          return (
            <div style={{ marginTop: 8 }}>
              <div style={{ display: "flex", alignItems: "flex-end", height: 26, gap: 1 }}>
                {dist.map((d, i) => (
                  <div key={d.label}
                    title={`${d.label} — ${d.percentage.toFixed(1)}% of land`}
                    style={{
                      // Bars sit at their TRUE metric width on the ramp's own axis,
                      // so the shape reads against the colours directly above it.
                      flex: Math.min(1000, top - i * 1000) / top,
                      height: `${Math.max(2, (d.percentage / maxPct) * 100)}%`,
                      background: rampAt(palettes.elevation, i * 1000 + 500),
                      opacity: 0.85,
                    }} />
                ))}
              </div>
              <div style={{ fontSize: 8.5, color: "#4d6480", marginTop: 2 }}>
                share of land by height
              </div>
            </div>
          );
        })()}

        <div style={{ ...title, marginTop: 10 }}>Sea depth</div>
        <div style={{ ...barBase, background: rampGradient(sea) }} />
        <div style={ticks}><span>Shore</span><span>Shelf</span><span>Abyss</span></div>
        <div style={{ fontSize: 9, color: "#5a7390", marginTop: 6, lineHeight: 1.4 }}>
          Metres above sea level; breaks follow atlas convention, not even steps.
          Below 1200 m the tint also carries CLIMATE — desert lowland reads khaki,
          rainforest green, tundra grey — converging on this ramp above it.
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

  // A CLASS list, not a ramp — and clickable. On a 41-class biome map where
  // adjacent greens are near-indistinguishable, isolating one class answers
  // "where is THIS one" without needing a palette that separates 41 classes,
  // which may not be achievable at all.
  const isBiome = spec.kind === "biomes";
  const rows = isBiome
    ? biomes.map((b) => ({
        code: b.code,
        label: b.name,
        color: palettes.biome.find((c) => c.code === b.code)?.color ?? "#6e7660",
      }))
    : palettes.koppen.map((c) => ({ code: c.code, label: koppenName(c.code), color: c.color }));

  return (
    <div style={{ ...box, maxWidth: 320, maxHeight: "50vh", overflowY: "auto", pointerEvents: "auto" }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
        <div style={{ ...title, marginBottom: 5, flex: 1 }}>
          {isBiome ? "Biomes" : "Climate — Köppen"}
        </div>
        {isolateClass !== null && (
          <span onClick={() => setIsolateClass(null)} title="Show all classes"
            style={{ fontSize: 9, color: "#6db2d6", cursor: "pointer", whiteSpace: "nowrap" }}>
            show all ×
          </span>
        )}
      </div>
      <div style={{ display: "grid", gridTemplateColumns: isBiome ? "1fr" : "1fr 1fr", gap: "1px 8px" }}>
        {rows.map((r) => {
          const on = isolateClass === r.code;
          const muted = isolateClass !== null && !on;
          return (
            <div key={r.code}
              onClick={() => setIsolateClass(on ? null : r.code)}
              title={on ? "Show all classes" : `Isolate ${r.label}`}
              style={{
                display: "flex", alignItems: "center", gap: 5, minWidth: 0,
                cursor: "pointer", borderRadius: 3, padding: "1px 3px",
                background: on ? "#1e3a58" : "transparent",
                opacity: muted ? 0.45 : 1,
              }}>
              <span style={{
                width: 10, height: 10, flex: "0 0 auto", borderRadius: 2,
                background: r.color, border: "1px solid rgba(0,0,0,0.4)",
              }} />
              <span style={{
                fontSize: 9.5, color: on ? "#cfe2f6" : "#a8bed4", whiteSpace: "nowrap",
                overflow: "hidden", textOverflow: "ellipsis",
                fontWeight: on ? 600 : 400,
              }}>{r.label}</span>
            </div>
          );
        })}
      </div>
      <div style={{ fontSize: 9, color: "#5a7390", marginTop: 6, lineHeight: 1.4 }}>
        Click a class to isolate it; the rest of the map dims but stays visible.
      </div>
    </div>
  );
}

const box2: React.CSSProperties = { ...box, left: "auto", right: 8 };

/** GOODS_LOCALITIES_PLAN.md D10 · the trade-good QUALITY key. Independent of
 *  `LayerLegend` above (it keys off the ACTIVE base layer; this keys off the
 *  `goodQuality` OVERLAY, which can be switched on over any base layer at all), so
 *  it is its own small component, docked to the opposite corner so the two never
 *  overlap when both are showing at once.
 *
 *  Reads `good_quality_grades` for its swatches — the SAME breakpoints and, in
 *  heatmap mode, the exact colours `get_render_palettes` computed for them — so the
 *  key cannot show a colour the map itself would not paint at that grade (§8.18). */
export function GoodQualityLegend() {
  const on = useUIStore((s) => s.overlayVisibility.goodQuality);
  const heatmap = useUIStore((s) => s.goodQualityHeatmap);
  const palettes = usePaletteStore((s) => s.palettes);
  const load = usePaletteStore((s) => s.load);
  useEffect(() => { void load(); }, [load]);

  if (!on || !palettes) return null;
  const grades = palettes.good_quality_grades;
  if (grades.length === 0) return null;

  return (
    <div style={box2}>
      <div style={title}>Trade-good quality</div>
      {heatmap ? (
        <>
          <div style={{ ...barBase, background: rampGradient(palettes.good_quality_heatmap) }} />
          <div style={ticks}><span>poorest</span><span>finest</span></div>
        </>
      ) : (
        <div style={{ ...barBase, background: `linear-gradient(to right, ${palettes.good_quality_pale}, #6a6a6a)` }} />
      )}
      <div style={{ display: "flex", justifyContent: "space-between", marginTop: 5, gap: 2 }}>
        {grades.map((g) => (
          <div key={g.at} style={{ display: "flex", flexDirection: "column", alignItems: "center", flex: 1, minWidth: 0 }}>
            <span style={{
              width: 12, height: 12, borderRadius: 3, border: "1px solid rgba(0,0,0,0.4)",
              background: heatmap ? g.color : mixHex(palettes.good_quality_pale, "#6a6a6a", g.at),
            }} />
            <span style={{ fontSize: 8.5, color: "#93a8bd", marginTop: 2, whiteSpace: "nowrap", textTransform: "capitalize" }}>
              {g.label}
            </span>
          </div>
        ))}
      </div>
      <div style={{ fontSize: 9, color: "#5a7390", marginTop: 6, lineHeight: 1.4 }}>
        {heatmap
          ? "One absolute scale, shared by every good — a red patch of any good outgrades a blue patch of any other."
          : <>Colour is the good itself; shading is how strongly it shows here. Toggle <b>🌡 heatmap</b> above the goods list for a colour-coded reading instead.</>}
      </div>
    </div>
  );
}
