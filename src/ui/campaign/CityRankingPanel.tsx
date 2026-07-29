import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignCityRanking } from "@bridge";
import type { CityRank } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Meter, EmptyNote, FootNote } from "@ui/kit";

/** Live "Richest Cities" ranking — the busiest trading cities top to bottom, with
 *  each one's share of ALL world trade. Click a city to open its settlement view.
 *  Reference adoption of the shared UI kit (src/ui/kit.tsx). */
export function CityRankingPanel() {
  const open = useUIStore((s) => s.showCityRanking);
  const setOpen = useUIStore((s) => s.setShowCityRanking);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const active = useCampaignStore((s) => s.snapshot?.active ?? false);
  const tick = useCampaignStore((s) => s.snapshot?.clock.tick ?? 0);
  const [rows, setRows] = useState<CityRank[]>([]);

  useEffect(() => {
    if (!open || !active) return;
    let alive = true;
    campaignCityRanking().then((r) => { if (alive) setRows(r); }).catch(() => {});
    return () => { alive = false; };
  }, [open, active, tick]);

  const [mode, setMode] = useState<"prosperity" | "trade">("prosperity");
  const { rootStyle, onPointerDown, dragRoot } = useFloatingWindow(PANEL_TINTS.ranking);
  if (!open) return null;
  const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));
  const maxTrade = Math.max(1e-6, ...rows.map((r) => r.trade));
  const maxProsp = Math.max(1e-6, ...rows.map((r) => r.prosperity));
  // C1 · rank by the honest prosperity composite, or by trade volume (busiest).
  const ranked = [...rows].sort((a, b) =>
    mode === "prosperity" ? b.prosperity - a.prosperity : b.trade - a.trade);
  const chip = (on: boolean): React.CSSProperties => ({
    fontSize: FZ.small, padding: "2px 8px", borderRadius: 4, cursor: "pointer",
    border: `1px solid ${on ? "#3a80c0" : T.lineSoft}`, background: on ? "#19324a" : "transparent",
    color: on ? "#cfe2f6" : T.inkMid,
  });

  return (
    <Panel {...dragRoot} width={300} style={{ top: 60, right: 360, zIndex: 42, ...rootStyle }}>
      <PanelHeader icon="🏆" title={mode === "prosperity" ? "Most Prosperous Cities" : "Busiest by Trade"}
        onDragStart={onPointerDown} onClose={() => setOpen(false)} />
      <PanelBody style={{ padding: "4px 8px 10px" }}>
        <div style={{ display: "flex", gap: 5, marginBottom: 6 }}>
          <span onClick={() => setMode("prosperity")} style={chip(mode === "prosperity")} title="A composite of trade, public treasury, common wealth and equity">✦ Prosperity</span>
          <span onClick={() => setMode("trade")} style={chip(mode === "trade")} title="Ranked purely by trade volume moved">💰 Busiest</span>
        </div>
        {!active && <EmptyNote>Begin the campaign (Step 11) to rank cities.</EmptyNote>}
        {active && ranked.length === 0 && <EmptyNote>No trade yet.</EmptyNote>}
        {ranked.map((c, i) => (
          <div key={c.id} onClick={() => { setSelectedHub(c.id); }}
            title="Open this city's settlement view"
            style={{ display: "flex", alignItems: "center", gap: 6, padding: "3px 2px", borderBottom: `1px solid ${T.lineSoft}`, cursor: "pointer" }}>
            <span style={{ color: T.inkDim, fontSize: FZ.small, minWidth: 18, textAlign: "right" }}>#{i + 1}</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
                <span style={{ color: T.parchment, fontSize: FZ.body, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{c.name}</span>
                <span style={{ color: T.inkMid, fontSize: FZ.tiny }}>{c.population.toLocaleString()}</span>
                <span style={{ flex: 1 }} />
                {mode === "prosperity"
                  ? <span style={{ color: T.goodInk, fontSize: FZ.small, fontWeight: 700 }} title="Prosperity index (0–100)">{(c.prosperity / maxProsp * 100).toFixed(0)}</span>
                  : <span style={{ color: T.goodInk, fontSize: FZ.small, fontWeight: 700 }}>{c.pct_world.toFixed(1)}%</span>}
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
                {mode === "prosperity" ? (
                  <>
                    <Meter value={c.prosperity} max={maxProsp} color={T.gold} />
                    <span style={{ color: T.inkMid, fontSize: FZ.tiny, minWidth: 74, textAlign: "right" }}
                      title={`treasury ${fmt(c.treasury)} · common wealth ${c.commoner_wealth.toFixed(2)} · inequality ${(c.inequality * 100).toFixed(0)}%`}>
                      🏛{fmt(c.treasury)} ⚖{(1 - c.inequality > 0 ? (1 - c.inequality) * 100 : 0).toFixed(0)}
                    </span>
                  </>
                ) : (
                  <>
                    <Meter value={c.trade} max={maxTrade} color={T.gold} />
                    <span style={{ color: T.inkMid, fontSize: FZ.tiny, minWidth: 56, textAlign: "right" }} title="trade value moved">
                      💰 {fmt(c.trade)}
                    </span>
                  </>
                )}
              </div>
            </div>
          </div>
        ))}
        {ranked.length > 0 && (
          <FootNote>{mode === "prosperity"
            ? "Prosperity = trade + treasury + common wealth + equity · 🏛 treasury · ⚖ equity (100−inequality)"
            : "% = share of all world trade · 💰 = trade value moved"}</FootNote>
        )}
      </PanelBody>
    </Panel>
  );
}
