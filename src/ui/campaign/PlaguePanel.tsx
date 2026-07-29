import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetEpidemics } from "@bridge";
import type { EpidemicBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE, RADIUS } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Chip, EmptyNote } from "@ui/kit";

/** Phase 6 · Plagues & Epidemics — every outbreak (a contagion chain), sortable
 *  most→least deadly, expandable to its affected cities + contagion routes. Clicking
 *  a city focuses it on the map (and the plague overlay draws the spread).
 *  Built on the shared UI kit (src/ui/kit.tsx). */

type Sort = "deadliest" | "cities" | "recent" | "active";
const SORTS: { id: Sort; label: string }[] = [
  { id: "deadliest", label: "Deadliest" },
  { id: "cities", label: "Most cities" },
  { id: "recent", label: "Recent" },
  { id: "active", label: "Active first" },
];

export function PlaguePanel() {
  const open = useUIStore((s) => s.showPlagues);
  const close = () => useUIStore.getState().setShowPlagues(false);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const showZones = useUIStore((s) => s.overlayVisibility.plagueZones);
  const plagueReplay = useUIStore((s) => s.plagueReplay);
  const setPlagueReplay = useUIStore((s) => s.setPlagueReplay);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [rows, setRows] = useState<EpidemicBrief[]>([]);
  const [sort, setSort] = useState<Sort>("active");
  const [activeOnly, setActiveOnly] = useState(false);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());

  // Closing the panel ends any spread replay on the map.
  useEffect(() => { if (!open) setPlagueReplay(null); }, [open, setPlagueReplay]);

  useEffect(() => {
    if (!open || !active) return;
    campaignGetEpidemics().then(setRows).catch(() => setRows([]));
  }, [open, active, tick]);

  const shown = useMemo(() => {
    let r = activeOnly ? rows.filter((e) => e.active) : rows;
    r = [...r].sort((a, b) => {
      switch (sort) {
        case "deadliest": return b.total_dead - a.total_dead;
        case "cities": return b.cities.length - a.cities.length;
        case "recent": return b.end_year - a.end_year;
        default: return (b.active ? 1 : 0) - (a.active ? 1 : 0) || b.total_dead - a.total_dead;
      }
    });
    return r.slice(0, 120);
  }, [rows, sort, activeOnly]);

  const { rootStyle, onPointerDown, dragRoot } = useFloatingWindow(PANEL_TINTS.plague);
  if (!open) return null;

  const totalDead = rows.reduce((s, e) => s + e.total_dead, 0);
  const totalIll = rows.reduce((s, e) => s + (e.total_ill ?? e.total_dead), 0);
  const totalRecovered = rows.reduce((s, e) => s + (e.total_recovered ?? 0), 0);
  const activeCount = rows.filter((e) => e.active).length;
  const focusCity = (hubIdx: number) => {
    const id = snapshot?.hubs?.[hubIdx]?.id;
    if (id != null) setSelectedHub(id);
  };

  return (
    <Panel {...dragRoot} width={340} maxHeight="72%" style={{ top: 70, right: 12, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="☠" title="Plagues & Epidemics" onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: `${SPACE.md}px ${SPACE.lg}px ${SPACE.lg}px` }}>
        <div style={{ display: "flex", gap: SPACE.md, marginBottom: SPACE.md, fontSize: FZ.small, color: T.inkMid, alignItems: "center" }}>
          <span>{rows.length} outbreak{rows.length === 1 ? "" : "s"}</span>
          <span>· {activeCount} active</span>
          <span style={{ flex: 1 }} />
          <label style={{ display: "flex", alignItems: "center", gap: 3, cursor: "pointer" }} data-no-drag>
            <input type="checkbox" checked={showZones} onChange={(e) => setOverlayVisible("plagueZones", e.target.checked)}
              style={{ accentColor: T.bad, width: 11, height: 11 }} />
            <span>Show on map</span>
          </label>
        </div>

        {/* SIR tally across all outbreaks: how many fell ill, died, and recovered. */}
        <div style={{ display: "flex", gap: 6, marginBottom: SPACE.md }}>
          <SirStat label="fell ill" value={totalIll} color={T.warn} />
          <SirStat label="died" value={totalDead} color={T.badInk} />
          <SirStat label="recovered" value={totalRecovered} color={T.goodInk} />
        </div>

        {/* How plagues work — the mechanics, so the numbers are legible. */}
        <details style={{ marginBottom: SPACE.md }} data-no-drag>
          <summary style={{ fontSize: FZ.small, color: T.inkMid, cursor: "pointer" }}>How plagues work</summary>
          <div style={{ fontSize: FZ.tiny, color: T.inkDim, lineHeight: 1.5, padding: "3px 2px 0" }}>
            A disease breaks out in a city, kills a share of its people, and shuts the gates under
            <b> quarantine</b> (trade suspended). It then travels the <b>trade lanes only</b> — Local
            outbreaks stay put, Regional reach one more city, a Great Plague spreads ~4000&nbsp;km along
            the network. Survivors gain <b>immunity</b> for years (stronger for smallpox/plague, weak for
            flu/cholera, so those recur). Wealthy cities that fund <b>hospices/quarantine</b> lose fewer
            people and resist longer. <i>Ill</i> = caught it; <i>recovered</i> = ill who survived.
          </div>
        </details>

        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: SPACE.md }} data-no-drag>
          {SORTS.map((s) => (
            <Chip key={s.id} on={sort === s.id} onClick={() => setSort(s.id)}>{s.label}</Chip>
          ))}
          <Chip on={activeOnly} onClick={() => setActiveOnly((v) => !v)}>Active only</Chip>
        </div>

        <div style={{ overflowY: "auto", flex: 1 }}>
          {!active && <EmptyNote>Start the campaign to track epidemics.</EmptyNote>}
          {active && shown.length === 0 && <EmptyNote>No plagues recorded yet.</EmptyNote>}
          {shown.map((e) => {
            const isOpen = expanded.has(e.id);
            return (
              <div key={e.id} style={{ borderBottom: `1px solid ${T.lineSoft}` }}>
                <div onClick={() => setExpanded((prev) => { const n = new Set(prev); n.has(e.id) ? n.delete(e.id) : n.add(e.id); return n; })}
                  style={{ display: "flex", alignItems: "center", gap: 6, padding: "5px 2px", cursor: "pointer" }}>
                  <span style={{ color: T.inkDim, fontSize: FZ.tiny, width: 9 }}>{isOpen ? "▾" : "▸"}</span>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ color: T.parchment, fontSize: FZ.body, fontWeight: 600 }}>
                      {e.name} <CategoryTag category={e.category} />
                      {e.transmission && (
                        <span style={{ fontSize: FZ.micro, color: T.inkMid, border: `1px solid ${T.line}`, borderRadius: 8, padding: "0 5px", marginLeft: 3 }}>
                          {e.transmission}
                        </span>
                      )}
                      {e.active && <span style={{ color: T.badInk, fontSize: FZ.tiny }}> ● active</span>}
                    </div>
                    <div style={{ color: T.inkDim, fontSize: FZ.tiny }}>
                      Year {e.start_year}{e.end_year > e.start_year ? `–${e.end_year}` : ""} · {e.cities.length} cit{e.cities.length === 1 ? "y" : "ies"}
                    </div>
                  </div>
                  <div style={{ textAlign: "right" }}>
                    <div style={{ color: T.badInk, fontSize: FZ.base, fontWeight: 700 }}>{e.total_dead.toLocaleString()}</div>
                    <div style={{ color: T.inkDim, fontSize: FZ.micro }}>died</div>
                  </div>
                </div>
                {isOpen && (
                  <div style={{ paddingLeft: 14, paddingBottom: 5 }}>
                    <div style={{ fontSize: FZ.tiny, color: T.inkDim, marginBottom: 3 }}>
                      Began in <b style={{ color: T.badInk }}>{e.origin_name}</b> · spread to {e.cities.length - 1} more
                    </div>
                    {/* Spread REPLAY: drag to trace how the plague travelled the trade
                        lanes from its origin, city by city (plague-red on the map). */}
                    {(() => {
                      const maxOrder = e.cities.reduce((m, c) => Math.max(m, c.order), 0);
                      if (maxOrder < 1) return null;
                      const replaying = plagueReplay?.id === e.id;
                      const step = replaying ? plagueReplay!.step : 0;
                      return (
                        <div style={{ display: "flex", alignItems: "center", gap: 6, margin: "2px 0 5px", padding: "3px 5px", background: T.card, border: `1px solid ${T.line}`, borderRadius: RADIUS.sm }}>
                          <span title="Trace the spread on the map" data-no-drag
                            onClick={() => setPlagueReplay(replaying ? null : { id: e.id, step: 0 })}
                            style={{ fontSize: FZ.small, cursor: "pointer", color: replaying ? T.badInk : T.inkMid }}>
                            {replaying ? "■ stop" : "▶ trace"}
                          </span>
                          <input type="range" min={0} max={maxOrder} step={1} value={step} data-no-drag
                            onChange={(ev) => setPlagueReplay({ id: e.id, step: Number(ev.target.value) })}
                            style={{ flex: 1, accentColor: T.bad }} />
                          <span style={{ fontSize: FZ.tiny, color: T.inkMid, minWidth: 54, textAlign: "right" }}>
                            {step === 0 ? "origin" : `+${step} ${step === 1 ? "city" : "cities"}`}
                          </span>
                        </div>
                      );
                    })()}
                    {/* Cities in SPREAD ORDER (origin first) — the contagion history. */}
                    {e.cities.map((c) => (
                      <div key={c.hub} onClick={() => focusCity(c.hub)}
                        style={{ display: "flex", alignItems: "center", gap: 6, fontSize: FZ.small, padding: "2px 4px", cursor: "pointer" }}>
                        <span style={{ color: T.inkDim, fontSize: FZ.micro, minWidth: 26 }}>
                          {c.order === 0 ? "★ src" : `→ ${c.order}`}
                        </span>
                        <span style={{ flex: 1, color: T.inkMid }}>
                          {c.name} <span style={{ color: T.inkFaint }}>Yr{c.year}</span>
                          {c.active && <span style={{ color: T.badInk }}> · quarantined</span>}
                          {c.from_name && <span style={{ color: T.inkFaint }}> ← {c.from_name}</span>}
                        </span>
                        {c.ill != null && c.ill > c.deaths && (
                          <span style={{ color: T.warn, minWidth: 42, textAlign: "right" }} title="fell ill">{c.ill.toLocaleString()} ⚕</span>
                        )}
                        <span style={{ color: T.badInk, minWidth: 44, textAlign: "right" }} title="died">{c.deaths.toLocaleString()} †</span>
                        {c.recovered != null && c.recovered > 0 && (
                          <span style={{ color: T.goodInk, minWidth: 42, textAlign: "right" }} title="recovered">{c.recovered.toLocaleString()} ✚</span>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </PanelBody>
    </Panel>
  );
}

/** One SIR tally chip (fell ill / died / recovered) for the panel header. */
function SirStat({ label, value, color }: { label: string; value: number; color: string }) {
  return (
    <div style={{ flex: 1, textAlign: "center", padding: "3px 2px", borderRadius: RADIUS.sm,
      background: T.card, border: `1px solid ${T.lineSoft}` }}>
      <div style={{ color, fontSize: FZ.head, fontWeight: 700 }}>{value.toLocaleString()}</div>
      <div style={{ color: T.inkDim, fontSize: FZ.micro }}>{label}</div>
    </div>
  );
}

/** Plague severity badge: Cat 1 = Great Plague (rare, reaches ~4000 km along the trade
 *  lanes), Cat 2 = Regional (reaches one further city), Cat 3 = Local outbreak. */
function CategoryTag({ category }: { category: number }) {
  const meta = category === 1
    ? { label: "Cat 1 · Great Plague", bg: "rgba(192,64,64,0.18)", bd: "#c04040", fg: "#ffb0a0" }
    : category === 2
    ? { label: "Cat 2 · Regional", bg: "rgba(176,112,48,0.18)", bd: "#b07030", fg: "#f0c890" }
    : { label: "Cat 3 · Local", bg: "rgba(90,112,64,0.18)", bd: "#5a7040", fg: "#bcd8a0" };
  return (
    <span style={{ fontSize: FZ.micro, fontWeight: 700, padding: "1px 5px", borderRadius: 10,
      border: `1px solid ${meta.bd}`, background: meta.bg, color: meta.fg, whiteSpace: "nowrap" }}>
      {meta.label}
    </span>
  );
}
