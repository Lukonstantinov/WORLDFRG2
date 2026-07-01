import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetLandmarks } from "../bridge/tauri";
import type { LandmarkBrief } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** Phase 6 · Landmarks & Sacred Sites — the world's places of note (civic wonders,
 *  holy cities, fair towns, guildhalls), filterable by kind. Click → focus its city;
 *  a map toggle marks them all. */

const KIND_EMOJI: Record<string, string> = { wonder: "🗿", temple: "⛪", fair: "🎪", guildhall: "🏛" };
const KIND_LABEL: Record<string, string> = { wonder: "Wonders", temple: "Holy cities", fair: "Fairs", guildhall: "Guildhalls" };
const KINDS = ["All", "wonder", "temple", "fair", "guildhall"];

export function LandmarksPanel() {
  const open = useUIStore((s) => s.showLandmarks);
  const close = () => useUIStore.getState().setShowLandmarks(false);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const showMarks = useUIStore((s) => s.overlayVisibility.landmarks);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [rows, setRows] = useState<LandmarkBrief[]>([]);
  const [kind, setKind] = useState("All");

  useEffect(() => {
    if (!open || !active) return;
    campaignGetLandmarks().then(setRows).catch(() => setRows([]));
  }, [open, active, tick]);

  const shown = useMemo(() => (kind === "All" ? rows : rows.filter((l) => l.kind === kind)), [rows, kind]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.landmarks);
  if (!open) return null;

  const focus = (hubIdx: number) => {
    const id = snapshot?.hubs?.[hubIdx]?.id;
    if (id != null) setSelectedHub(id);
  };

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ display: "flex", alignItems: "center", marginBottom: 6, cursor: "move" }} onPointerDown={onPointerDown}>
        <span style={{ color: "#bfe0d4", fontWeight: 700, fontSize: 13 }}>🗿 Landmarks &amp; Sacred Sites</span>
        <span style={{ flex: 1 }} />
        <span data-no-drag onClick={close} title="Close" style={{ color: "#70a090", cursor: "pointer", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>

      <div style={{ display: "flex", gap: 8, marginBottom: 6, fontSize: 10, color: "#7f9c90" }}>
        <span>{rows.length} places of note</span>
        <span style={{ flex: 1 }} />
        <label style={{ display: "flex", alignItems: "center", gap: 3, cursor: "pointer" }} data-no-drag>
          <input type="checkbox" checked={showMarks} onChange={(e) => setOverlayVisible("landmarks", e.target.checked)}
            style={{ accentColor: "#40b090", width: 11, height: 11 }} />
          <span>Show on map</span>
        </label>
      </div>

      <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 6 }} data-no-drag>
        {KINDS.map((k) => (
          <span key={k} onClick={() => setKind(k)}
            style={{ fontSize: 10, padding: "2px 8px", borderRadius: 20, cursor: "pointer",
              border: `1px solid ${kind === k ? "#40b090" : "#243a34"}`,
              background: kind === k ? "#12281f" : "transparent",
              color: kind === k ? "#cfeee0" : "#7f9c90" }}>
            {k === "All" ? "All" : `${KIND_EMOJI[k]} ${KIND_LABEL[k]}`}
          </span>
        ))}
      </div>

      <div style={{ overflowY: "auto", flex: 1 }}>
        {!active && <div style={empty}>Start the campaign to see landmarks.</div>}
        {active && shown.length === 0 && <div style={empty}>No landmarks yet — advance time.</div>}
        {shown.map((l, i) => (
          <div key={i} onClick={() => focus(l.hub)}
            style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 2px", borderBottom: "1px dashed #172622", cursor: "pointer" }}>
            <span style={{ fontSize: 15, width: 20, textAlign: "center" }}>{KIND_EMOJI[l.kind] ?? "•"}</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ color: "#d8ecdf", fontSize: 11, textTransform: "capitalize" }}>{l.label}</div>
              <div style={{ color: "#7f9c90", fontSize: 9 }}>
                {l.city}{l.detail ? ` · ${l.detail}` : ""}
              </div>
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
  border: "1px solid #243a34", borderRadius: 8, padding: "10px 12px", zIndex: 117,
  boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
const empty: React.CSSProperties = { color: "#7f9c90", fontSize: 11, padding: "8px 2px" };
