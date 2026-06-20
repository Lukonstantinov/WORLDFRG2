/** Trade ▸ Flows subtab — the REALIZED trade at a settlement once a campaign has
 *  run: per-good average + last-year volume with a trend graph (so a fallen trade
 *  is visible), the routes a good came along (from/to + share), and the top partner
 *  cities. Selecting a good or a partner highlights the routes on the map. */
import { useEffect, useMemo, useState } from "react";
import { campaignTradeFlows } from "../bridge/tauri";
import type { TradeFlows } from "../types";
import { GOOD_DEFS } from "../goods";

type Seg = { ax: number; ay: number; bx: number; by: number; dir: number; w: number };

const GOOD_META = new Map(GOOD_DEFS.map((g) => [g.name, g]));
function fmt(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(n >= 10000 ? 0 : 1)}k`;
  return n.toFixed(n >= 100 ? 0 : 1);
}

/** A tiny trend sparkline of a good's yearly trade volume. Green when rising into
 *  the last year, red when it has fallen from its peak. */
function Spark({ vals }: { vals: number[] }) {
  if (vals.length < 2) return <span style={{ color: "#566", fontSize: 9 }}>—</span>;
  const w = 150, h = 30, max = Math.max(...vals, 1e-6);
  const pts = vals.map((v, i) => `${(i / (vals.length - 1)) * w},${h - (v / max) * (h - 3) - 1.5}`).join(" ");
  const last = vals[vals.length - 1], peak = max;
  const fallen = last < peak * 0.6;
  const color = fallen ? "#e06a5a" : last >= vals[vals.length - 2] ? "#6fce8f" : "#d9c46a";
  return (
    <svg width={w} height={h} style={{ display: "block" }}>
      <polyline points={pts} fill="none" stroke={color} strokeWidth={1.4} />
      <circle cx={w} cy={h - (last / max) * (h - 3) - 1.5} r={2} fill={color} />
    </svg>
  );
}

export function FlowsView({ hubId, active, tick, setFlowHighlight }: {
  hubId: number; active: boolean; tick: number; setFlowHighlight: (s: Seg[]) => void;
}) {
  const [flows, setFlows] = useState<TradeFlows | null>(null);
  const [loading, setLoading] = useState(false);
  const [selGood, setSelGood] = useState<number | null>(null);
  const [selPartner, setSelPartner] = useState<number | null>(null);

  // Fetch on open / when the campaign year ticks over (flows refresh yearly). Track
  // a real loading flag so a resolved-but-empty result ("no trade recorded") is told
  // apart from "still fetching" — otherwise a null reply hung on "Loading…" forever.
  useEffect(() => {
    if (!active) { setFlows(null); setLoading(false); return; }
    let alive = true;
    setLoading(true);
    campaignTradeFlows(hubId)
      .then((f) => { if (alive) { setFlows(f); setLoading(false); } })
      .catch(() => { if (alive) { setFlows(null); setLoading(false); } });
    return () => { alive = false; };
  }, [hubId, active, tick]);

  // Reset selection when the settlement changes.
  useEffect(() => { setSelGood(null); setSelPartner(null); }, [hubId]);

  // Drive the map highlight from the current selection.
  useEffect(() => {
    if (!flows) { setFlowHighlight([]); return; }
    const ax = flows.hub_x + 0.5, ay = flows.hub_y + 0.5;
    let segs: Seg[] = [];
    if (selGood != null) {
      const rs = flows.routes.filter((r) => r.good === selGood);
      const max = Math.max(...rs.map((r) => r.amount), 1e-6);
      segs = rs.map((r) => ({ ax, ay, bx: r.px + 0.5, by: r.py + 0.5, dir: r.dir, w: 1 + (r.amount / max) * 3 }));
    } else if (selPartner != null) {
      const rs = flows.routes.filter((r) => r.partner === selPartner);
      const max = Math.max(...rs.map((r) => r.amount), 1e-6);
      segs = rs.map((r) => ({ ax, ay, bx: r.px + 0.5, by: r.py + 0.5, dir: r.dir, w: 1 + (r.amount / max) * 3 }));
    }
    setFlowHighlight(segs);
  }, [flows, selGood, selPartner, setFlowHighlight]);

  const goodRoutes = useMemo(
    () => (flows && selGood != null ? flows.routes.filter((r) => r.good === selGood) : []),
    [flows, selGood]);

  if (!active) return <Note>Realized trade appears once a campaign is running.</Note>;
  if (loading && !flows) return <Note>Loading trade flows…</Note>;
  if (!flows) return <Note>No trade data for this settlement yet.</Note>;
  if (flows.goods.length === 0) return <Note>No trade recorded yet — let a campaign year or two pass.</Note>;

  const maxAvg = Math.max(...flows.goods.map((g) => g.avg_volume), 1e-6);
  const maxPartner = Math.max(...flows.partners.map((p) => p.pct), 1e-6);

  return (
    <div style={{ fontSize: 11, color: "#c7d6e8" }}>
      {/* ── Traded goods (avg / yr) with trend ── */}
      <div style={hdr}>Traded goods — average per year <span style={{ color: "#5a7290", fontWeight: 400 }}>(click to map its routes)</span></div>
      {flows.goods.slice(0, 16).map((g) => {
        const meta = GOOD_META.get(g.name);
        const sel = selGood === g.good;
        return (
          <div key={g.good} onClick={() => { setSelGood(sel ? null : g.good); setSelPartner(null); }}
            style={{ ...row, background: sel ? "#1a2a3c" : "transparent", cursor: "pointer", flexWrap: "wrap" }}>
            <span style={{ width: 16 }}>{meta?.emoji ?? "•"}</span>
            <span style={{ flex: 1, minWidth: 70, color: sel ? "#cfe2f6" : "#c7d6e8" }}>{meta?.label ?? g.name}</span>
            <div style={{ flex: 1.4, minWidth: 60, height: 7, background: "#16202c", borderRadius: 3, overflow: "hidden" }}>
              <div style={{ width: `${(g.avg_volume / maxAvg) * 100}%`, height: "100%", background: meta?.color ?? "#5a8fc0" }} />
            </div>
            <span style={{ width: 52, textAlign: "right", color: "#9fb4cc" }}>{fmt(g.avg_volume)}/yr</span>
            <span style={{ width: 40, textAlign: "right", color: "#6a86a6", fontSize: 9 }}>
              {g.in_volume >= g.out_volume ? "▼in" : "▲out"} {g.route_count}r
            </span>
            {sel && (
              <div style={{ width: "100%", display: "flex", alignItems: "center", gap: 8, padding: "3px 0 1px 16px" }}>
                <Spark vals={g.history} />
                <span style={{ color: "#7a90a8", fontSize: 9 }}>
                  last yr {fmt(g.last_volume)} · {g.history.length}-yr trend
                  {g.history.length >= 2 && g.last_volume < Math.max(...g.history) * 0.6 ? " · ⚠ fallen" : ""}
                </span>
              </div>
            )}
          </div>
        );
      })}

      {/* ── Selected good's routes ── */}
      {selGood != null && (
        <>
          <div style={hdr}>Routes — {GOOD_META.get(flows.goods.find((x) => x.good === selGood)?.name ?? "")?.label ?? "good"}
            <span style={{ color: "#5a7290", fontWeight: 400 }}> (largest flows first)</span></div>
          {goodRoutes.length === 0 && (() => {
            const g = flows.goods.find((x) => x.good === selGood);
            if (g && g.avg_volume > 0) {
              return <Note>Not traded last year — {g.history.length}-yr average {fmt(g.avg_volume)}/yr.
                Per-partner routes are recorded for the most recent trading year only.</Note>;
            }
            return <Note>No routed flows recorded.</Note>;
          })()}
          {goodRoutes.slice(0, 8).map((r, i) => (
            <div key={i} style={{ ...row }}>
              <span style={{ width: 30, color: r.dir === 0 ? "#5fd0ff" : "#ffce5f" }}>{r.dir === 0 ? "◀ in" : "out ▶"}</span>
              <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {r.dir === 0 ? `${r.partner_name} → here` : `here → ${r.partner_name}`}
              </span>
              <span style={{ width: 50, textAlign: "right", color: "#9fb4cc" }}>{fmt(r.amount)}</span>
              <span style={{ width: 40, textAlign: "right", color: "#6a86a6" }}>{r.pct.toFixed(0)}%</span>
            </div>
          ))}
        </>
      )}

      {/* ── Top partner cities ── */}
      <div style={hdr}>Top partner cities <span style={{ color: "#5a7290", fontWeight: 400 }}>(share of all trade · click to map)</span></div>
      {flows.partners.map((p) => {
        const sel = selPartner === p.hub;
        return (
          <div key={p.hub} onClick={() => { setSelPartner(sel ? null : p.hub); setSelGood(null); }}
            style={{ ...row, background: sel ? "#1a2a3c" : "transparent", cursor: "pointer" }}>
            <span style={{ flex: 1, minWidth: 70, color: sel ? "#cfe2f6" : "#c7d6e8", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{p.name}</span>
            <div style={{ flex: 1, minWidth: 50, height: 7, background: "#16202c", borderRadius: 3, overflow: "hidden" }}>
              <div style={{ width: `${(p.pct / maxPartner) * 100}%`, height: "100%", background: "#c99a3a" }} />
            </div>
            <span style={{ width: 36, textAlign: "right", color: "#9fb4cc" }}>{p.pct.toFixed(0)}%</span>
            <span style={{ width: 96, textAlign: "right", color: "#6a86a6", fontSize: 12, overflow: "hidden", whiteSpace: "nowrap" }}>
              {p.goods.map((gn) => GOOD_META.get(gn)?.emoji ?? "").join("")}
            </span>
          </div>
        );
      })}
    </div>
  );
}

const hdr: React.CSSProperties = { color: "#7fa0c4", fontSize: 10, textTransform: "uppercase", margin: "9px 0 3px", fontWeight: 600 };
const row: React.CSSProperties = { display: "flex", alignItems: "center", gap: 6, padding: "2px 2px" };
function Note({ children }: { children: React.ReactNode }) {
  return <div style={{ color: "#7a8aa0", fontSize: 11, padding: "10px 2px" }}>{children}</div>;
}
