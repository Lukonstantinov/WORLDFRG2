import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetLandmarks } from "@bridge";
import type { LandmarkBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Chip, EmptyNote } from "@ui/kit";

/** Phase 6 · Landmarks & Sacred Sites — the world's places of note (civic wonders,
 *  holy cities, fair towns, guildhalls), filterable by kind. Click → focus its city;
 *  a map toggle marks them all. Built on the shared UI kit (src/ui/kit.tsx). */

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
    <Panel onPointerDown={onPointerDown} width={320} maxHeight="72%" style={{ top: 70, right: 12, zIndex: 117, ...rootStyle }}>
      <PanelHeader
        icon="🗿"
        title="Landmarks & Sacred Sites"
        onDragStart={onPointerDown}
        onClose={close}
      />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: `${SPACE.md}px ${SPACE.lg}px ${SPACE.lg}px` }}>
        <div style={{ display: "flex", gap: SPACE.md, marginBottom: SPACE.md, fontSize: FZ.small, color: T.inkMid, alignItems: "center" }}>
          <span>{rows.length} places of note</span>
          <span style={{ flex: 1 }} />
          <label style={{ display: "flex", alignItems: "center", gap: 3, cursor: "pointer" }} data-no-drag>
            <input type="checkbox" checked={showMarks} onChange={(e) => setOverlayVisible("landmarks", e.target.checked)}
              style={{ accentColor: T.accent, width: 11, height: 11 }} />
            <span>Show on map</span>
          </label>
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: SPACE.md }} data-no-drag>
          {KINDS.map((k) => (
            <Chip key={k} on={kind === k} onClick={() => setKind(k)}>
              {k === "All" ? "All" : `${KIND_EMOJI[k]} ${KIND_LABEL[k]}`}
            </Chip>
          ))}
        </div>

        <div style={{ overflowY: "auto", flex: 1 }}>
          {!active && <EmptyNote>Start the campaign to see landmarks.</EmptyNote>}
          {active && shown.length === 0 && <EmptyNote>No landmarks yet — advance time.</EmptyNote>}
          {shown.map((l, i) => (
            <div key={i} onClick={() => focus(l.hub)}
              style={{ display: "flex", alignItems: "center", gap: SPACE.md, padding: "5px 2px", borderBottom: `1px dashed ${T.lineSoft}`, cursor: "pointer" }}>
              <span style={{ fontSize: FZ.title, width: 20, textAlign: "center" }}>{KIND_EMOJI[l.kind] ?? "•"}</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ color: T.ink, fontSize: FZ.body, textTransform: "capitalize" }}>{l.label}</div>
                <div style={{ color: T.inkMid, fontSize: FZ.tiny }}>
                  {l.city}{l.detail ? ` · ${l.detail}` : ""}
                </div>
              </div>
            </div>
          ))}
        </div>
      </PanelBody>
    </Panel>
  );
}
