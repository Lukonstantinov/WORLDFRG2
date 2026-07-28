import { useMemo, useState } from "react";
import type { Province, PBuilding, PSettlement } from "@types";
import type { ProvinceRaster } from "@state/worldStore";

// C-1 province subwindow: a minimal mini-map (province footprint + settlements +
// buildings drawn with custom minimalist icons) beside a holdings list. Hovering any
// icon reveals its name + full stats.

// ── Custom minimalist building icons (tiny SVG, no emoji). kind: 0 estate ·
//    1 manufactory · 2 warehouse · 3 bank · 4 mint. Drawn centred on (0,0). ──
const B_COLOR = ["#7fb069", "#d98c40", "#5a9bd4", "#e3c14a", "#c9a24a"];
const B_LABEL = ["Estate", "Manufactory", "Depot", "Bank", "Mint"];
function BuildingGlyph({ kind, s }: { kind: number; s: number }) {
  const c = B_COLOR[kind] ?? "#aaa";
  switch (kind) {
    case 0: // estate — filled field square
      return <rect x={-s} y={-s} width={2 * s} height={2 * s} rx={1} fill={c} stroke="#0a1620" strokeWidth={0.6} />;
    case 1: // manufactory — roof/triangle
      return <path d={`M0 ${-s * 1.2} L ${s} ${s} L ${-s} ${s} Z`} fill={c} stroke="#0a1620" strokeWidth={0.6} />;
    case 2: // warehouse — crate (outlined square with a bar)
      return <g stroke="#0a1620" strokeWidth={0.6}>
        <rect x={-s} y={-s} width={2 * s} height={2 * s} rx={0.5} fill={c} />
        <line x1={-s} y1={0} x2={s} y2={0} stroke="#0a1620" strokeWidth={0.7} />
      </g>;
    case 3: // bank — diamond
      return <path d={`M0 ${-s * 1.3} L ${s * 1.1} 0 L 0 ${s * 1.3} L ${-s * 1.1} 0 Z`} fill={c} stroke="#0a1620" strokeWidth={0.6} />;
    case 4: // mint — coin (circle + dot)
      return <g stroke="#0a1620" strokeWidth={0.6}><circle r={s} fill={c} /><circle r={s * 0.35} fill="#0a1620" /></g>;
    default:
      return <circle r={s} fill={c} />;
  }
}

interface Hover { x: number; y: number; title: string; rows: [string, string][] }

export function ProvinceMiniMap({
  province, raster, settlements, buildings,
}: {
  province: Province;
  raster: ProvinceRaster | null;
  settlements: PSettlement[];
  buildings: PBuilding[];
}) {
  const [hover, setHover] = useState<Hover | null>(null);

  // Extract the province's footprint from the raster → an SVG viewBox. To keep the
  // SHAPE (and its proportions) faithful for a small province as well as a huge one,
  // the sample stride is sized to THIS province's bounding box, not the whole map:
  // pass 1 finds the bbox coarsely, pass 2 re-samples it at a per-province stride so
  // every province renders at a similar ~130-cell fidelity instead of a coarse blob.
  const geo = useMemo(() => {
    if (!raster) return null;
    const { data, w, h, gridW, gridH } = raster;
    // Pass 1 — coarse bbox.
    const coarse = Math.max(1, Math.round(Math.max(w, h) / 300));
    let minx = w, miny = h, maxx = -1, maxy = -1;
    for (let ry = 0; ry < h; ry += coarse) {
      for (let rx = 0; rx < w; rx += coarse) {
        if (data[ry * w + rx] !== province.id) continue;
        if (rx < minx) minx = rx; if (rx > maxx) maxx = rx;
        if (ry < miny) miny = ry; if (ry > maxy) maxy = ry;
      }
    }
    if (maxx < 0) return null;
    // Widen by the coarse step so the fine pass doesn't clip the edges.
    minx = Math.max(0, minx - coarse); miny = Math.max(0, miny - coarse);
    maxx = Math.min(w - 1, maxx + coarse); maxy = Math.min(h - 1, maxy + coarse);
    // Pass 2 — fine scan within the bbox at a province-relative stride.
    const bw = maxx - minx + 1, bh = maxy - miny + 1;
    const stride = Math.max(1, Math.round(Math.max(bw, bh) / 130));
    const cells: [number, number][] = [];
    for (let ry = miny; ry <= maxy; ry += stride) {
      for (let rx = minx; rx <= maxx; rx += stride) {
        if (data[ry * w + rx] === province.id) cells.push([rx, ry]);
      }
    }
    if (cells.length === 0) return null;
    const pad = stride;
    const ox = minx - pad, oy = miny - pad;
    const vw = bw + stride + 2 * pad, vh = bh + stride + 2 * pad;
    // world-cell → raster-cell → local viewBox coords
    const toLocal = (x: number, y: number): [number, number] => [
      (x * w) / gridW - ox, (y * h) / gridH - oy,
    ];
    return { cells, ox, oy, vw, vh, toLocal, stride };
  }, [raster, province.id]);

  if (!geo) return <div style={{ opacity: 0.5, padding: 8 }}>map unavailable</div>;

  const { cells, ox, oy, vw, vh, toLocal, stride } = geo;
  const fill = "#3f6d55";
  const boxW = 240, boxH = Math.max(120, Math.round((240 * vh) / vw));
  // Settlement dot radius proportional to the province footprint so a 4-cell micro-
  // province doesn't have dots bigger than itself, and a 500-cell province still
  // shows legible markers. Clamped to a narrow range so they never disappear either.
  const dotScale = Math.min(1.0, Math.max(0.35, 1.8 / Math.sqrt(cells.length)));
  // City size: marker scales with population (√, so a metropolis reads bigger than a
  // hamlet without dwarfing it), normalised to the largest settlement in the province.
  const maxPop = Math.max(1, ...settlements.map((s) => s.population || 1));
  const popScale = (pop: number) => 0.55 + 1.1 * Math.sqrt((pop || 1) / maxPop);

  return (
    <div style={{ display: "flex", gap: 10, position: "relative" }}>
      {/* Mini-map */}
      <svg width={boxW} height={boxH} viewBox={`0 0 ${vw} ${vh}`}
        style={{ background: "#0a1620", border: "1px solid #1c3242", borderRadius: 6, flexShrink: 0 }}
        onMouseLeave={() => setHover(null)}>
        {/* province footprint (each sample = one stride-sized square) */}
        {cells.map(([rx, ry], i) => (
          <rect key={i} x={rx - ox} y={ry - oy} width={stride * 1.05} height={stride * 1.05} fill={fill} opacity={0.55} />
        ))}
        {/* settlements */}
        {settlements.map((s, i) => {
          const [lx, ly] = toLocal(s.x, s.y);
          const psc = popScale(s.population);
          const r = (s.seat ? 1.6 : 1.1) * dotScale * psc;
          return (
            <g key={`s${i}`} transform={`translate(${lx} ${ly})`} style={{ cursor: "pointer" }}
              onMouseEnter={(e) => setHover({
                x: e.nativeEvent.offsetX, y: e.nativeEvent.offsetY,
                title: `${s.seat ? "★ " : ""}${s.name}${s.seat ? " (seat)" : ""}`,
                rows: [["Population", s.population.toLocaleString()],
                       ["Class", ["ordinary", "trade hub", "entrepôt"][s.hub_class] ?? "—"]],
              })}
              onMouseLeave={() => setHover(null)}>
              {s.seat
                ? <path transform={`scale(${dotScale * psc})`}
                    d="M0 -2.2 L0.7 -0.7 L2.2 -0.7 L1 0.4 L1.4 2 L0 1 L-1.4 2 L-1 0.4 L-2.2 -0.7 L-0.7 -0.7 Z"
                    fill="#fff" stroke="#0a1620" strokeWidth={0.4 / (dotScale * psc)} />
                : <circle r={r} fill="#e8eef4" stroke="#0a1620" strokeWidth={0.4} />}
            </g>
          );
        })}
        {/* buildings — custom minimalist icons */}
        {buildings.map((b, i) => {
          const [lx, ly] = toLocal(b.x, b.y);
          return (
            <g key={`b${i}`} transform={`translate(${lx} ${ly})`} style={{ cursor: "pointer" }}
              onMouseEnter={(e) => setHover({
                x: e.nativeEvent.offsetX, y: e.nativeEvent.offsetY,
                title: `${B_LABEL[b.kind] ?? "Building"} · ${b.name}`,
                rows: b.stats.map((st) => [st.label, st.value] as [string, string]),
              })}
              onMouseLeave={() => setHover(null)}>
              <BuildingGlyph kind={b.kind} s={1.5} />
            </g>
          );
        })}
      </svg>

      {/* Holdings list */}
      <div style={{ flex: 1, minWidth: 0, fontSize: 12, maxHeight: boxH, overflowY: "auto" }}>
        {settlements.map((s, i) => (
          <div key={`ls${i}`} style={{ padding: "1px 0", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
            {s.seat ? "★ " : "· "}{s.name} <span style={{ opacity: 0.55 }}>{s.population.toLocaleString()}</span>
          </div>
        ))}
        {buildings.length > 0 && <div style={{ height: 4 }} />}
        {buildings.map((b, i) => (
          <div key={`lb${i}`} style={{ display: "flex", gap: 5, alignItems: "center", padding: "1px 0" }}>
            <svg width={12} height={12} viewBox="-3 -3 6 6"><BuildingGlyph kind={b.kind} s={1.6} /></svg>
            <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{b.name}</span>
          </div>
        ))}
        {settlements.length === 0 && buildings.length === 0 &&
          <div style={{ opacity: 0.5 }}>frontier — no holdings</div>}
      </div>

      {/* Hover tooltip */}
      {hover && (
        <div style={{
          position: "absolute", left: Math.min(hover.x + 12, boxW - 40), top: hover.y + 8,
          background: "#10202c", border: "1px solid #2a4a5a",
          borderRadius: 6, padding: "6px 8px", pointerEvents: "none", zIndex: 60,
          font: "11px/1.35 system-ui", color: "#dceaf4", boxShadow: "0 4px 14px rgba(0,0,0,.5)",
          maxWidth: 220,
        }}>
          <div style={{ fontWeight: 600, marginBottom: 3 }}>{hover.title}</div>
          {hover.rows.map(([k, v], i) => (
            <div key={i} style={{ display: "flex", justifyContent: "space-between", gap: 10 }}>
              <span style={{ opacity: 0.6 }}>{k}</span><span>{v}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
