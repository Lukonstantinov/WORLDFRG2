import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetMigrationRoutes } from "@bridge";
import type { MigrationRouteBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE, RADIUS } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, EmptyNote } from "@ui/kit";

/** Migration & Immigration — for the SELECTED city: who is arriving (and from where),
 *  who is leaving (the diaspora out), and the net flow. Aggregated from the live
 *  migration routes (people move only along trade ties). Prosperous cities pull people
 *  in and raise births; struggling ones bleed people to richer neighbours.
 *  Built on the shared UI kit (src/ui/kit.tsx). */

interface FlowRow { hub: number; name: string; volume: number; cultures: Set<string> }

const IN = T.goodInk;
const OUT = "#e0906a";

export function ImmigrationPanel() {
  const open = useUIStore((s) => s.showImmigration);
  const close = () => useUIStore.getState().setShowImmigration(false);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const selectedHub = useUIStore((s) => s.selectedHub);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [routes, setRoutes] = useState<MigrationRouteBrief[]>([]);
  useEffect(() => {
    if (!open || !active) return;
    campaignGetMigrationRoutes().then(setRoutes).catch(() => setRoutes([]));
  }, [open, active, tick]);

  // The selected city's SIM INDEX (migration routes reference hubs by index; the
  // snapshot.hubs array is index-aligned, each carrying its selection `id`).
  const simIdx = useMemo(
    () => (snapshot?.hubs ?? []).findIndex((h) => h.id === selectedHub),
    [snapshot, selectedHub],
  );
  const cityName = simIdx >= 0 ? snapshot?.hubs?.[simIdx]?.name ?? "" : "";

  const { arrivals, departures, arrivalsTotal, departuresTotal } = useMemo(() => {
    const nameOf = (i: number) => snapshot?.hubs?.[i]?.name ?? `#${i}`;
    const inMap = new Map<number, FlowRow>();
    const outMap = new Map<number, FlowRow>();
    let inSum = 0, outSum = 0;
    for (const r of routes) {
      if (r.to_hub === simIdx && r.from_hub !== simIdx) {
        const row = inMap.get(r.from_hub) ?? { hub: r.from_hub, name: nameOf(r.from_hub), volume: 0, cultures: new Set() };
        row.volume += r.volume; if (r.culture) row.cultures.add(r.culture); inMap.set(r.from_hub, row); inSum += r.volume;
      } else if (r.from_hub === simIdx && r.to_hub !== simIdx) {
        const row = outMap.get(r.to_hub) ?? { hub: r.to_hub, name: nameOf(r.to_hub), volume: 0, cultures: new Set() };
        row.volume += r.volume; if (r.culture) row.cultures.add(r.culture); outMap.set(r.to_hub, row); outSum += r.volume;
      }
    }
    const sortRows = (m: Map<number, FlowRow>) => [...m.values()].sort((a, b) => b.volume - a.volume);
    return { arrivals: sortRows(inMap), departures: sortRows(outMap), arrivalsTotal: inSum, departuresTotal: outSum };
  }, [routes, simIdx, snapshot]);

  const net = arrivalsTotal - departuresTotal;
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.immigration);
  if (!open) return null;

  const jump = (i: number) => { const id = snapshot?.hubs?.[i]?.id; if (id != null) setSelectedHub(id); };

  return (
    <Panel width={320} maxHeight="72%" style={{ top: 70, right: 12, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="🧭" title="Migration & Immigration" onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: `${SPACE.md}px ${SPACE.lg}px ${SPACE.lg}px` }}>
        {!active && <EmptyNote>Start the campaign to track migration.</EmptyNote>}
        {active && simIdx < 0 && <EmptyNote>Select a city on the map to see its migration.</EmptyNote>}

        {active && simIdx >= 0 && (
          <>
            <div style={{ color: T.ink, fontSize: FZ.base, fontWeight: 600, marginBottom: 4 }}>{cityName}</div>
            <div style={{ display: "flex", gap: 6, marginBottom: SPACE.md }}>
              <SignedStat label="arrived" value={arrivalsTotal} color={IN} sign="+" />
              <SignedStat label="left" value={departuresTotal} color={OUT} sign="−" />
              <SignedStat label="net" value={Math.abs(net)} color={net >= 0 ? IN : T.badInk} sign={net >= 0 ? "+" : "−"} />
            </div>

            <FlowList title="Arriving from" rows={arrivals} accent={IN}
              empty="No recent arrivals." onJump={jump} />
            <div style={{ height: SPACE.md }} />
            <FlowList title="Diaspora out — left for" rows={departures} accent={OUT}
              empty="No recent departures." onJump={jump} />

            <div style={{ fontSize: FZ.tiny, color: T.inkDim, marginTop: SPACE.md, lineHeight: 1.5 }}>
              People move only along <b>trade routes</b>, drawn toward prosperous, well-fed cities (which
              also see higher births). Newcomers form <b>minority quarters</b> that slowly assimilate — see
              the culture donut in the city's People tab.
            </div>
          </>
        )}
      </PanelBody>
    </Panel>
  );
}

function SignedStat({ label, value, color, sign }: { label: string; value: number; color: string; sign: string }) {
  return (
    <div style={{ flex: 1, textAlign: "center", padding: "4px 2px", borderRadius: RADIUS.sm, background: T.card, border: `1px solid ${T.lineSoft}` }}>
      <div style={{ color, fontSize: FZ.base, fontWeight: 700 }}>{value > 0.5 ? `${sign}${Math.round(value).toLocaleString()}` : "0"}</div>
      <div style={{ color: T.inkDim, fontSize: FZ.micro }}>{label}</div>
    </div>
  );
}

function FlowList({ title, rows, accent, empty, onJump }: {
  title: string; rows: FlowRow[]; accent: string; empty: string; onJump: (hub: number) => void;
}) {
  return (
    <div>
      <div style={{ color: T.inkMid, fontSize: FZ.micro, textTransform: "uppercase", letterSpacing: 0.4, marginBottom: 3 }}>{title}</div>
      {rows.length === 0 ? (
        <div style={{ color: T.inkDim, fontSize: FZ.small }}>{empty}</div>
      ) : (
        rows.slice(0, 8).map((r) => (
          <div key={r.hub} onClick={() => onJump(r.hub)}
            style={{ display: "flex", alignItems: "center", gap: 6, fontSize: FZ.small, padding: "2px 3px", cursor: "pointer" }}>
            <span style={{ width: 6, height: 6, borderRadius: 3, background: accent, flex: "0 0 auto" }} />
            <span style={{ flex: 1, color: T.ink, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.name}</span>
            {r.cultures.size > 0 && (
              <span style={{ color: T.inkMid, fontSize: FZ.micro, maxWidth: 90, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {[...r.cultures].slice(0, 2).join(", ")}
              </span>
            )}
            <span style={{ color: accent, minWidth: 44, textAlign: "right" }}>{Math.round(r.volume).toLocaleString()}</span>
          </div>
        ))
      )}
    </div>
  );
}
