import { useEffect, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignCityRanking } from "../bridge/tauri";
import type { CityRank } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** Live "Richest Cities" ranking — the busiest trading cities top to bottom, with
 *  each one's share of ALL world trade. Click a city to open its settlement view. */
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
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>🏆 Richest Cities</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={() => setOpen(false)}>✕</span>
      </div>
      <div style={{ overflowY: "auto", padding: "4px 8px 10px" }}>
        {!active && <div style={{ color: "#506080", fontSize: 11, padding: 10 }}>Begin the campaign (Step 11) to rank cities by trade.</div>}
        {active && rows.length === 0 && <div style={{ color: "#506080", fontSize: 11, padding: 10 }}>No trade yet.</div>}
        {rows.map((c, i) => (
          <div key={c.id} onClick={() => { setSelectedHub(c.id); }}
            title="Open this city's settlement view"
            style={{ display: "flex", alignItems: "center", gap: 6, padding: "3px 2px", borderBottom: "1px solid #131e2a", cursor: "pointer" }}>
            <span style={{ color: "#6a86a6", fontSize: 10, minWidth: 18, textAlign: "right" }}>#{i + 1}</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
                <span style={{ color: "#e8d8b0", fontSize: 11, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{c.name}</span>
                <span style={{ color: "#7a90a8", fontSize: 9 }}>{c.population.toLocaleString()}</span>
                <span style={{ flex: 1 }} />
                <span style={{ color: "#7fd0a0", fontSize: 10, fontWeight: 700 }}>{c.pct_world.toFixed(1)}%</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
                <div style={{ flex: 1, height: 4, background: "#0a1018", borderRadius: 3, overflow: "hidden" }}>
                  <div style={{ width: `${(c.trade / maxTrade) * 100}%`, height: "100%", background: "#c9a227" }} />
                </div>
                <span style={{ color: "#9ab0c8", fontSize: 9, minWidth: 56, textAlign: "right" }} title="trade value moved">
                  💰 {fmt(c.trade)}
                </span>
              </div>
            </div>
          </div>
        ))}
        {rows.length > 0 && (
          <div style={{ color: "#56708e", fontSize: 8, marginTop: 4 }}>% = share of all world trade · 💰 = trade value moved</div>
        )}
      </div>
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 300, maxHeight: "78vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 42,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#cfe0f4", fontWeight: 700, fontSize: 12,
};
