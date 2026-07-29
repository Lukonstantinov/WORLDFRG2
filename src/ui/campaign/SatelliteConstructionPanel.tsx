import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetSatellite } from "@bridge";
import type { SatelliteBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { GOOD_DEFS } from "@goods";
import { T, FZ, RADIUS } from "@ui/campaign/chronicleTheme";
import { Panel, Meter } from "@ui/kit";

const STAGE_NAMES = ["Survey", "Foundations", "Warehousing", "Walls", "Market"];
const CAT_ICON = ["🌾", "🧂", "🧱"];
const CAT_COLOR = ["#7fcf6b", "#5ec6e0", "#d9a441"];
const GOOD_EMOJI: Record<string, string> = Object.fromEntries(GOOD_DEFS.map((g) => [g.name, g.emoji]));
const goodIcon = (name: string) => GOOD_EMOJI[name] ?? "📦";

/** Satellite CONSTRUCTION window (Blend V1+V3): a 5-stage bar + monthly cost/runway on
 *  top, the 3 supply tabs (food / preservables / construction) each carrying the convoy
 *  manifest, and the future-exploit goods. Shown while the selected hub is a build site
 *  (build_stage>0); it vanishes on completion and the normal city window takes over.
 *  Built on the shared UI kit (src/ui/kit.tsx). */
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
  const barStyle: React.CSSProperties = { border: `1px solid ${T.line}`, borderRadius: 6 };

  return (
    <Panel onPointerDown={onPointerDown} width={360} maxHeight="90vh" style={{ top: 12, right: 12, zIndex: 120, ...rootStyle }}>
      <div style={{ overflowY: "auto", padding: "10px 12px" }}>
        {/* Header */}
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", cursor: "move" }}
          onPointerDown={onPointerDown}>
          <div>
            <div style={{ fontWeight: 700, color: T.parchment, fontSize: FZ.title }}>
              {brief.name}{" "}
              <span style={satTag}>◮ satellite of {brief.metropolis}</span>
            </div>
            <div style={{ fontSize: FZ.body, color: T.inkDim }}>
              {brief.role} town · broke ground yr {brief.founded_year.toFixed(0)} · ~{brief.eta_years.toFixed(1)} yr to go
            </div>
          </div>
          <span data-no-drag onClick={() => setSelectedHub(null)}
            style={{ color: T.inkDim, cursor: "pointer", fontSize: 18, lineHeight: 1 }} title="Close">×</span>
        </div>

        {/* 5-step stage bar */}
        <div style={{ display: "flex", alignItems: "center", margin: "12px 0 4px" }}>
          {STAGE_NAMES.map((_, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", flex: i < 4 ? 1 : "0 0 auto" }}>
              <div style={{
                width: 15, height: 15, borderRadius: "50%", flex: "0 0 auto",
                border: `2px solid ${i <= stageIdx ? T.accent : "#2a3d52"}`,
                background: i < stageIdx ? T.accent : i === stageIdx ? "#ffd75e" : T.raised,
                boxShadow: i === stageIdx ? "0 0 0 3px rgba(255,215,94,0.18)" : "none",
              }} />
              {i < 4 && <div style={{ height: 3, flex: 1, background: i < stageIdx ? T.accent : "#243650" }} />}
            </div>
          ))}
        </div>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: FZ.micro, color: T.inkDim, marginBottom: 8 }}>
          {STAGE_NAMES.map((s, i) => <span key={i} style={{ color: i === stageIdx ? "#ffd75e" : undefined }}>{s}</span>)}
        </div>
        <Meter value={brief.overall} height={12} track={T.raised}
          color="linear-gradient(90deg,#2f6ea6,#4bc07a)" style={barStyle} />
        <div style={{ ...row, marginTop: 4 }}>
          <span style={{ color: T.inkMid }}>Stage {brief.stage}/5 · {STAGE_NAMES[stageIdx]}</span>
          <span><b>{pct}%</b> built</span>
        </div>

        {/* Cost + runway */}
        <div style={{ ...row }}>
          <span style={{ color: T.inkMid }}>Monthly upkeep</span>
          <span><b>{brief.monthly_cost.toFixed(0)}</b> <span style={muted}>gr-eq · {brief.convoys} convoys</span></span>
        </div>
        <div style={{ ...row }}>
          <span style={{ color: T.inkMid }}>{brief.metropolis} council fund</span>
          <span>{brief.fund.toFixed(0)} <span style={muted}>· ~{brief.runway_months.toFixed(0)} mo runway</span></span>
        </div>

        {/* Supply tabs (Blend V3 manifest inside) */}
        <div style={{ display: "flex", gap: 4, margin: "12px 0 8px" }}>
          {brief.supply.map((s, i) => (
            <div key={i} data-no-drag onClick={() => setTab(i)} style={{
              flex: 1, textAlign: "center", padding: "5px 2px", borderRadius: RADIUS.md, fontSize: FZ.body, cursor: "pointer",
              border: `1px solid ${tab === i ? CAT_COLOR[i] : "#243650"}`,
              color: tab === i ? CAT_COLOR[i] : T.inkMid,
              background: tab === i ? T.accentSoft : T.raised,
            }}>{CAT_ICON[i]} {s.category}</div>
          ))}
        </div>
        {sup && (
          <div style={card}>
            <div style={{ fontSize: FZ.base, fontWeight: 600, color: CAT_COLOR[tab], marginBottom: 6 }}>
              {sup.category} · {goodIcon(sup.good)} {sup.good} <span style={muted}>(from {sup.source})</span>
            </div>
            <div style={{ fontSize: FZ.body, color: T.inkMid, marginBottom: 6 }}>
              {CAT_ICON[tab]} {Math.max(1, Math.round(brief.convoys / 3))} convoy(s) · {sup.rate.toFixed(0)} u/mo quota
            </div>
            <Meter value={sup.met} height={12} track={T.raised} color={CAT_COLOR[tab]} style={barStyle} />
            <div style={{ ...muted, marginTop: 4 }}>
              {Math.round(sup.met * 100)}% of monthly quota delivered{sup.met < 0.8 ? " · shortfall slows the stage" : ""}
            </div>
          </div>
        )}

        {/* Manifest table (all three at a glance) */}
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: FZ.body, marginTop: 4 }}>
          <tbody>
            {brief.supply.map((s, i) => (
              <tr key={i}>
                <td style={tdCell}>{CAT_ICON[i]} {goodIcon(s.good)} {s.good}</td>
                <td style={{ ...tdCell, color: T.inkDim }}>{s.source}</td>
                <td style={{ ...tdCell, textAlign: "right", color: s.met >= 0.8 ? T.good : s.met >= 0.4 ? "#ffd75e" : "#e07a5a" }}>
                  {Math.round(s.met * 100)}%
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        {/* Future exploits */}
        {brief.exploits.length > 0 && (
          <>
            <div style={{ ...row, marginTop: 10, marginBottom: 4 }}><span style={{ color: T.inkMid }}>Future exploits here</span></div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {brief.exploits.map((g, i) => <span key={i} style={exploitChip}>{goodIcon(g)} {g}</span>)}
            </div>
          </>
        )}

        {/* Status / event line */}
        <div style={{
          marginTop: 10, borderRadius: RADIUS.lg, padding: "8px 10px", fontSize: FZ.body,
          background: starved ? "rgba(217,164,65,0.1)" : "rgba(76,174,122,0.1)",
          border: `1px solid ${starved ? "rgba(217,164,65,0.4)" : "rgba(76,174,122,0.4)"}`,
          color: starved ? "#e8cf8a" : T.goodInk,
        }}>
          {starved
            ? `⚠ Supply short — the works have idled ${brief.idle_months} month(s); progress is decaying.`
            : "✔ On schedule — convoys are keeping the site supplied."}
        </div>
      </div>
    </Panel>
  );
}

const satTag: React.CSSProperties = {
  fontSize: FZ.small, color: T.bad, border: `1px solid rgba(192,87,58,0.5)`, background: "rgba(192,87,58,0.12)",
  borderRadius: RADIUS.sm, padding: "1px 6px",
};
const row: React.CSSProperties = { display: "flex", justifyContent: "space-between", alignItems: "center", margin: "9px 0" };
const muted: React.CSSProperties = { color: T.inkDim, fontSize: FZ.body };
const card: React.CSSProperties = { background: T.raised, border: `1px solid ${T.line}`, borderRadius: RADIUS.lg, padding: 10, margin: "8px 0" };
const tdCell: React.CSSProperties = { borderBottom: `1px solid ${T.lineSoft}`, padding: "4px 6px" };
const exploitChip: React.CSSProperties = {
  fontSize: FZ.body, background: T.card, border: `1px dashed ${T.line}`, borderRadius: RADIUS.lg, padding: "4px 9px", color: T.ink,
};
