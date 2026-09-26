import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetHordes } from "@bridge";
import type { HordeBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { Panel, PanelHeader, PanelBody, Section, DataRow, Badge, EmptyNote, FootNote } from "@ui/kit";

/** 🏇 Barbarian Tribes — `docs/living_world/09_REALMS_WAR_AND_BARBARIANS.md`
 *  Part D/F. Each horde: leader, goal, origin story, strength, and its
 *  running tally of cities sacked/razed. This is Part D's own data only —
 *  Parts B/C/E (armies, realm wars, a real conflict-map layer) were not
 *  built this session, so there is no minimap/army-token overlay here yet
 *  (queued, see `hordes.rs`'s own doc comment). */
export function BarbarianTribesPanel() {
  const open = useUIStore((s) => s.showHordes);
  const close = () => useUIStore.getState().setShowHordes(false);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.war ?? PANEL_TINTS.government);
  const [rows, setRows] = useState<HordeBrief[]>([]);

  useEffect(() => {
    if (!open) return;
    campaignGetHordes().then(setRows).catch(() => setRows([]));
  }, [open, tick]);

  if (!open) return null;

  return (
    <Panel onPointerDown={onPointerDown} width={420} maxHeight="80%" style={{ top: 70, right: 12, zIndex: 118, ...rootStyle }}>
      <PanelHeader icon="🏇" title="Barbarian Tribes" onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ overflowY: "auto", padding: "6px 10px 10px" }}>
        {rows.length === 0 && <EmptyNote>No hordes have risen yet.</EmptyNote>}
        {rows.map((h) => (
          <Section key={h.id} title={h.name}>
            <FootNote>{h.origin_story}</FootNote>
            <DataRow cols="1fr 1fr" zebra>
              <span>Leader: {h.leader_name}</span>
              <span style={{ fontSize: 11, opacity: 0.7 }}>Goal: {h.goal_name}</span>
            </DataRow>
            <DataRow cols="1fr 1fr 0.6fr" zebra>
              <span style={{ fontSize: 11, opacity: 0.7 }}>Strength {h.strength.toFixed(2)}</span>
              <span style={{ fontSize: 11, opacity: 0.7 }}>Sacked {h.cities_sacked} · Razed {h.cities_razed}</span>
              <Badge tone={h.stage_name === "active" ? "warn" : h.stage_name === "defeated" ? "bad" : "good"}>
                {h.stage_name}
              </Badge>
            </DataRow>
          </Section>
        ))}
      </PanelBody>
    </Panel>
  );
}
