import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetGuilds } from "../bridge/tauri";
import type { GuildBrief } from "../types";
import { GOOD_DEFS } from "../goods";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** Phase 6 · Guilds & Crafts — every craft guild, its luxury/manufactured good, the
 *  quality it masters, its output amount and standing, and whether it has raised a
 *  guildhall. Sortable; a row focuses its city (and the guild-city map overlay). */

type Sort = "quality" | "output" | "strength";
const SORTS: { id: Sort; label: string }[] = [
  { id: "quality", label: "Finest" },
  { id: "output", label: "Most made" },
  { id: "strength", label: "Strongest" },
];

const EMOJI: Record<string, string> = Object.fromEntries(GOOD_DEFS.map((g) => [g.name, g.emoji]));

export function GuildsPanel() {
  const open = useUIStore((s) => s.showGuilds);
  const close = () => useUIStore.getState().setShowGuilds(false);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const showCities = useUIStore((s) => s.overlayVisibility.guildCities);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [rows, setRows] = useState<GuildBrief[]>([]);
  const [sort, setSort] = useState<Sort>("quality");
  const [luxuryOnly, setLuxuryOnly] = useState(false);

  useEffect(() => {
    if (!open || !active) return;
    campaignGetGuilds().then(setRows).catch(() => setRows([]));
  }, [open, active, tick]);

  const shown = useMemo(() => {
    let r = luxuryOnly ? rows.filter((g) => g.luxury) : rows;
    r = [...r].sort((a, b) => {
      switch (sort) {
        case "output": return b.output - a.output;
        case "strength": return b.strength - a.strength;
        default: return b.quality - a.quality;
      }
    });
    return r;
  }, [rows, sort, luxuryOnly]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.guild);
  if (!open) return null;

  const halls = rows.filter((g) => g.hall).length;
  const focusCity = (hubIdx: number) => {
    const id = snapshot?.hubs?.[hubIdx]?.id;
    if (id != null) setSelectedHub(id);
  };

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ display: "flex", alignItems: "center", marginBottom: 6, cursor: "move" }} onPointerDown={onPointerDown}>
        <span style={{ color: "#e8d8b0", fontWeight: 700, fontSize: 13 }}>🏛 Guilds &amp; Crafts</span>
        <span style={{ flex: 1 }} />
        <span data-no-drag onClick={close} title="Close" style={{ color: "#a89060", cursor: "pointer", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>

      <div style={{ display: "flex", gap: 8, marginBottom: 6, fontSize: 10, color: "#9c9070" }}>
        <span>{rows.length} guild{rows.length === 1 ? "" : "s"}</span>
        <span>· {halls} guildhall{halls === 1 ? "" : "s"}</span>
        <span style={{ flex: 1 }} />
        <label style={{ display: "flex", alignItems: "center", gap: 3, cursor: "pointer" }} data-no-drag>
          <input type="checkbox" checked={showCities} onChange={(e) => setOverlayVisible("guildCities", e.target.checked)}
            style={{ accentColor: "#c0a040", width: 11, height: 11 }} />
          <span>Show on map</span>
        </label>
      </div>

      <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 6 }} data-no-drag>
        {SORTS.map((s) => (
          <span key={s.id} onClick={() => setSort(s.id)}
            style={{ fontSize: 10, padding: "2px 8px", borderRadius: 20, cursor: "pointer",
              border: `1px solid ${sort === s.id ? "#c0a040" : "#3a3220"}`,
              background: sort === s.id ? "#2a2410" : "transparent",
              color: sort === s.id ? "#f0e0b0" : "#9c9070" }}>{s.label}</span>
        ))}
        <span onClick={() => setLuxuryOnly((v) => !v)}
          style={{ fontSize: 10, padding: "2px 8px", borderRadius: 20, cursor: "pointer",
            border: `1px solid ${luxuryOnly ? "#c0a040" : "#3a3220"}`,
            background: luxuryOnly ? "#2a2410" : "transparent",
            color: luxuryOnly ? "#f0e0b0" : "#9c9070" }}>Luxuries</span>
      </div>

      <div style={{ overflowY: "auto", flex: 1 }}>
        {!active && <div style={empty}>Start the campaign to see craft guilds.</div>}
        {active && shown.length === 0 && <div style={empty}>No craft guilds yet.</div>}
        {shown.map((g) => (
          <div key={`${g.hub}-${g.good}`} onClick={() => focusCity(g.hub)}
            style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 2px",
              borderBottom: "1px dashed #241f12", cursor: "pointer" }}>
            <span style={{ fontSize: 15, width: 20, textAlign: "center" }}>{EMOJI[g.good_name] ?? "🏺"}</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ color: "#e8dcc0", fontSize: 11 }}>
                {g.good_name}{g.luxury && <span style={{ color: "#d0a0d0", fontSize: 9 }}> · luxury</span>}
                {g.hall && <span title="Guildhall raised" style={{ marginLeft: 4 }}>🏛</span>}
              </div>
              <div style={{ color: "#8c8060", fontSize: 9 }}>{g.city}</div>
              {/* strength bar */}
              <div style={{ height: 3, background: "#241f12", borderRadius: 2, marginTop: 3, width: "80%" }}>
                <div style={{ width: `${Math.round(g.strength * 100)}%`, height: "100%", background: "#c0a040", borderRadius: 2 }} />
              </div>
            </div>
            <div style={{ textAlign: "right" }}>
              <div style={{ color: "#e0c060", fontSize: 12, fontWeight: 700 }}>{Math.round(g.quality * 100)}%</div>
              <div style={{ color: "#7a7050", fontSize: 8 }}>{fmt(g.output)}/day</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function fmt(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "k";
  return Math.round(n).toString();
}

const panel: React.CSSProperties = {
  position: "absolute", top: 70, right: 12, width: 320, maxHeight: "72%",
  display: "flex", flexDirection: "column",
  border: "1px solid #3a3220", borderRadius: 8, padding: "10px 12px", zIndex: 117,
  boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
const empty: React.CSSProperties = { color: "#8c8060", fontSize: 11, padding: "8px 2px" };
