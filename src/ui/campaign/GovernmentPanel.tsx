import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetGovernment, campaignGetCityIdeology, campaignGetCityGallery, campaignGetVenues } from "@bridge";
import type { GovernmentBrief, CityIdeologyBrief, MasterworkBrief, VenueBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { Panel, PanelHeader, PanelBody, Section, Divider, Badge, Meter, DataRow, EmptyNote, FootNote } from "@ui/kit";
import { CouncilChamber } from "@ui/campaign/CouncilChamber";


/** 🏛 Government — `docs/living_world/04_GOVERNMENT_AND_EDICTS.md` slice 04.7,
 *  "its own panel, like the war panel". Same seed-then-independent city-picker
 *  pattern as `MarketsPanel`/`marketsHub` — opens on whatever city is selected
 *  on the map, then stays put while the map moves on.
 *
 *  2026-09-30 · the seats are now a council CHAMBER (`CouncilChamber`) —
 *  portraits on a semicircle ringed in the colour of whom they answer to,
 *  office and path icons, traits as icons, each seat holder's own ideology
 *  and live vote lean, the motion on the floor with its round pips, and the
 *  prevailing doctrine's demands (met/unmet). Still not built (Q04.14): an
 *  animated round-by-round timeline. */
export function GovernmentPanel() {
  const open = useUIStore((s) => s.showGovernment);
  const hubId = useUIStore((s) => s.governmentHub);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.government);
  const [brief, setBrief] = useState<GovernmentBrief | null>(null);
  const [ideology, setIdeology] = useState<CityIdeologyBrief | null>(null);
  const [gallery, setGallery] = useState<MasterworkBrief[]>([]);
  const [venues, setVenues] = useState<VenueBrief[]>([]);
  const tick = snapshot?.clock?.tick ?? 0;
  const year = Math.floor(tick / 365);

  useEffect(() => {
    let alive = true;
    if (!open || hubId == null) { setBrief(null); return; }
    campaignGetGovernment(hubId).then((b) => { if (alive) setBrief(b); }).catch(() => { if (alive) setBrief(null); });
    campaignGetCityIdeology(hubId).then((b) => { if (alive) setIdeology(b); }).catch(() => { if (alive) setIdeology(null); });
    campaignGetCityGallery(hubId).then((g) => { if (alive) setGallery(g); }).catch(() => { if (alive) setGallery([]); });
    campaignGetVenues(hubId).then((v) => { if (alive) setVenues(v); }).catch(() => { if (alive) setVenues([]); });
    return () => { alive = false; };
  }, [open, hubId, year]);

  if (!open) return null;
  const close = () => useUIStore.getState().setShowGovernment(false);

  return (
    <Panel onPointerDown={onPointerDown} width={500} maxHeight="80%" style={{ top: 70, right: 12, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="🏛" title={brief ? `The Council of ${brief.city}` : "Government"} onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ overflowY: "auto", padding: "6px 10px 10px" }}>
        {hubId == null && <EmptyNote>Select a city on the map to see who governs it.</EmptyNote>}
        {hubId != null && !brief && <EmptyNote>This city has no government seated yet.</EmptyNote>}
        {brief && (
          <>
            <Section>
              <CouncilChamber brief={brief} />
            </Section>

            {ideology && (
              <>
                <Divider />
                <Section title="Ideology — living_world/06">
                  {ideology.dominant_id >= 0 && (
                    <FootNote>Prevailing doctrine: <b>{ideology.dominant_name}</b></FootNote>
                  )}
                  {["Authority", "Tradition", "Openness", "Economy"].map((axis, i) => (
                    <div key={axis} style={{ marginTop: 4, display: "flex", alignItems: "center", gap: 6 }}>
                      <span style={{ fontSize: 11, opacity: 0.7, width: 66 }}>{axis}</span>
                      <Meter value={ideology.government[i] + 5} max={10} color="#7bb0d8" />
                      <span style={{ fontSize: 10, opacity: 0.6 }}>{ideology.government[i].toFixed(1)}</span>
                    </div>
                  ))}
                  {ideology.scholars.length === 0 && ideology.schools.length === 0 && (
                    <FootNote>No resident scholars.</FootNote>
                  )}
                  {ideology.scholars.map((s) => (
                    <DataRow key={s.individual_id} cols="1fr 0.6fr" zebra>
                      <span>{s.name}</span>
                      <span style={{ fontSize: 11, opacity: 0.7, textAlign: "right" }}>
                        {["", "studying", "teaching", "returned home", "under a patron", "in politics", "in exile"][s.stage] ?? ""}
                      </span>
                    </DataRow>
                  ))}
                  {ideology.schools.length > 0 && (
                    <FootNote>{ideology.schools.length} school{ideology.schools.length === 1 ? "" : "s"} founded here.</FootNote>
                  )}
                </Section>
              </>
            )}

            {venues.length > 0 && (
              <>
                <Divider />
                <Section title="Venues — living_world/08">
                  {venues.map((v) => (
                    <DataRow key={v.id} cols="1fr 0.6fr 0.5fr" zebra>
                      <span>{v.name} <FootNote>(tier {v.tier} {v.leisure_type_name}{v.international_host ? " · international" : ""})</FootNote></span>
                      <span style={{ fontSize: 11, opacity: 0.7 }}>{v.games_held} games</span>
                      <Badge tone={v.condition_name === "thriving" ? "good" : v.condition_name === "declining" ? "warn" : "bad"}>
                        {v.condition_name}
                      </Badge>
                    </DataRow>
                  ))}
                </Section>
              </>
            )}

            {gallery.length > 0 && (
              <>
                <Divider />
                <Section title="Gallery — living_world/07">
                  {gallery.map((m) => (
                    <DataRow key={m.id} cols="1fr 0.5fr 0.5fr" zebra>
                      <span title={m.provenance.map(([, t]) => t).join(" · ")}>{m.title}</span>
                      <span style={{ fontSize: 11, opacity: 0.7 }}>{m.maker_name}</span>
                      <Badge tone={m.condition_name === "intact" ? "good" : m.condition_name === "damaged" ? "warn" : "bad"}>
                        {m.condition_name}
                      </Badge>
                    </DataRow>
                  ))}
                </Section>
              </>
            )}
          </>
        )}
      </PanelBody>
    </Panel>
  );
}
