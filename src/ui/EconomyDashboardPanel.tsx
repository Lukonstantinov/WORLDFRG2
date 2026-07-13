import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useGoodsStore } from "../state/goodsStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetInequality, campaignCityPriceIndex } from "../bridge/tauri";
import type { InequalitySnapshot, CityPriceIndex } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";
import { T, FZ } from "./chronicleTheme";
import { Panel, PanelHeader, Tabs, Meter, EmptyNote } from "./kit";

/** #30/#29 · Economy Dashboard.
 *   • Price Index — a basket cost-of-living index per city from the worldgen
 *     market snapshot (price ÷ world base value, weighted toward necessities).
 *   • Inequality — Gini coefficient + a yearly trend, wealth concentration and
 *     house turnover, from the live campaign sim.
 *   Built on the shared UI kit (src/ui/kit.tsx). */
export function EconomyDashboardPanel() {
  const open = useUIStore((s) => s.showEconomyDashboard);
  const economy = useWorldStore((s) => s.economy);
  const specs = useGoodsStore((s) => s.specs);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const active = !!snapshot?.active;
  const tick = snapshot?.clock?.tick ?? 0;
  const year = snapshot?.clock?.year ?? 0;

  const [tab, setTab] = useState<"prices" | "ineq">("prices");
  const [ineq, setIneq] = useState<InequalitySnapshot | null>(null);
  // Live per-city basket from the running sim, refreshed once per campaign YEAR so
  // the Prices tab tracks the living economy instead of the frozen worldgen snapshot.
  const [liveCities, setLiveCities] = useState<CityPriceIndex[]>([]);

  useEffect(() => {
    if (!open || tab !== "ineq" || !active) return;
    campaignGetInequality().then(setIneq).catch(() => setIneq(null));
  }, [open, tab, active, tick]);

  useEffect(() => {
    if (!open || tab !== "prices" || !active) { setLiveCities([]); return; }
    let alive = true;
    campaignCityPriceIndex().then((c) => { if (alive) setLiveCities(c); }).catch(() => { if (alive) setLiveCities([]); });
    return () => { alive = false; };
  }, [open, tab, active, year]);

  // Need-tier weights so the basket leans on staples (0 basic) over luxuries (2).
  const weightOf = useMemo(() => {
    const byId = new Map(specs.map((s) => [s.id, s]));
    return (goodName: string) => {
      const s = byId.get(goodName);
      const tier = s?.need_tier ?? 1;
      return Math.max(1, 3 - tier);
    };
  }, [specs]);

  // Per-city basket index from the STATIC worldgen snapshot (pre-campaign fallback).
  const staticCities = useMemo(() => {
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

  // Live campaign baskets (refreshed yearly) take precedence; fall back to the static
  // worldgen snapshot before/without a campaign.
  const cities = active && liveCities.length > 0
    ? [...liveCities].sort((a, b) => a.index - b.index)
    : staticCities;

  const idxLo = cities.length ? cities[0].index : 0;
  const idxHi = cities.length ? cities[cities.length - 1].index : 100;
  const span = Math.max(1, idxHi - idxLo);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.ranking);
  if (!open) return null;
  const close = () => useUIStore.getState().setShowEconomyDashboard(false);

  return (
    <Panel width={332} maxHeight="82vh" style={{ top: 60, right: 360, zIndex: 40, ...rootStyle }}>
      <PanelHeader icon="📊" title="Economy Dashboard" onDragStart={onPointerDown} onClose={close} />
      <Tabs<"prices" | "ineq">
        tabs={[["prices", "💰 Price Index"], ["ineq", "📊 Inequality"]]}
        active={tab} onSelect={setTab} style={{ padding: "0 8px" }}
      />

      <div style={{ overflowY: "auto", padding: "8px 10px 12px", maxHeight: "66vh" }}>
        {/* PRICE INDEX */}
        {tab === "prices" && (
          <>
            {!economy && !active && <EmptyNote>Run the Economy step (10) to compute market prices.</EmptyNote>}
            {(economy || active) && cities.length === 0 && <EmptyNote>No market price data yet.</EmptyNote>}
            {cities.length > 0 && (
              <>
                <div style={{ color: T.inkMid, fontSize: FZ.small, marginBottom: 6 }}>
                  Basket cost of living — 100 = the world-standard price. Cheapest cities first.
                  {active && liveCities.length > 0
                    ? <span style={{ color: T.goodInk }}> · live (year {year})</span>
                    : <span style={{ color: T.inkDim }}> · worldgen snapshot</span>}
                </div>
                {cities.map((c) => {
                  const pct = ((c.index - idxLo) / span) * 100;
                  const dear = c.index >= 100;
                  return (
                    <div key={c.name} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
                      <span style={{ width: 96, color: T.inkMid, fontSize: FZ.small, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{c.name}</span>
                      <Meter value={Math.max(3, pct)} max={100} height={8} color={dear ? "#d07a5a" : "#5aa0c0"} />
                      <span style={{ width: 34, textAlign: "right", color: dear ? T.warn : T.goodInk, fontSize: FZ.small, fontWeight: 600 }}>
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
            {!active && <EmptyNote>Begin the campaign (Step 11) and let it run — inequality is read from the living economy.</EmptyNote>}
            {active && !ineq && <EmptyNote>Reading the houses…</EmptyNote>}
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
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: FZ.small, color: T.inkMid }}>
                    <span>Social mobility (rank churn)</span>
                    <span style={{ color: T.ink }}>{Math.round(ineq.rank_churn * 100)}%</span>
                  </div>
                  <div style={{ marginTop: 3 }}>
                    <Meter value={Math.max(2, ineq.rank_churn * 100)} max={100} height={7} color="#7a9adf" />
                  </div>
                  <div style={{ color: T.inkDim, fontSize: FZ.tiny, marginTop: 3 }}>
                    How much the wealth pecking order reshuffled since last year — low = entrenched dynasties.
                  </div>
                </div>
              </>
            )}
          </>
        )}
      </div>
    </Panel>
  );
}

function Stat({ label, value, hint, big }: { label: string; value: string; hint?: string; big?: boolean }) {
  return (
    <div style={{ flex: 1, background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 6, padding: "6px 8px" }}>
      <div style={{ color: T.inkMid, fontSize: FZ.tiny }}>{label}</div>
      <div style={{ color: T.parchment, fontWeight: 700, fontSize: big ? 18 : FZ.head, marginTop: 1 }}>{value}</div>
      {hint && <div style={{ color: T.inkDim, fontSize: FZ.micro, marginTop: 1 }}>{hint}</div>}
    </div>
  );
}

/** Inline Gini-over-time line (and the top-10% share as a fainter line). */
function GiniChart({ snap }: { snap: InequalitySnapshot }) {
  const s = snap.series;
  if (s.length < 2) {
    return <div style={{ color: T.inkDim, fontSize: FZ.small, padding: "6px 0" }}>Gini trend appears after a few years of play.</div>;
  }
  const W = 300, H = 96, padL = 24, padR = 8, padT = 8, padB = 16;
  const y0 = snap.series[0].year, y1 = snap.series[s.length - 1].year;
  const xOf = (i: number) => padL + (i / (s.length - 1)) * (W - padL - padR);
  const yOf = (v: number) => padT + (1 - Math.max(0, Math.min(1, v))) * (H - padT - padB);
  const path = (sel: (p: typeof s[number]) => number) =>
    s.map((p, i) => `${i === 0 ? "M" : "L"}${xOf(i).toFixed(1)},${yOf(sel(p)).toFixed(1)}`).join(" ");
  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ background: T.card, border: `1px solid ${T.line}`, borderRadius: 6 }}>
      {[0, 0.5, 1].map((g) => (
        <g key={g}>
          <line x1={padL} x2={W - padR} y1={yOf(g)} y2={yOf(g)} stroke={T.line} strokeWidth="1" />
          <text x={2} y={yOf(g) + 3} fill={T.inkDim} fontSize="8">{g.toFixed(1)}</text>
        </g>
      ))}
      <path d={path((p) => p.top10_share)} fill="none" stroke="#9a7acf" strokeWidth="1.2" opacity="0.7" strokeDasharray="3 2" />
      <path d={path((p) => p.gini)} fill="none" stroke={T.gold} strokeWidth="1.8" />
      <text x={padL} y={H - 4} fill={T.inkDim} fontSize="8">yr {y0}</text>
      <text x={W - padR} y={H - 4} fill={T.inkDim} fontSize="8" textAnchor="end">yr {y1}</text>
      <text x={W - padR} y={padT + 6} fill={T.gold} fontSize="8" textAnchor="end">Gini</text>
      <text x={W - padR} y={padT + 15} fill="#9a7acf" fontSize="8" textAnchor="end">top 10%</text>
    </svg>
  );
}
