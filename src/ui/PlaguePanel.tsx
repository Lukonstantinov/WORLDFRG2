import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetEpidemics } from "../bridge/tauri";
import type { EpidemicBrief } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** Phase 6 · Plagues & Epidemics — every outbreak (a contagion chain), sortable
 *  most→least deadly, expandable to its affected cities + contagion routes. Clicking
 *  a city focuses it on the map (and the plague overlay draws the spread). */

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
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [rows, setRows] = useState<EpidemicBrief[]>([]);
  const [sort, setSort] = useState<Sort>("active");
  const [activeOnly, setActiveOnly] = useState(false);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());

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

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.plague);
  if (!open) return null;

  const totalDead = rows.reduce((s, e) => s + e.total_dead, 0);
  const activeCount = rows.filter((e) => e.active).length;
  const focusCity = (hubIdx: number) => {
    const id = snapshot?.hubs?.[hubIdx]?.id;
    if (id != null) setSelectedHub(id);
  };

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ display: "flex", alignItems: "center", marginBottom: 6, cursor: "move" }} onPointerDown={onPointerDown}>
        <span style={{ color: "#e8c0c0", fontWeight: 700, fontSize: 13 }}>☠ Plagues &amp; Epidemics</span>
        <span style={{ flex: 1 }} />
        <span data-no-drag onClick={close} title="Close" style={{ color: "#a07070", cursor: "pointer", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>

      <div style={{ display: "flex", gap: 8, marginBottom: 6, fontSize: 10, color: "#9c8080" }}>
        <span>{rows.length} outbreak{rows.length === 1 ? "" : "s"}</span>
        <span>· {activeCount} active</span>
        <span>· {totalDead.toLocaleString()} dead</span>
        <span style={{ flex: 1 }} />
        <label style={{ display: "flex", alignItems: "center", gap: 3, cursor: "pointer" }} data-no-drag>
          <input type="checkbox" checked={showZones} onChange={(e) => setOverlayVisible("plagueZones", e.target.checked)}
            style={{ accentColor: "#c05050", width: 11, height: 11 }} />
          <span>Show on map</span>
        </label>
      </div>

      <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 6 }} data-no-drag>
        {SORTS.map((s) => (
          <span key={s.id} onClick={() => setSort(s.id)}
            style={{ fontSize: 10, padding: "2px 8px", borderRadius: 20, cursor: "pointer",
              border: `1px solid ${sort === s.id ? "#c05858" : "#3a2626"}`,
              background: sort === s.id ? "#301818" : "transparent",
              color: sort === s.id ? "#f0cccc" : "#9c7676" }}>{s.label}</span>
        ))}
        <span onClick={() => setActiveOnly((v) => !v)}
          style={{ fontSize: 10, padding: "2px 8px", borderRadius: 20, cursor: "pointer",
            border: `1px solid ${activeOnly ? "#c05858" : "#3a2626"}`,
            background: activeOnly ? "#301818" : "transparent",
            color: activeOnly ? "#f0cccc" : "#9c7676" }}>Active only</span>
      </div>

      <div style={{ overflowY: "auto", flex: 1 }}>
        {!active && <div style={empty}>Start the campaign to track epidemics.</div>}
        {active && shown.length === 0 && <div style={empty}>No plagues recorded yet.</div>}
        {shown.map((e) => {
          const isOpen = expanded.has(e.id);
          return (
            <div key={e.id} style={{ borderBottom: "1px solid #2a1818" }}>
              <div onClick={() => setExpanded((prev) => { const n = new Set(prev); n.has(e.id) ? n.delete(e.id) : n.add(e.id); return n; })}
                style={{ display: "flex", alignItems: "center", gap: 6, padding: "5px 2px", cursor: "pointer" }}>
                <span style={{ color: "#7a5656", fontSize: 9, width: 9 }}>{isOpen ? "▾" : "▸"}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: "#e8c8c8", fontSize: 11, fontWeight: 600 }}>
                    {e.name} <CategoryTag category={e.category} />
                    {e.active && <span style={{ color: "#ff8a6a", fontSize: 9 }}> ● active</span>}
                  </div>
                  <div style={{ color: "#8c6a6a", fontSize: 9 }}>
                    Year {e.start_year}{e.end_year > e.start_year ? `–${e.end_year}` : ""} · {e.cities.length} cit{e.cities.length === 1 ? "y" : "ies"}
                  </div>
                </div>
                <div style={{ textAlign: "right" }}>
                  <div style={{ color: "#ff8a6a", fontSize: 12, fontWeight: 700 }}>{e.total_dead.toLocaleString()}</div>
                  <div style={{ color: "#7a5656", fontSize: 8 }}>died</div>
                </div>
              </div>
              {isOpen && (
                <div style={{ paddingLeft: 14, paddingBottom: 5 }}>
                  <div style={{ fontSize: 9, color: "#8c6a6a", marginBottom: 3 }}>
                    Began in <b style={{ color: "#e8a0a0" }}>{e.origin_name}</b> · spread to {e.cities.length - 1} more
                  </div>
                  {/* Cities in SPREAD ORDER (origin first) — the contagion history. */}
                  {e.cities.map((c) => (
                    <div key={c.hub} onClick={() => focusCity(c.hub)}
                      style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 10, padding: "2px 4px", cursor: "pointer" }}>
                      <span style={{ color: "#7a5656", fontSize: 8, minWidth: 26 }}>
                        {c.order === 0 ? "★ src" : `→ ${c.order}`}
                      </span>
                      <span style={{ flex: 1, color: "#c8a8a8" }}>
                        {c.name} <span style={{ color: "#6a5050" }}>Yr{c.year}</span>
                        {c.active && <span style={{ color: "#ff8a6a" }}> · quarantined</span>}
                        {c.from_name && <span style={{ color: "#6a5050" }}> ← {c.from_name}</span>}
                      </span>
                      <span style={{ color: "#e08a6a", minWidth: 44, textAlign: "right" }}>{c.deaths.toLocaleString()} †</span>
                      <span style={{ color: "#7a8a6a", minWidth: 40, textAlign: "right" }}>{c.pop.toLocaleString()} ✚</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

/** Plague severity badge: Cat 1 = Great Plague (rare, reaches ~4000 km along the trade
 *  lanes), Cat 2 = Regional (reaches one further city), Cat 3 = Local outbreak. */
function CategoryTag({ category }: { category: number }) {
  const meta = category === 1
    ? { label: "Cat 1 · Great Plague", bg: "#4a1010", bd: "#c04040", fg: "#ffb0a0" }
    : category === 2
    ? { label: "Cat 2 · Regional", bg: "#3a2410", bd: "#b07030", fg: "#f0c890" }
    : { label: "Cat 3 · Local", bg: "#25301a", bd: "#5a7040", fg: "#bcd8a0" };
  return (
    <span style={{ fontSize: 8, fontWeight: 700, padding: "1px 5px", borderRadius: 10,
      border: `1px solid ${meta.bd}`, background: meta.bg, color: meta.fg, whiteSpace: "nowrap" }}>
      {meta.label}
    </span>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 70, right: 12, width: 340, maxHeight: "72%",
  display: "flex", flexDirection: "column",
  border: "1px solid #3a2020", borderRadius: 8, padding: "10px 12px", zIndex: 117,
  boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
const empty: React.CSSProperties = { color: "#8c6a6a", fontSize: 11, padding: "8px 2px" };
