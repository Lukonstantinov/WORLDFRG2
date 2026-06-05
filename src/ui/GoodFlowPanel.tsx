import { useEffect, useRef, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useGoodsStore } from "../state/goodsStore";
import type { EconChain, EconCorridor } from "../types";

const MODE_LABEL = ["overland", "maritime", "river"];
const MODE_ICON = ["\u{1F42B}", "⛵", "\u{1F6F6}"]; // camel / sailboat / canoe

// Distinct colours for the multi-line price graph (cycled per route).
const LINE_COLORS = [
  "#e0c060", "#5fc8d8", "#9fe07a", "#d97fb0", "#ff9a6a",
  "#9aa0ff", "#7fe0c0", "#e8d8a0", "#c8a06a", "#b0a0e0",
];

/** Good-flow panel (movable): click a good on the map → every route it travels,
 *  a multi-line price-vs-travel-days graph, and (when a road is picked) a cargo
 *  popup with two side-by-side directional columns. */
export function GoodFlowPanel() {
  const selectedGood = useUIStore((s) => s.selectedGood);
  const setSelectedGood = useUIStore((s) => s.setSelectedGood);
  const selectedRoad = useUIStore((s) => s.selectedRoad);
  const setSelectedRoad = useUIStore((s) => s.setSelectedRoad);
  const economy = useWorldStore((s) => s.economy);
  const goodMeta = useGoodsStore((s) => s.meta);

  const [pos, setPos] = useState({ x: 70, y: 70 });
  const dragRef = useRef<{ dx: number; dy: number } | null>(null);

  // Window drag handlers (header drag moves the whole panel).
  useEffect(() => {
    const move = (e: PointerEvent) => {
      if (!dragRef.current) return;
      setPos({ x: e.clientX - dragRef.current.dx, y: e.clientY - dragRef.current.dy });
    };
    const up = () => { dragRef.current = null; };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => { window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up); };
  }, []);

  if (!selectedGood || !economy) return null;

  const icon = goodMeta(selectedGood).icon;
  const label = goodMeta(selectedGood).name;
  const hubName = (id: number) => economy.hubs.find((h) => h.id === id)?.name ?? `Hub ${id}`;

  const routes: EconChain[] = economy.chains
    .filter((c) => c.good_name === selectedGood)
    .sort((a, b) => b.value - a.value)
    .slice(0, 10);

  // Corridor lookup (hub-pair → corridor) for cargo diversity + columns.
  const corrMap = new Map<string, EconCorridor>();
  for (const c of economy.corridors) corrMap.set(`${c.a},${c.b}`, c);
  const corrFor = (h1: number, h2: number) => corrMap.get(`${Math.min(h1, h2)},${Math.max(h1, h2)}`);

  const routeDiversity = (chain: EconChain): number => {
    const goods = new Set<number>();
    for (let i = 0; i < chain.stops.length - 1; i++) {
      const c = corrFor(chain.stops[i].hub, chain.stops[i + 1].hub);
      if (c) { for (const g of c.fwd_goods) goods.add(g.good); for (const g of c.bwd_goods) goods.add(g.good); }
    }
    return goods.size;
  };

  // ── Multi-line price/days graph geometry ──
  const W = 280, H = 130, padL = 30, padR = 10, padT = 10, padB = 22;
  const maxDays = Math.max(1, ...routes.map((r) => r.days));
  const maxPrice = Math.max(1.2, ...routes.flatMap((r) => r.stops.map((s) => s.price)));
  const sx = (d: number) => padL + (d / maxDays) * (W - padL - padR);
  const sy = (p: number) => H - padB - (p / maxPrice) * (H - padT - padB);

  // ── Cargo popup data for the selected road ──
  const road = selectedRoad !== null ? routes.find((r) => r.id === selectedRoad) ?? null : null;
  let fwdCol: { good_name: string; value: number }[] = [];
  let bwdCol: { good_name: string; value: number }[] = [];
  if (road) {
    const fwd = new Map<string, number>();
    const bwd = new Map<string, number>();
    for (let i = 0; i < road.stops.length - 1; i++) {
      const c = corrFor(road.stops[i].hub, road.stops[i + 1].hub);
      if (!c) continue;
      for (const g of c.fwd_goods) fwd.set(g.good_name, (fwd.get(g.good_name) ?? 0) + g.value);
      for (const g of c.bwd_goods) bwd.set(g.good_name, (bwd.get(g.good_name) ?? 0) + g.value);
    }
    const toSorted = (m: Map<string, number>) =>
      [...m.entries()].map(([good_name, value]) => ({ good_name, value })).sort((a, b) => b.value - a.value);
    fwdCol = toSorted(fwd);
    bwdCol = toSorted(bwd);
  }
  const colMax = Math.max(1e-6, ...fwdCol.map((g) => g.value), ...bwdCol.map((g) => g.value));

  return (
    <>
      <div style={{ ...panel, left: pos.x, top: pos.y }}>
        {/* Header (drag handle) */}
        <div
          onPointerDown={(e) => { dragRef.current = { dx: e.clientX - pos.x, dy: e.clientY - pos.y }; }}
          style={header}
        >
          <span style={{ fontSize: 14 }}>{icon} {label}</span>
          <span onClick={() => setSelectedGood(null)} style={closeBtn} title="Close">×</span>
        </div>

        {routes.length === 0 && (
          <div style={emptyTxt}>No traded routes — this good is consumed locally or unreachable.</div>
        )}

        {routes.length > 0 && (
          <>
            {/* Price-vs-days graph (multi-line) */}
            <div style={sectionHdr}>Price along the journey (× base vs. travel days)</div>
            <svg width={W} height={H} style={{ display: "block", background: "#0b1622", borderRadius: 4 }}>
              {/* axes */}
              <line x1={padL} y1={padT} x2={padL} y2={H - padB} stroke="#24364e" />
              <line x1={padL} y1={H - padB} x2={W - padR} y2={H - padB} stroke="#24364e" />
              <text x={2} y={padT + 6} fill="#5a7090" fontSize={8}>{maxPrice.toFixed(1)}×</text>
              <text x={2} y={H - padB} fill="#5a7090" fontSize={8}>1×</text>
              <text x={W - padR} y={H - 6} fill="#5a7090" fontSize={8} textAnchor="end">{Math.round(maxDays)}d</text>
              <text x={padL} y={H - 6} fill="#5a7090" fontSize={8}>0d</text>
              {routes.map((r, ri) => {
                const color = LINE_COLORS[ri % LINE_COLORS.length];
                const active = selectedRoad === r.id;
                const pts = r.stops.map((s) => `${sx(s.days).toFixed(1)},${sy(s.price).toFixed(1)}`).join(" ");
                return (
                  <g key={r.id} opacity={selectedRoad === null || active ? 1 : 0.3}>
                    <polyline points={pts} fill="none" stroke={color} strokeWidth={active ? 2.4 : 1.2} />
                    {r.stops.map((s, i) => (
                      <circle key={i} cx={sx(s.days)} cy={sy(s.price)} r={active ? 2.4 : 1.6} fill={color} />
                    ))}
                  </g>
                );
              })}
            </svg>

            {/* Road list */}
            <div style={{ ...sectionHdr, marginTop: 6 }}>Roads (click to highlight on the map)</div>
            <div style={{ maxHeight: 150, overflowY: "auto" }}>
              {routes.map((r, ri) => {
                const active = selectedRoad === r.id;
                const dest = r.stops.length > 0 ? r.stops[r.stops.length - 1].hub : -1;
                const origin = r.stops.length > 0 ? r.stops[0].hub : -1;
                return (
                  <div key={r.id} onClick={() => setSelectedRoad(active ? null : r.id)}
                    style={{ ...row, cursor: "pointer", background: active ? "#1a2c40" : "transparent" }}>
                    <span style={{ width: 8, height: 8, borderRadius: 8, background: LINE_COLORS[ri % LINE_COLORS.length], flex: "0 0 auto" }} />
                    <span style={{ flex: 1, color: active ? "#e8d8b0" : "#c0d0e0", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {hubName(origin)} → {hubName(dest)}
                    </span>
                    <span style={{ color: "#8aa0c0", fontSize: 9 }} title={MODE_LABEL[r.mode]}>{MODE_ICON[r.mode]}</span>
                    <span style={{ color: "#9ab0c8", fontSize: 9, minWidth: 30, textAlign: "right" }}>{Math.round(r.days)}d</span>
                    <span style={{ color: "#7aa890", fontSize: 9, minWidth: 24, textAlign: "right" }} title="distinct goods on this road">⬡{routeDiversity(r)}</span>
                    <span style={{ color: "#e0c060", fontSize: 9, minWidth: 34, textAlign: "right" }} title="shipment value">{r.value.toFixed(1)}</span>
                  </div>
                );
              })}
            </div>
            <div style={{ color: "#506680", fontSize: 9, marginTop: 4 }}>
              {MODE_ICON.join(" ")} = overland / sea / river · ⬡ = distinct goods on the road · value = volume × delivered price.
            </div>
          </>
        )}
      </div>

      {/* Cargo popup (secondary): two directional columns for the selected road. */}
      {road && (fwdCol.length > 0 || bwdCol.length > 0) && (
        <div style={{ ...cargoPopup, left: pos.x + 312, top: pos.y }}>
          <div style={{ ...header, cursor: "default" }}>
            <span style={{ fontSize: 11 }}>Cargo both ways</span>
            <span onClick={() => setSelectedRoad(null)} style={closeBtn} title="Close">×</span>
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            {([["→ outbound", fwdCol], ["← return", bwdCol]] as const).map(([title, col]) => (
              <div key={title} style={{ flex: 1 }}>
                <div style={{ color: "#6a86a6", fontSize: 9, marginBottom: 3 }}>{title} · {col.length}</div>
                {col.length === 0 && <div style={emptyTxt}>—</div>}
                {col.slice(0, 12).map((g) => (
                  <div key={g.good_name} style={{ marginBottom: 2 }}>
                    <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: "#c0d0e0" }}>
                      <span>{goodMeta(g.good_name).icon} {goodMeta(g.good_name).name}</span>
                      <span style={{ color: "#9ab0c8" }}>{g.value.toFixed(1)}</span>
                    </div>
                    <div style={{ height: 4, background: "#13202e", borderRadius: 2 }}>
                      <div style={{ height: 4, width: `${(g.value / colMax) * 100}%`, background: "#5fc8a8", borderRadius: 2 }} />
                    </div>
                  </div>
                ))}
              </div>
            ))}
          </div>
        </div>
      )}
    </>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", width: 300, background: "rgba(12,18,26,0.97)",
  border: "1px solid #24364e", borderRadius: 8, padding: "8px 10px", zIndex: 120,
  boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
const cargoPopup: React.CSSProperties = {
  position: "absolute", width: 230, background: "rgba(12,18,26,0.97)",
  border: "1px solid #24364e", borderRadius: 8, padding: "8px 10px", zIndex: 121,
  boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  marginBottom: 6, color: "#e8d8b0", fontWeight: 700, cursor: "grab", userSelect: "none",
};
const closeBtn: React.CSSProperties = { color: "#7090b0", cursor: "pointer", fontSize: 18, lineHeight: 1 };
const sectionHdr: React.CSSProperties = {
  color: "#6a86a6", fontSize: 10, fontWeight: 700, textTransform: "uppercase",
  letterSpacing: 0.5, borderBottom: "1px solid #1e2e42", paddingBottom: 2, marginBottom: 4,
};
const row: React.CSSProperties = { display: "flex", alignItems: "center", gap: 5, fontSize: 11, padding: "2px 3px", borderRadius: 3 };
const emptyTxt: React.CSSProperties = { color: "#506680", fontSize: 10, fontStyle: "italic", padding: "2px 3px" };
