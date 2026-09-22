import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { FeudsView } from "@ui/campaign/HouseDossier";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { Panel, PanelHeader, PanelBody } from "@ui/kit";

/** ⚔ Feuds & Alliances — HOUSES_GUILDS_AND_MARKET_PLAN.md S10's world relations
 *  board. Split out of HousesPanel.tsx's old "feuds" tab (§5's own table: "a feud
 *  belongs to two houses, not one — it was never a house's tab"), so the world's
 *  quarrels are their own window rather than something you had to open a random
 *  house's panel to see. Reuses `FeudsView` (`HouseDossier.tsx`) with no house
 *  focus, which is exactly what the old tab already rendered.
 *
 *  Alliances (queue Q10 — the positive counterpart to the fully-built Feud) are
 *  NOT built: nothing tracks a house-to-house alliance yet, so this window shows
 *  only what the sim actually models today. The name is the plan's own, kept so
 *  the window doesn't need renaming again once Q10 lands. */
export function FeudsAlliancesPanel() {
  const open = useUIStore((s) => s.showFeuds);
  const close = () => useUIStore.getState().setShowFeuds(false);
  const clockTick = useCampaignStore((s) => s.snapshot?.clock.tick ?? 0);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.feuds);
  if (!open) return null;

  return (
    <Panel onPointerDown={onPointerDown} width={360} maxHeight="76%" style={{ top: 70, right: 12, zIndex: 116, ...rootStyle }}>
      <PanelHeader icon="⚔" title="Feuds & Alliances" onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ overflowY: "auto", padding: "6px 10px 10px" }}>
        <FeudsView refreshKey={clockTick} />
      </PanelBody>
    </Panel>
  );
}
