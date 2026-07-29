import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetDynasties } from "@bridge";
import type { DynastiesPayload, HouseLink } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Chip, EmptyNote } from "@ui/kit";

/** Phase 7 · Dynasties & Alliances — marriage alliances (💍) and feuds (🗡) between
 *  the great houses. Click a house to focus its seat; a map toggle draws the ties.
 *  Built on the shared UI kit (src/ui/kit.tsx). */

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
    <Panel onPointerDown={onPointerDown} width={330} maxHeight="72%" style={{ top: 70, right: 12, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="⚭" title="Dynasties & Alliances" onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: `${SPACE.md}px ${SPACE.lg}px ${SPACE.lg}px` }}>
        <div style={{ display: "flex", gap: SPACE.md, marginBottom: SPACE.md, fontSize: FZ.small, color: T.inkMid, alignItems: "center" }}>
          <span>{data.alliances.length} alliance{data.alliances.length === 1 ? "" : "s"}</span>
          <span>· {data.feuds.length} feud{data.feuds.length === 1 ? "" : "s"}</span>
          <span style={{ flex: 1 }} />
          <label style={{ display: "flex", alignItems: "center", gap: 3, cursor: "pointer" }} data-no-drag>
            <input type="checkbox" checked={showLinks} onChange={(e) => setOverlayVisible("dynastyLinks", e.target.checked)}
              style={{ accentColor: T.accent, width: 11, height: 11 }} />
            <span>Show on map</span>
          </label>
        </div>

        <div style={{ display: "flex", gap: 4, marginBottom: SPACE.md }} data-no-drag>
          {(["alliances", "feuds"] as Tab[]).map((t) => (
            <Chip key={t} on={tab === t} onClick={() => setTab(t)}>
              {t === "alliances" ? "💍 Alliances" : "🗡 Feuds"}
            </Chip>
          ))}
        </div>

        <div style={{ overflowY: "auto", flex: 1 }}>
          {!active && <EmptyNote>Start the campaign to trace the dynasties.</EmptyNote>}
          {active && rows.length === 0 && (
            <EmptyNote>{tab === "alliances" ? "No marriage alliances yet." : "No active feuds."}</EmptyNote>
          )}
          {rows.map((l: HouseLink, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 6, padding: "5px 2px", borderBottom: `1px dashed ${T.lineSoft}`, fontSize: FZ.body }}>
              <span style={{ width: 16, textAlign: "center" }}>{tab === "alliances" ? "💍" : "🗡"}</span>
              <span onClick={() => focus(l.a_hub)} title={l.a_city}
                style={{ color: T.parchment, cursor: "pointer", flex: 1, textAlign: "right", overflow: "hidden", textOverflow: "ellipsis" }}>
                {stripHouse(l.a_name)}
              </span>
              <span style={{ color: tab === "alliances" ? T.gold : T.bad, fontSize: FZ.base }}>
                {tab === "alliances" ? "⚭" : "⚔"}
              </span>
              <span onClick={() => focus(l.b_hub)} title={l.b_city}
                style={{ color: T.parchment, cursor: "pointer", flex: 1, overflow: "hidden", textOverflow: "ellipsis" }}>
                {stripHouse(l.b_name)}
              </span>
            </div>
          ))}
        </div>
      </PanelBody>
    </Panel>
  );
}
