import { useEffect, useRef, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useGoodsStore } from "../state/goodsStore";
import type { EconChain, EconHub } from "../types";
import { koppenName } from "./climate";

const MODE_LABEL = ["overland", "maritime", "river"];
const MODE_ICON = ["\u{1F42B}", "⛵", "\u{1F6F6}"]; // camel / sailboat / canoe
const MODE_PHRASE = ["overland by caravan", "by sea", "up the rivers"];

// Distinct colours for the multi-line price graph (cycled per route).
const LINE_COLORS = [
  "#e0c060", "#5fc8d8", "#9fe07a", "#d97fb0", "#ff9a6a",
  "#9aa0ff", "#7fe0c0", "#e8d8a0", "#c8a06a", "#b0a0e0",
];

/** Good-flow panel (movable): click a good on the map → every route it travels, a
 *  multi-line price-vs-travel-days graph, the full road list, and — for each road
 *  opened INDEPENDENTLY — a price/toll ladder and a generated merchant's-tale of
 *  who bought it, where it travelled, and to whom it was sold. */
export function GoodFlowPanel() {
  const selectedGood = useUIStore((s) => s.selectedGood);
  const setSelectedGood = useUIStore((s) => s.setSelectedGood);
  const openRoads = useUIStore((s) => s.openRoads);
  const toggleOpenRoad = useUIStore((s) => s.toggleOpenRoad);
  const economy = useWorldStore((s) => s.economy);
  const goodMeta = useGoodsStore((s) => s.meta);

  const [pos, setPos] = useState({ x: 70, y: 60 });
  const dragRef = useRef<{ dx: number; dy: number } | null>(null);

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
  const hubById = (id: number) => economy.hubs.find((h) => h.id === id);

  // All routes for this good (no hard cap — the list is scrollable).
  const routes: EconChain[] = economy.chains
    .filter((c) => c.good_name === selectedGood)
    .sort((a, b) => b.value - a.value);

  // Good-stats header (biggest importer/exporter + the class that craves it).
  const gstat = economy.good_stats?.find((g) => g.good_name === selectedGood);

  // ── Multi-line price/days graph geometry (Y anchored at 1×) ──
  const W = 340, H = 160, padL = 30, padR = 10, padT = 10, padB = 22;
  const graphRoutes = routes.slice(0, 12);
  const maxDays = Math.max(1, ...graphRoutes.map((r) => r.days));
  const maxPrice = Math.max(1.3, ...graphRoutes.flatMap((r) => r.stops.map((s) => s.price)));
  const sx = (d: number) => padL + (d / maxDays) * (W - padL - padR);
  const sy = (p: number) => H - padB - ((p - 1) / (maxPrice - 1)) * (H - padT - padB);

  return (
    <div style={{ ...panel, left: pos.x, top: pos.y }}>
      {/* Header (drag handle) */}
      <div onPointerDown={(e) => { dragRef.current = { dx: e.clientX - pos.x, dy: e.clientY - pos.y }; }} style={header}>
        <span style={{ fontSize: 14 }}>{icon} {label}</span>
        <span onClick={() => setSelectedGood(null)} style={closeBtn} title="Close">×</span>
      </div>

      {/* Who moves the most of it + who craves it */}
      {gstat && (
        <div style={{ color: "#9ab0c8", fontSize: 9, marginBottom: 4, lineHeight: 1.5 }}>
          {gstat.top_exporter >= 0 && <>Greatest exporter: <b style={{ color: "#7fd0a0" }}>{hubName(gstat.top_exporter)}</b>. </>}
          {gstat.top_importer >= 0 && <>Hungriest market: <b style={{ color: "#ff9a6a" }}>{hubName(gstat.top_importer)}</b>. </>}
          {gstat.biggest_desire_class && <>Most desired by the <b style={{ color: "#e0c060" }}>{gstat.biggest_desire_class}</b>.</>}
        </div>
      )}

      {routes.length === 0 && (
        <div style={emptyTxt}>No traded routes — this good is consumed locally or unreachable.</div>
      )}

      {routes.length > 0 && (
        <>
          {/* Price-vs-days graph (multi-line) */}
          <div style={sectionHdr}>Price along the journey (× base vs. travel days)</div>
          <svg width={W} height={H} style={{ display: "block", background: "#0b1622", borderRadius: 4 }}>
            <line x1={padL} y1={padT} x2={padL} y2={H - padB} stroke="#24364e" />
            <line x1={padL} y1={H - padB} x2={W - padR} y2={H - padB} stroke="#24364e" />
            <text x={2} y={padT + 6} fill="#5a7090" fontSize={8}>{maxPrice.toFixed(1)}×</text>
            <text x={2} y={H - padB} fill="#5a7090" fontSize={8}>1×</text>
            <text x={W - padR} y={H - 6} fill="#5a7090" fontSize={8} textAnchor="end">{Math.round(maxDays)}d</text>
            <text x={padL} y={H - 6} fill="#5a7090" fontSize={8}>0d</text>
            {graphRoutes.map((r, ri) => {
              const color = LINE_COLORS[ri % LINE_COLORS.length];
              const active = openRoads.includes(r.id);
              const anyOpen = openRoads.length > 0;
              const pts = r.stops.map((s) => `${sx(s.days).toFixed(1)},${sy(s.price).toFixed(1)}`).join(" ");
              return (
                <g key={r.id} opacity={!anyOpen || active ? 1 : 0.25}>
                  <polyline points={pts} fill="none" stroke={color} strokeWidth={active ? 2.4 : 1.2} />
                  {r.stops.map((s, i) => (
                    <circle key={i} cx={sx(s.days)} cy={sy(s.price)} r={active ? 2.4 : 1.6} fill={color} />
                  ))}
                </g>
              );
            })}
          </svg>

          {/* Road list — click to open/close each independently */}
          <div style={{ ...sectionHdr, marginTop: 6 }}>Roads ({routes.length}) — click to open</div>
          <div style={{ maxHeight: 150, overflowY: "auto" }}>
            {routes.map((r, ri) => {
              const active = openRoads.includes(r.id);
              const dest = r.stops.length > 0 ? r.stops[r.stops.length - 1].hub : -1;
              const origin = r.stops.length > 0 ? r.stops[0].hub : -1;
              const term = r.stops.length > 0 ? r.stops[r.stops.length - 1].price : 1;
              return (
                <div key={r.id} onClick={() => toggleOpenRoad(r.id)}
                  style={{ ...rowS, cursor: "pointer", background: active ? "#1a2c40" : "transparent" }}>
                  <span style={{ width: 8, height: 8, borderRadius: 8, background: LINE_COLORS[ri % LINE_COLORS.length], flex: "0 0 auto" }} />
                  <span style={{ flex: 1, color: active ? "#e8d8b0" : "#c0d0e0", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {hubName(origin)} → {hubName(dest)}
                  </span>
                  <span style={{ color: "#8aa0c0", fontSize: 9 }} title={MODE_LABEL[r.mode]}>{MODE_ICON[r.mode]}</span>
                  <span style={{ color: "#9ab0c8", fontSize: 9, minWidth: 28, textAlign: "right" }}>{Math.round(r.days)}d</span>
                  <span style={{ color: "#ff9a6a", fontSize: 9, minWidth: 30, textAlign: "right" }}>{term.toFixed(1)}×</span>
                </div>
              );
            })}
          </div>

          {/* Opened roads: price/toll ladder + merchant's tale, one block each */}
          {openRoads
            .map((id) => routes.find((r) => r.id === id))
            .filter((r): r is EconChain => !!r)
            .map((r) => <RoadDetail key={r.id} chain={r} hubName={hubName} hubById={hubById} iconFor={(id) => goodMeta(id).icon} labelFor={(id) => goodMeta(id).name} />)}

          <div style={{ color: "#506680", fontSize: 9, marginTop: 4 }}>
            {MODE_ICON.join(" ")} = overland / sea / river · click roads to open several at once.
          </div>
        </>
      )}
    </div>
  );
}

/** One opened road's detail: a per-stop price ladder (markup / toll / demand) and a
 *  generated merchant narrative. */
function RoadDetail({ chain, hubName, hubById, iconFor, labelFor }: {
  chain: EconChain;
  hubName: (id: number) => string;
  hubById: (id: number) => EconHub | undefined;
  iconFor: (id: string) => string;
  labelFor: (id: string) => string;
}) {
  const tollTotal = chain.stops.reduce((a, s) => a + (s.toll ?? 0), 0);
  return (
    <div style={{ marginTop: 6, padding: "6px 8px", background: "#0b1622", borderRadius: 5, border: "1px solid #1e3550" }}>
      <div style={{ color: "#9ab0c8", fontSize: 10, marginBottom: 4 }}>
        {iconFor(chain.good_name)} {hubName(chain.stops[0]?.hub ?? 0)} → {hubName(chain.stops[chain.stops.length - 1]?.hub ?? 0)}
        <span style={{ color: "#6a86a6" }}>{`  ·  ${Math.round(chain.days)}d`}{tollTotal > 0 ? `  ·  tolls +${Math.round(tollTotal * 100)}%` : ""}</span>
      </div>
      {/* Price ladder per stop */}
      {chain.stops.map((s, i) => (
        <div key={i} style={{ display: "flex", alignItems: "baseline", gap: 5, fontSize: 9, marginBottom: 1 }}>
          <span style={{ color: "#5a7090", minWidth: 14 }}>{i === 0 ? "◉" : "→"}</span>
          <span style={{ flex: 1, color: i === 0 ? "#80dc8c" : i === chain.stops.length - 1 ? "#ff9a6a" : "#e0c060", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {hubName(s.hub)}
          </span>
          {(s.toll ?? 0) > 0 && <span style={{ color: "#d98a5a", fontSize: 8 }} title="toll">+{Math.round((s.toll ?? 0) * 100)}%🛃</span>}
          {(s.demand_spike ?? 0) > 0.25 && <span style={{ color: "#d97fb0", fontSize: 8 }} title="demand spike">▲</span>}
          <span style={{ color: "#c0d0e0", minWidth: 30, textAlign: "right" }}>{s.price.toFixed(1)}×</span>
        </div>
      ))}
      {/* Merchant's tale */}
      <div style={{ marginTop: 4, color: "#a8bcd4", fontSize: 9.5, fontStyle: "italic", lineHeight: 1.45 }}>
        {merchantStory(chain, hubName, hubById, labelFor)}
      </div>
    </div>
  );
}

/** Generated prose: who bought the good where, the climate it crossed, and to whom
 *  it was sold at each hub (noble / merchants / split) — built purely from the chain
 *  + hub data, no simulation. */
function merchantStory(
  chain: EconChain,
  hubName: (id: number) => string,
  hubById: (id: number) => EconHub | undefined,
  labelFor: (id: string) => string,
): string {
  const good = labelFor(chain.good_name);
  const stops = chain.stops;
  if (stops.length === 0) return "";
  const mode = MODE_PHRASE[chain.mode] ?? "overland";
  const buyer = (id: number): string => {
    const h = hubById(id);
    const e = h?.elite_level ?? 0;
    const m = h?.merchant_level ?? 0;
    if (e > 0.5 && m > 0.4) return "split between a noble house and the merchant guilds";
    if (e > 0.5) return "to a noble house for its table and coffers";
    if (m > 0.35) return "to other merchants, who load it for the next leg";
    return "to local traders at the market";
  };

  let s = `A merchant of ${hubName(stops[0].hub)} (${koppenName(stops[0].koppen ?? 0)}) buys ${good} at the source for ${stops[0].price.toFixed(1)}×.`;
  for (let i = 1; i < stops.length; i++) {
    const prev = stops[i - 1];
    const cur = stops[i];
    const legDays = Math.max(0, Math.round((cur.days ?? 0) - (prev.days ?? 0)));
    const last = i === stops.length - 1;
    s += ` Carried ${mode}${legDays > 0 ? ` ${legDays}d` : ""} to ${hubName(cur.hub)} (${koppenName(cur.koppen ?? 0)})`;
    const reasons: string[] = [];
    if ((cur.toll ?? 0) > 0) reasons.push("after tolls");
    if ((cur.demand_spike ?? 0) > 0.25) reasons.push("where it is keenly wanted");
    s += `, it changes hands at ${cur.price.toFixed(1)}×${reasons.length ? ` (${reasons.join(", ")})` : ""}`;
    s += last ? `, sold ${buyer(cur.hub)}.` : `, ${buyer(cur.hub)}.`;
  }
  return s;
}

const panel: React.CSSProperties = {
  position: "absolute", width: 360, maxHeight: "90vh", overflowY: "auto",
  background: "rgba(12,18,26,0.97)", border: "1px solid #24364e", borderRadius: 8,
  padding: "8px 10px", zIndex: 120, boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
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
const rowS: React.CSSProperties = { display: "flex", alignItems: "center", gap: 5, fontSize: 11, padding: "2px 3px", borderRadius: 3 };
const emptyTxt: React.CSSProperties = { color: "#506680", fontSize: 10, fontStyle: "italic", padding: "2px 3px" };
