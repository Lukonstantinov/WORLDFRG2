import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetFigures } from "@bridge";
import type { FigureBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Chip, EmptyNote } from "@ui/kit";

/** Phase 6 · Notable Figures — the campaign's Great Lives (admirals, demagogues,
 *  master craftsmen, bankers, explorers), living first, filterable by role. Clicking
 *  a figure focuses their city; a map toggle marks the living.
 *  Built on the shared UI kit (src/ui/kit.tsx). */

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

  const { rootStyle, onPointerDown, dragRoot } = useFloatingWindow(PANEL_TINTS.figures);
  if (!open) return null;

  const living = rows.filter((f) => f.alive).length;
  const focus = (hubIdx: number) => {
    const id = snapshot?.hubs?.[hubIdx]?.id;
    if (id != null) setSelectedHub(id);
  };

  return (
    <Panel {...dragRoot} width={320} maxHeight="72%" style={{ top: 70, right: 12, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="⚜️" title="Notable Figures" onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: `${SPACE.md}px ${SPACE.lg}px ${SPACE.lg}px` }}>
        <div style={{ display: "flex", gap: SPACE.md, marginBottom: SPACE.md, fontSize: FZ.small, color: T.inkMid, alignItems: "center" }}>
          <span>{rows.length} figures</span><span>· {living} living</span>
          <span style={{ flex: 1 }} />
          <label style={{ display: "flex", alignItems: "center", gap: 3, cursor: "pointer" }} data-no-drag>
            <input type="checkbox" checked={showMarks} onChange={(e) => setOverlayVisible("figureMarks", e.target.checked)}
              style={{ accentColor: T.accent, width: 11, height: 11 }} />
            <span>Show on map</span>
          </label>
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: SPACE.md }} data-no-drag>
          {ROLES.map((r) => (
            <Chip key={r} on={role === r} onClick={() => setRole(r)}>
              {r === "All" ? "All" : (ROLE_EMOJI[r] ?? "") + " " + r.split(" ")[0]}
            </Chip>
          ))}
          <Chip on={livingOnly} onClick={() => setLivingOnly((v) => !v)}>Living</Chip>
        </div>

        <div style={{ overflowY: "auto", flex: 1 }}>
          {!active && <EmptyNote>Start the campaign to see notable figures.</EmptyNote>}
          {active && shown.length === 0 && <EmptyNote>No figures yet — advance time.</EmptyNote>}
          {shown.map((f, i) => (
            <div key={i} onClick={() => focus(f.hub)}
              style={{ display: "flex", alignItems: "center", gap: SPACE.md, padding: "5px 2px", borderBottom: `1px dashed ${T.lineSoft}`, cursor: "pointer" }}>
              <span style={{ fontSize: FZ.title, width: 20, textAlign: "center", opacity: f.alive ? 1 : 0.5 }}>{ROLE_EMOJI[f.role] ?? "•"}</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ color: f.alive ? T.parchment : T.inkMid, fontSize: FZ.body }}>
                  {f.name} {!f.alive && <span style={{ color: T.inkDim, fontSize: FZ.tiny }}>†</span>}
                </div>
                <div style={{ color: T.inkDim, fontSize: FZ.tiny }}>
                  {f.role}{f.good_name ? ` · ${f.good_name}` : ""} · {f.city}
                </div>
              </div>
              <div style={{ color: T.inkMid, fontSize: FZ.tiny, textAlign: "right" }}>
                {f.alive ? `b. ${f.born_year}` : `${f.born_year}–${f.died_year}`}
              </div>
            </div>
          ))}
        </div>
      </PanelBody>
    </Panel>
  );
}
