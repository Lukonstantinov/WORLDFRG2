import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetFigures } from "../bridge/tauri";
import type { FigureBrief } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** Phase 6 · Notable Figures — the campaign's Great Lives (admirals, demagogues,
 *  master craftsmen, bankers, explorers), living first, filterable by role. Clicking
 *  a figure focuses their city; a map toggle marks the living. */

const ROLE_EMOJI: Record<string, string> = {
  "Admiral": "⚓", "Demagogue": "📢", "Master Craftsman": "⚒", "Great Banker": "🏦", "Explorer": "🧭",
};
const ROLES = ["All", "Admiral", "Demagogue", "Master Craftsman", "Great Banker", "Explorer"];

export function FiguresPanel() {
  const open = useUIStore((s) => s.showFigures);
  const close = () => useUIStore.getState().setShowFigures(false);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const showMarks = useUIStore((s) => s.overlayVisibility.figureMarks);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [rows, setRows] = useState<FigureBrief[]>([]);
  const [role, setRole] = useState("All");
  const [livingOnly, setLivingOnly] = useState(false);

  useEffect(() => {
    if (!open || !active) return;
    campaignGetFigures().then(setRows).catch(() => setRows([]));
  }, [open, active, tick]);

  const shown = useMemo(() => {
    let r = rows;
    if (role !== "All") r = r.filter((f) => f.role === role);
    if (livingOnly) r = r.filter((f) => f.alive);
    return r.slice(0, 200);
  }, [rows, role, livingOnly]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.figures);
  if (!open) return null;

  const living = rows.filter((f) => f.alive).length;
  const focus = (hubIdx: number) => {
    const id = snapshot?.hubs?.[hubIdx]?.id;
    if (id != null) setSelectedHub(id);
  };

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ display: "flex", alignItems: "center", marginBottom: 6, cursor: "move" }} onPointerDown={onPointerDown}>
        <span style={{ color: "#d8c8f0", fontWeight: 700, fontSize: 13 }}>⚜️ Notable Figures</span>
        <span style={{ flex: 1 }} />
        <span data-no-drag onClick={close} title="Close" style={{ color: "#8a80a0", cursor: "pointer", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>

      <div style={{ display: "flex", gap: 8, marginBottom: 6, fontSize: 10, color: "#9088a0" }}>
        <span>{rows.length} figures</span><span>· {living} living</span>
        <span style={{ flex: 1 }} />
        <label style={{ display: "flex", alignItems: "center", gap: 3, cursor: "pointer" }} data-no-drag>
          <input type="checkbox" checked={showMarks} onChange={(e) => setOverlayVisible("figureMarks", e.target.checked)}
            style={{ accentColor: "#9070c0", width: 11, height: 11 }} />
          <span>Show on map</span>
        </label>
      </div>

      <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 6 }} data-no-drag>
        {ROLES.map((r) => (
          <span key={r} onClick={() => setRole(r)}
            style={{ fontSize: 10, padding: "2px 8px", borderRadius: 20, cursor: "pointer",
              border: `1px solid ${role === r ? "#9070c0" : "#2e2640"}`,
              background: role === r ? "#241a34" : "transparent",
              color: role === r ? "#e0d0f4" : "#9088a0" }}>
            {r === "All" ? "All" : (ROLE_EMOJI[r] ?? "") + " " + r.split(" ")[0]}
          </span>
        ))}
        <span onClick={() => setLivingOnly((v) => !v)}
          style={{ fontSize: 10, padding: "2px 8px", borderRadius: 20, cursor: "pointer",
            border: `1px solid ${livingOnly ? "#9070c0" : "#2e2640"}`,
            background: livingOnly ? "#241a34" : "transparent",
            color: livingOnly ? "#e0d0f4" : "#9088a0" }}>Living</span>
      </div>

      <div style={{ overflowY: "auto", flex: 1 }}>
        {!active && <div style={empty}>Start the campaign to see notable figures.</div>}
        {active && shown.length === 0 && <div style={empty}>No figures yet — advance time.</div>}
        {shown.map((f, i) => (
          <div key={i} onClick={() => focus(f.hub)}
            style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 2px", borderBottom: "1px dashed #201a2c", cursor: "pointer" }}>
            <span style={{ fontSize: 15, width: 20, textAlign: "center", opacity: f.alive ? 1 : 0.5 }}>{ROLE_EMOJI[f.role] ?? "•"}</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ color: f.alive ? "#e0d4f4" : "#9c94ac", fontSize: 11 }}>
                {f.name} {!f.alive && <span style={{ color: "#6a6480", fontSize: 9 }}>†</span>}
              </div>
              <div style={{ color: "#8880a0", fontSize: 9 }}>
                {f.role}{f.good_name ? ` · ${f.good_name}` : ""} · {f.city}
              </div>
            </div>
            <div style={{ color: "#9088b0", fontSize: 9, textAlign: "right" }}>
              {f.alive ? `b. ${f.born_year}` : `${f.born_year}–${f.died_year}`}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 70, right: 12, width: 320, maxHeight: "72%",
  display: "flex", flexDirection: "column",
  border: "1px solid #2e2640", borderRadius: 8, padding: "10px 12px", zIndex: 117,
  boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
const empty: React.CSSProperties = { color: "#8880a0", fontSize: 11, padding: "8px 2px" };
