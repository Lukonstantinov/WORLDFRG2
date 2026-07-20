import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignCityRanking } from "@bridge";
import type { CityRank } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";
import { T, FZ } from "./chronicleTheme";
import { Panel, PanelHeader, PanelBody, Meter, EmptyNote, FootNote } from "./kit";

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

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.ranking);
  if (!open) return null;
  const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));
  const maxTrade = Math.max(1e-6, ...rows.map((r) => r.trade));

  return (
    <Panel width={300} style={{ top: 60, right: 360, zIndex: 42, ...rootStyle }}>
      <PanelHeader icon="🏆" title="Richest Cities" onDragStart={onPointerDown} onClose={() => setOpen(false)} />
      <PanelBody style={{ padding: "4px 8px 10px" }}>
        {!active && <EmptyNote>Begin the campaign (Step 11) to rank cities by trade.</EmptyNote>}
        {active && rows.length === 0 && <EmptyNote>No trade yet.</EmptyNote>}
        {rows.map((c, i) => (
          <div key={c.id} onClick={() => { setSelectedHub(c.id); }}
            title="Open this city's settlement view"
            style={{ display: "flex", alignItems: "center", gap: 6, padding: "3px 2px", borderBottom: `1px solid ${T.lineSoft}`, cursor: "pointer" }}>
            <span style={{ color: T.inkDim, fontSize: FZ.small, minWidth: 18, textAlign: "right" }}>#{i + 1}</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
                <span style={{ color: T.parchment, fontSize: FZ.body, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{c.name}</span>
                <span style={{ color: T.inkMid, fontSize: FZ.tiny }}>{c.population.toLocaleString()}</span>
                <span style={{ flex: 1 }} />
                <span style={{ color: T.goodInk, fontSize: FZ.small, fontWeight: 700 }}>{c.pct_world.toFixed(1)}%</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
                <Meter value={c.trade} max={maxTrade} color={T.gold} />
                <span style={{ color: T.inkMid, fontSize: FZ.tiny, minWidth: 56, textAlign: "right" }} title="trade value moved">
                  💰 {fmt(c.trade)}
                </span>
              </div>
            </div>
          </div>
        ))}
        {rows.length > 0 && (
          <FootNote>% = share of all world trade · 💰 = trade value moved</FootNote>
        )}
      </PanelBody>
    </Panel>
  );
}
