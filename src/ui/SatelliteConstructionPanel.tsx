import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetSatellite } from "../bridge/tauri";
import type { SatelliteBrief } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

const STAGE_NAMES = ["Survey", "Foundations", "Warehousing", "Walls", "Market"];
const CAT_ICON = ["🌾", "🧂", "🧱"];
const CAT_COLOR = ["#7fcf6b", "#5ec6e0", "#d9a441"];

/** Satellite CONSTRUCTION window (Blend V1+V3): a 5-stage bar + monthly cost/runway on
 *  top, the 3 supply tabs (food / preservables / construction) each carrying the convoy
 *  manifest, and the future-exploit goods. Shown while the selected hub is a build site
 *  (build_stage>0); it vanishes on completion and the normal city window takes over. */
export function SatelliteConstructionPanel() {
  const selectedHub = useUIStore((s) => s.selectedHub);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.settlement);
  const [brief, setBrief] = useState<SatelliteBrief | null>(null);
  const [tab, setTab] = useState(2);

  // Is the selected hub a construction site? (cheap check from the snapshot marker)
  const isBuild = useMemo(() => {
    if (selectedHub == null || !snapshot?.active) return false;
    const h = snapshot.hubs.find((x) => x.id === selectedHub);
    return !!h && (h.build_stage ?? 0) > 0;
  }, [selectedHub, snapshot]);

  useEffect(() => {
    let alive = true;
    if (!isBuild || selectedHub == null) { setBrief(null); return; }
    campaignGetSatellite(selectedHub).then((b) => { if (alive) setBrief(b); }).catch(() => { if (alive) setBrief(null); });
    return () => { alive = false; };
  }, [isBuild, selectedHub, tick]);

  if (!brief) return null;

  const stageIdx = Math.min(Math.max(brief.stage - 1, 0), 4);
  const pct = Math.round(brief.overall * 100);
  const sup = brief.supply[tab] ?? brief.supply[0];
  const starved = brief.idle_months > 0;

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      {/* Header */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", cursor: "move" }}
        onPointerDown={onPointerDown}>
        <div>
          <div style={{ fontWeight: 700, color: "#fff", fontSize: 15 }}>
            {brief.name}{" "}
            <span style={satTag}>◮ satellite of {brief.metropolis}</span>
          </div>
          <div style={{ fontSize: 11, color: "#6a86a6" }}>
            {brief.role} town · broke ground yr {brief.founded_year.toFixed(0)} · ~{brief.eta_years.toFixed(1)} yr to go
          </div>
        </div>
        <span data-no-drag onClick={() => setSelectedHub(null)}
          style={{ color: "#7090b0", cursor: "pointer", fontSize: 18, lineHeight: 1 }} title="Close">×</span>
      </div>

      {/* 5-step stage bar */}
      <div style={{ display: "flex", alignItems: "center", margin: "12px 0 4px" }}>
        {STAGE_NAMES.map((_, i) => (
          <div key={i} style={{ display: "flex", alignItems: "center", flex: i < 4 ? 1 : "0 0 auto" }}>
            <div style={{
              width: 15, height: 15, borderRadius: "50%", flex: "0 0 auto",
              border: `2px solid ${i <= stageIdx ? "#3a80c0" : "#2a3d52"}`,
              background: i < stageIdx ? "#3a80c0" : i === stageIdx ? "#ffd75e" : "#0e1a27",
              boxShadow: i === stageIdx ? "0 0 0 3px rgba(255,215,94,0.18)" : "none",
            }} />
            {i < 4 && <div style={{ height: 3, flex: 1, background: i < stageIdx ? "#3a80c0" : "#243650" }} />}
          </div>
        ))}
      </div>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 8.5, color: "#5a7391", marginBottom: 8 }}>
        {STAGE_NAMES.map((s, i) => <span key={i} style={{ color: i === stageIdx ? "#ffd75e" : undefined }}>{s}</span>)}
      </div>
      <div style={barOuter}><div style={{ ...barInner, width: `${pct}%` }} /></div>
      <div style={{ ...row, marginTop: 4 }}>
        <span style={{ color: "#9ab0c8" }}>Stage {brief.stage}/5 · {STAGE_NAMES[stageIdx]}</span>
        <span><b>{pct}%</b> built</span>
      </div>

      {/* Cost + runway */}
      <div style={{ ...row }}>
        <span style={{ color: "#9ab0c8" }}>Monthly upkeep</span>
        <span><b>{brief.monthly_cost.toFixed(0)}</b> <span style={muted}>gr-eq · {brief.convoys} convoys</span></span>
      </div>
      <div style={{ ...row }}>
        <span style={{ color: "#9ab0c8" }}>{brief.metropolis} council fund</span>
        <span>{brief.fund.toFixed(0)} <span style={muted}>· ~{brief.runway_months.toFixed(0)} mo runway</span></span>
      </div>

      {/* Supply tabs (Blend V3 manifest inside) */}
      <div style={{ display: "flex", gap: 4, margin: "12px 0 8px" }}>
        {brief.supply.map((s, i) => (
          <div key={i} onClick={() => setTab(i)} style={{
            flex: 1, textAlign: "center", padding: "5px 2px", borderRadius: 6, fontSize: 11, cursor: "pointer",
            border: `1px solid ${tab === i ? CAT_COLOR[i] : "#243650"}`,
            color: tab === i ? CAT_COLOR[i] : "#8ea6c0",
            background: tab === i ? "#16324a" : "#0e1a27",
          }}>{CAT_ICON[i]} {s.category}</div>
        ))}
      </div>
      {sup && (
        <div style={card}>
          <div style={{ fontSize: 12, fontWeight: 600, color: CAT_COLOR[tab], marginBottom: 6 }}>
            {sup.category} · {sup.good} <span style={muted}>(from {sup.source})</span>
          </div>
          <div style={{ fontSize: 11, color: "#9ab0c8", marginBottom: 6 }}>
            {CAT_ICON[tab]} {Math.max(1, Math.round(brief.convoys / 3))} convoy(s) · {sup.rate.toFixed(0)} u/mo quota
          </div>
          <div style={barOuter}>
            <div style={{ ...barInner, width: `${Math.round(sup.met * 100)}%`, background: CAT_COLOR[tab] }} />
          </div>
          <div style={{ ...muted, marginTop: 4 }}>
            {Math.round(sup.met * 100)}% of monthly quota delivered{sup.met < 0.8 ? " · shortfall slows the stage" : ""}
          </div>
        </div>
      )}

      {/* Manifest table (all three at a glance) */}
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11, marginTop: 4 }}>
        <tbody>
          {brief.supply.map((s, i) => (
            <tr key={i}>
              <td style={tdCell}>{CAT_ICON[i]} {s.good}</td>
              <td style={{ ...tdCell, color: "#6a86a6" }}>{s.source}</td>
              <td style={{ ...tdCell, textAlign: "right", color: s.met >= 0.8 ? "#4bc07a" : s.met >= 0.4 ? "#ffd75e" : "#e07a5a" }}>
                {Math.round(s.met * 100)}%
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {/* Future exploits */}
      {brief.exploits.length > 0 && (
        <>
          <div style={{ ...row, marginTop: 10, marginBottom: 4 }}><span style={{ color: "#9ab0c8" }}>Future exploits here</span></div>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            {brief.exploits.map((g, i) => <span key={i} style={exploitChip}>{g}</span>)}
          </div>
        </>
      )}

      {/* Status / event line */}
      <div style={{
        marginTop: 10, borderRadius: 8, padding: "8px 10px", fontSize: 11,
        background: starved ? "#1c1408" : "#0c1e14",
        border: `1px solid ${starved ? "#4a3a12" : "#1d4a30"}`,
        color: starved ? "#e8cf8a" : "#8fe0aa",
      }}>
        {starved
          ? `⚠ Supply short — the works have idled ${brief.idle_months} month(s); progress is decaying.`
          : "✔ On schedule — convoys are keeping the site supplied."}
      </div>
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 12, right: 12, width: 360, maxHeight: "90vh", overflowY: "auto",
  border: "1px solid #24364e", borderRadius: 8, padding: "10px 12px", zIndex: 120,
  boxShadow: "0 8px 30px rgba(0,0,0,0.5)", color: "#cfe2f6",
  fontFamily: '"Segoe UI",system-ui,sans-serif', fontSize: 13,
};
const satTag: React.CSSProperties = {
  fontSize: 10, color: "#e0503a", border: "1px solid #5a2a22", background: "#20120e",
  borderRadius: 4, padding: "1px 6px",
};
const row: React.CSSProperties = { display: "flex", justifyContent: "space-between", alignItems: "center", margin: "9px 0" };
const muted: React.CSSProperties = { color: "#5a7391", fontSize: 11 };
const barOuter: React.CSSProperties = { height: 12, borderRadius: 6, background: "#0e1a27", border: "1px solid #243650", overflow: "hidden" };
const barInner: React.CSSProperties = { height: "100%", background: "linear-gradient(90deg,#2f6ea6,#4bc07a)" };
const card: React.CSSProperties = { background: "#0e1a27", border: "1px solid #243650", borderRadius: 8, padding: 10, margin: "8px 0" };
const tdCell: React.CSSProperties = { borderBottom: "1px solid #1c2c40", padding: "4px 6px" };
const exploitChip: React.CSSProperties = {
  fontSize: 11, background: "#0c1826", border: "1px dashed #243650", borderRadius: 8, padding: "4px 9px", color: "#cfe2f6",
};
