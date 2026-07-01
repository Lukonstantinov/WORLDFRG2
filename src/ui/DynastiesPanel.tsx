import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetDynasties } from "../bridge/tauri";
import type { DynastiesPayload, HouseLink } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** Phase 7 · Dynasties & Alliances — marriage alliances (💍) and feuds (🗡) between
 *  the great houses. Click a house to focus its seat; a map toggle draws the ties. */

type Tab = "alliances" | "feuds";

export function DynastiesPanel() {
  const open = useUIStore((s) => s.showDynasties);
  const close = () => useUIStore.getState().setShowDynasties(false);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const showLinks = useUIStore((s) => s.overlayVisibility.dynastyLinks);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [data, setData] = useState<DynastiesPayload>({ alliances: [], feuds: [] });
  const [tab, setTab] = useState<Tab>("alliances");

  useEffect(() => {
    if (!open || !active) return;
    campaignGetDynasties().then(setData).catch(() => setData({ alliances: [], feuds: [] }));
  }, [open, active, tick]);

  const rows = useMemo(() => (tab === "alliances" ? data.alliances : data.feuds), [data, tab]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.dynasties);
  if (!open) return null;

  const focus = (hubIdx: number) => {
    const id = snapshot?.hubs?.[hubIdx]?.id;
    if (id != null) setSelectedHub(id);
  };
  const stripHouse = (n: string) => n.replace(/^House /, "");

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ display: "flex", alignItems: "center", marginBottom: 6, cursor: "move" }} onPointerDown={onPointerDown}>
        <span style={{ color: "#e8c0c8", fontWeight: 700, fontSize: 13 }}>⚭ Dynasties &amp; Alliances</span>
        <span style={{ flex: 1 }} />
        <span data-no-drag onClick={close} title="Close" style={{ color: "#a07884", cursor: "pointer", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>

      <div style={{ display: "flex", gap: 8, marginBottom: 6, fontSize: 10, color: "#9c8088" }}>
        <span>{data.alliances.length} alliance{data.alliances.length === 1 ? "" : "s"}</span>
        <span>· {data.feuds.length} feud{data.feuds.length === 1 ? "" : "s"}</span>
        <span style={{ flex: 1 }} />
        <label style={{ display: "flex", alignItems: "center", gap: 3, cursor: "pointer" }} data-no-drag>
          <input type="checkbox" checked={showLinks} onChange={(e) => setOverlayVisible("dynastyLinks", e.target.checked)}
            style={{ accentColor: "#c06078", width: 11, height: 11 }} />
          <span>Show on map</span>
        </label>
      </div>

      <div style={{ display: "flex", gap: 4, marginBottom: 6 }} data-no-drag>
        {(["alliances", "feuds"] as Tab[]).map((t) => (
          <span key={t} onClick={() => setTab(t)}
            style={{ fontSize: 10, padding: "2px 10px", borderRadius: 20, cursor: "pointer", textTransform: "capitalize",
              border: `1px solid ${tab === t ? "#c06078" : "#3a2830"}`,
              background: tab === t ? "#2a1820" : "transparent",
              color: tab === t ? "#f0ccd4" : "#9c8088" }}>
            {t === "alliances" ? "💍 Alliances" : "🗡 Feuds"}
          </span>
        ))}
      </div>

      <div style={{ overflowY: "auto", flex: 1 }}>
        {!active && <div style={empty}>Start the campaign to trace the dynasties.</div>}
        {active && rows.length === 0 && (
          <div style={empty}>{tab === "alliances" ? "No marriage alliances yet." : "No active feuds."}</div>
        )}
        {rows.map((l: HouseLink, i) => (
          <div key={i} style={{ display: "flex", alignItems: "center", gap: 6, padding: "5px 2px", borderBottom: "1px dashed #241820", fontSize: 11 }}>
            <span style={{ width: 16, textAlign: "center" }}>{tab === "alliances" ? "💍" : "🗡"}</span>
            <span onClick={() => focus(l.a_hub)} title={l.a_city}
              style={{ color: "#e0c0c8", cursor: "pointer", flex: 1, textAlign: "right", overflow: "hidden", textOverflow: "ellipsis" }}>
              {stripHouse(l.a_name)}
            </span>
            <span style={{ color: tab === "alliances" ? "#d8b060" : "#c06060", fontSize: 12 }}>
              {tab === "alliances" ? "⚭" : "⚔"}
            </span>
            <span onClick={() => focus(l.b_hub)} title={l.b_city}
              style={{ color: "#e0c0c8", cursor: "pointer", flex: 1, overflow: "hidden", textOverflow: "ellipsis" }}>
              {stripHouse(l.b_name)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 70, right: 12, width: 330, maxHeight: "72%",
  display: "flex", flexDirection: "column",
  border: "1px solid #3a2830", borderRadius: 8, padding: "10px 12px", zIndex: 117,
  boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
const empty: React.CSSProperties = { color: "#9c8088", fontSize: 11, padding: "8px 2px" };
