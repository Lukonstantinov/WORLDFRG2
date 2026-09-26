import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetGovernment } from "@bridge";
import type { GovernmentBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { Panel, PanelHeader, PanelBody, Section, Card, Divider, StatGrid, Stat, Badge, Meter, ColHead, DataRow, EmptyNote, FootNote } from "@ui/kit";

const ALLEGIANCE_LABEL = ["house", "ruler", "commons"];
const ALLEGIANCE_TONE: ("gold" | "accent" | "neutral")[] = ["gold", "accent", "neutral"];

/** 🏛 Government — `docs/living_world/04_GOVERNMENT_AND_EDICTS.md` slice 04.7,
 *  "its own panel, like the war panel". Same seed-then-independent city-picker
 *  pattern as `MarketsPanel`/`marketsHub` — opens on whatever city is selected
 *  on the map, then stays put while the map moves on.
 *
 *  This is a first, deliberately plain cut: a header, a seat table, edicts in
 *  force and recent history. NOT built this session (queued, doc's own Q04.14):
 *  seat portraits, a live round-by-round debate TIMELINE (the tally is shown as
 *  a single number, not animated), and a bloc-grouped seat layout — the doc's
 *  own richer "seats as portraits grouped by bloc" spec. Nothing here is
 *  invented to fill the gap; each seat's real path/suitability/allegiance and
 *  every edict/history entry are the sim's own real fields. */
export function GovernmentPanel() {
  const open = useUIStore((s) => s.showGovernment);
  const hubId = useUIStore((s) => s.governmentHub);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.government);
  const [brief, setBrief] = useState<GovernmentBrief | null>(null);
  const tick = snapshot?.clock?.tick ?? 0;
  const year = Math.floor(tick / 365);

  useEffect(() => {
    let alive = true;
    if (!open || hubId == null) { setBrief(null); return; }
    campaignGetGovernment(hubId).then((b) => { if (alive) setBrief(b); }).catch(() => { if (alive) setBrief(null); });
    return () => { alive = false; };
  }, [open, hubId, year]);

  if (!open) return null;
  const close = () => useUIStore.getState().setShowGovernment(false);

  return (
    <Panel onPointerDown={onPointerDown} width={420} maxHeight="80%" style={{ top: 70, right: 12, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="🏛" title={brief ? `Government — ${brief.city}` : "Government"} onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ overflowY: "auto", padding: "6px 10px 10px" }}>
        {hubId == null && <EmptyNote>Select a city on the map to see who governs it.</EmptyNote>}
        {hubId != null && !brief && <EmptyNote>This city has no government seated yet.</EmptyNote>}
        {brief && (
          <>
            <Section>
              <StatGrid cols={3}>
                <Stat label="Form" value={brief.form} />
                <Stat label="Legitimacy" value={`${Math.round(brief.legitimacy * 100)}%`} />
                <Stat label="Points" value={brief.gov_points.toFixed(2)} hint="years of accrual" />
              </StatGrid>
              <div style={{ marginTop: 6, display: "flex", alignItems: "center", gap: 6 }}>
                <span style={{ fontSize: 11, opacity: 0.7 }}>conservative</span>
                <Meter value={brief.gov_position + 1} max={2} color="#9a7bd8" />
                <span style={{ fontSize: 11, opacity: 0.7 }}>libertarian</span>
              </div>
            </Section>

            {brief.debate && (
              <Section title="Debate in progress">
                <Card>
                  <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                    <span>{brief.debate.major ? "Major" : "Minor"} — {brief.debate.family}</span>
                    <Badge tone={brief.debate.tag < 0 ? "accent" : brief.debate.tag > 0 ? "gold" : "neutral"}>
                      {brief.debate.tag < 0 ? "conservative" : brief.debate.tag > 0 ? "libertarian" : "neutral"}
                    </Badge>
                  </div>
                  <FootNote>Round {brief.debate.round} of {brief.debate.round_cap} · cost {brief.debate.cost.toFixed(1)}</FootNote>
                  <div style={{ marginTop: 4, display: "flex", alignItems: "center", gap: 6 }}>
                    <span style={{ fontSize: 11, opacity: 0.7 }}>fail</span>
                    <Meter value={brief.debate.tally + 1} max={2} color="#5fd0ff" />
                    <span style={{ fontSize: 11, opacity: 0.7 }}>pass</span>
                  </div>
                </Card>
              </Section>
            )}

            <Section title={`Seats (${brief.seats.length})`}>
              <ColHead cols="1.4fr 1fr 0.8fr 0.7fr">
                <span>Officeholder</span><span>Path</span><span>Allegiance</span><span>Suit.</span>
              </ColHead>
              {brief.seats.map((s, i) => (
                <DataRow key={i} cols="1.4fr 1fr 0.8fr 0.7fr" zebra>
                  <span title={s.office_title}>{s.name} <FootNote>({s.office_title})</FootNote></span>
                  <span style={{ fontSize: 11 }}>{s.path}</span>
                  <Badge tone={ALLEGIANCE_TONE[s.allegiance] ?? "neutral"}>
                    {s.allegiance === 0 && s.house_name ? s.house_name : ALLEGIANCE_LABEL[s.allegiance] ?? "?"}
                  </Badge>
                  <span style={{ textAlign: "right" }}>{Math.round(s.suitability * 100)}%</span>
                </DataRow>
              ))}
            </Section>

            <Section title={`Edicts in force (${brief.edicts.length})`}>
              {brief.edicts.length === 0 && <FootNote>None currently in force.</FootNote>}
              {brief.edicts.map((e, i) => (
                <DataRow key={i} cols="1fr 0.6fr 0.6fr" zebra>
                  <span>{e.family}{e.major ? " (major)" : ""}</span>
                  <span style={{ fontSize: 11, opacity: 0.7 }}>since {e.enacted_year}</span>
                  <span style={{ fontSize: 11, opacity: 0.7, textAlign: "right" }}>until {e.expires_year}</span>
                </DataRow>
              ))}
            </Section>

            <Divider />
            <Section title="Recent history">
              {brief.history.length === 0 && <FootNote>No debates decided yet.</FootNote>}
              {brief.history.slice().reverse().map((h, i) => (
                <DataRow key={i} cols="0.5fr 1fr 0.7fr" zebra>
                  <span style={{ fontSize: 11, opacity: 0.7 }}>{h.year}</span>
                  <span>{h.family}</span>
                  <Badge tone={h.outcome === "passed" ? "good" : h.outcome === "failed" ? "bad" : "warn"}>{h.outcome}</Badge>
                </DataRow>
              ))}
            </Section>
          </>
        )}
      </PanelBody>
    </Panel>
  );
}
