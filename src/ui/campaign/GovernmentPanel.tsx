import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetGovernment, campaignGetCityIdeology, campaignGetCityGallery, campaignGetVenues } from "@bridge";
import type { GovernmentBrief, CityIdeologyBrief, MasterworkBrief, VenueBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { Panel, PanelHeader, PanelBody, Section, Card, Divider, StatGrid, Stat, Badge, Meter, ColHead, DataRow, EmptyNote, FootNote } from "@ui/kit";
import { GovFormBadge } from "@ui/campaign/govFormBadge";

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
    <Panel onPointerDown={onPointerDown} width={420} maxHeight="80%" style={{ top: 70, right: 12, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="🏛" title={brief ? `Government — ${brief.city}` : "Government"} onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ overflowY: "auto", padding: "6px 10px 10px" }}>
        {hubId == null && <EmptyNote>Select a city on the map to see who governs it.</EmptyNote>}
        {hubId != null && !brief && <EmptyNote>This city has no government seated yet.</EmptyNote>}
        {brief && (
          <>
            <Section>
              <StatGrid cols={3}>
                <Stat label="Form" value={<GovFormBadge form={brief.form} />} />
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
                  <span>{h.regime_kind ? `a change of government — ${h.regime_kind}` : h.family}</span>
                  <Badge tone={h.outcome === "passed" ? "good" : h.outcome === "failed" ? "bad" : "warn"}>
                    {h.regime_kind || h.outcome}
                  </Badge>
                </DataRow>
              ))}
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
