import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useGoodsStore } from "../state/goodsStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetInequality } from "../bridge/tauri";
import type { InequalitySnapshot } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** #30/#29 · Economy Dashboard.
 *   • Price Index — a basket cost-of-living index per city from the worldgen
 *     market snapshot (price ÷ world base value, weighted toward necessities).
 *   • Inequality — Gini coefficient + a yearly trend, wealth concentration and
 *     house turnover, from the live campaign sim. */
export function EconomyDashboardPanel() {
  const open = useUIStore((s) => s.showEconomyDashboard);
  const economy = useWorldStore((s) => s.economy);
  const specs = useGoodsStore((s) => s.specs);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const active = !!snapshot?.active;
  const tick = snapshot?.clock?.tick ?? 0;

  const [tab, setTab] = useState<"prices" | "ineq">("prices");
  const [ineq, setIneq] = useState<InequalitySnapshot | null>(null);

  useEffect(() => {
    if (!open || tab !== "ineq" || !active) return;
    campaignGetInequality().then(setIneq).catch(() => setIneq(null));
  }, [open, tab, active, tick]);

  // Need-tier weights so the basket leans on staples (0 basic) over luxuries (2).
  const weightOf = useMemo(() => {
    const byId = new Map(specs.map((s) => [s.id, s]));
    return (goodName: string) => {
      const s = byId.get(goodName);
      const tier = s?.need_tier ?? 1;
      return Math.max(1, 3 - tier);
    };
  }, [specs]);

  // Per-city basket index: weighted mean of price/base_value (1.0 = world standard).
  const cities = useMemo(() => {
    if (!economy) return [];
    const out: { name: string; index: number }[] = [];
    for (const h of economy.hubs) {
      const prices = h.market?.prices ?? [];
      let num = 0, den = 0;
      for (const p of prices) {
        if (p.base_value <= 0) continue;
        const w = weightOf(p.good_name);
        num += w * (p.price / p.base_value);
        den += w;
      }
      if (den > 0) out.push({ name: h.name, index: (num / den) * 100 });
    }
    return out.sort((a, b) => a.index - b.index);
  }, [economy, weightOf]);

  const idxLo = cities.length ? cities[0].index : 0;
  const idxHi = cities.length ? cities[cities.length - 1].index : 100;
  const span = Math.max(1, idxHi - idxLo);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.ranking);
  if (!open) return null;
  const close = () => useUIStore.getState().setShowEconomyDashboard(false);

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>📊 Economy Dashboard</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>
      <div style={{ display: "flex", gap: 2, padding: "0 8px", borderBottom: "1px solid #1e2e42" }}>
        {([["prices", "💰 Price Index"], ["ineq", "📊 Inequality"]] as const).map(([id, lbl]) => (
          <div key={id} onClick={() => setTab(id)}
            style={{ padding: "4px 9px", cursor: "pointer", fontSize: 11, fontWeight: tab === id ? 700 : 400,
              color: tab === id ? "#cfe2f6" : "#6a86a6",
              borderBottom: tab === id ? "2px solid #d8b24a" : "2px solid transparent" }}>
            {lbl}
          </div>
        ))}
      </div>

      <div style={{ overflowY: "auto", padding: "8px 10px 12px", maxHeight: "66vh" }}>
        {/* PRICE INDEX */}
        {tab === "prices" && (
          <>
            {!economy && <div style={empty}>Run the Economy step (10) to compute market prices.</div>}
            {economy && cities.length === 0 && <div style={empty}>No market price data yet.</div>}
            {cities.length > 0 && (
              <>
                <div style={{ color: "#8aa0b8", fontSize: 10, marginBottom: 6 }}>
                  Basket cost of living — 100 = the world-standard price. Cheapest cities first.
                </div>
                {cities.map((c) => {
                  const pct = ((c.index - idxLo) / span) * 100;
                  const dear = c.index >= 100;
                  return (
                    <div key={c.name} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
                      <span style={{ width: 96, color: "#bcd0e4", fontSize: 10, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{c.name}</span>
                      <span style={{ flex: 1, height: 8, background: "#0a131d", borderRadius: 3, overflow: "hidden" }}>
                        <span style={{ display: "block", height: "100%", width: `${Math.max(3, pct)}%`,
                          background: dear ? "#d07a5a" : "#5aa0c0" }} />
                      </span>
                      <span style={{ width: 34, textAlign: "right", color: dear ? "#e0a080" : "#9fd0b0", fontSize: 10, fontWeight: 600 }}>
                        {Math.round(c.index)}
                      </span>
                    </div>
                  );
                })}
              </>
            )}
          </>
        )}

        {/* INEQUALITY */}
        {tab === "ineq" && (
          <>
            {!active && <div style={empty}>Begin the campaign (Step 11) and let it run — inequality is read from the living economy.</div>}
            {active && !ineq && <div style={empty}>Reading the houses…</div>}
            {active && ineq && (
              <>
                <div style={{ display: "flex", gap: 10, marginBottom: 10 }}>
                  <Stat label="Gini" value={ineq.gini_now.toFixed(2)} hint="0 equal · 1 concentrated" big />
                  <Stat label="Top 10% hold" value={`${Math.round(ineq.top10_share_now * 100)}%`} hint="of all house wealth" big />
                </div>

                <GiniChart snap={ineq} />

                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 6, marginTop: 10 }}>
                  <Stat label="Active houses" value={String(ineq.active_houses)} />
                  <Stat label="Gone defunct" value={String(ineq.defunct_houses)} />
                  <Stat label="Ever founded" value={String(ineq.founded_total)} />
                </div>

                <div style={{ marginTop: 10 }}>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#8aa0b8" }}>
                    <span>Social mobility (rank churn)</span>
                    <span style={{ color: "#cfe2f6" }}>{Math.round(ineq.rank_churn * 100)}%</span>
                  </div>
                  <div style={{ height: 7, background: "#0a131d", borderRadius: 3, overflow: "hidden", marginTop: 3 }}>
                    <span style={{ display: "block", height: "100%", width: `${Math.max(2, ineq.rank_churn * 100)}%`, background: "#7a9adf" }} />
                  </div>
                  <div style={{ color: "#5a6a80", fontSize: 9, marginTop: 3 }}>
                    How much the wealth pecking order reshuffled since last year — low = entrenched dynasties.
                  </div>
                </div>
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
}

function Stat({ label, value, hint, big }: { label: string; value: string; hint?: string; big?: boolean }) {
  return (
    <div style={{ flex: 1, background: "#0c1622", border: "1px solid #1a2a3e", borderRadius: 6, padding: "6px 8px" }}>
      <div style={{ color: "#8aa0b8", fontSize: 9 }}>{label}</div>
      <div style={{ color: "#e8dcc0", fontWeight: 700, fontSize: big ? 18 : 13, marginTop: 1 }}>{value}</div>
      {hint && <div style={{ color: "#5a6a80", fontSize: 8.5, marginTop: 1 }}>{hint}</div>}
    </div>
  );
}

/** Inline Gini-over-time line (and the top-10% share as a fainter line). */
function GiniChart({ snap }: { snap: InequalitySnapshot }) {
  const s = snap.series;
  if (s.length < 2) {
    return <div style={{ color: "#5a6a80", fontSize: 10, padding: "6px 0" }}>Gini trend appears after a few years of play.</div>;
  }
  const W = 300, H = 96, padL = 24, padR = 8, padT = 8, padB = 16;
  const y0 = snap.series[0].year, y1 = snap.series[s.length - 1].year;
  const xOf = (i: number) => padL + (i / (s.length - 1)) * (W - padL - padR);
  const yOf = (v: number) => padT + (1 - Math.max(0, Math.min(1, v))) * (H - padT - padB);
  const path = (sel: (p: typeof s[number]) => number) =>
    s.map((p, i) => `${i === 0 ? "M" : "L"}${xOf(i).toFixed(1)},${yOf(sel(p)).toFixed(1)}`).join(" ");
  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ background: "#0a131d", border: "1px solid #16243a", borderRadius: 6 }}>
      {[0, 0.5, 1].map((g) => (
        <g key={g}>
          <line x1={padL} x2={W - padR} y1={yOf(g)} y2={yOf(g)} stroke="#16243a" strokeWidth="1" />
          <text x={2} y={yOf(g) + 3} fill="#5a6a80" fontSize="8">{g.toFixed(1)}</text>
        </g>
      ))}
      <path d={path((p) => p.top10_share)} fill="none" stroke="#9a7acf" strokeWidth="1.2" opacity="0.7" strokeDasharray="3 2" />
      <path d={path((p) => p.gini)} fill="none" stroke="#d8b24a" strokeWidth="1.8" />
      <text x={padL} y={H - 4} fill="#6a86a6" fontSize="8">yr {y0}</text>
      <text x={W - padR} y={H - 4} fill="#6a86a6" fontSize="8" textAnchor="end">yr {y1}</text>
      <text x={W - padR} y={padT + 6} fill="#d8b24a" fontSize="8" textAnchor="end">Gini</text>
      <text x={W - padR} y={padT + 15} fill="#9a7acf" fontSize="8" textAnchor="end">top 10%</text>
    </svg>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 332, maxHeight: "82vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 40,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#cfe0f4", fontWeight: 700, fontSize: 12,
};
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "10px 0" };
