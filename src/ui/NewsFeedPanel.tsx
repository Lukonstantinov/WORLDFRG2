import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetJournal } from "../bridge/tauri";
import type { JournalEntry } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** Phase D · the filterable World News feed — a global, clickable view over the
 *  campaign chronicle (the same `journal` the per-city/house timelines read).
 *  Clicking an item focuses the relevant settlement. No playback controls. */

const CATS: { id: string; label: string; icon: string; kinds: string[] }[] = [
  { id: "all", label: "All", icon: "🗞", kinds: [] },
  { id: "houses", label: "Houses", icon: "🛡", kinds: ["house", "succession", "monopoly", "office"] },
  { id: "banks", label: "Banks", icon: "🏦", kinds: ["bank"] },
  { id: "wars", label: "Wars", icon: "⚔", kinds: ["war"] },
  { id: "crashes", label: "Crashes", icon: "📉", kinds: ["crash", "speculation"] },
  { id: "holdings", label: "Holdings", icon: "🏭", kinds: ["estate", "warehouse"] },
  { id: "life", label: "Life & Faith", icon: "✦",
    kinds: ["figure", "fair", "pilgrimage", "temple", "marriage", "feud", "guildhall",
            "guild_strike", "fashion", "wonder", "diaspora"] },
  { id: "plague", label: "Plague & Sea", icon: "☠", kinds: ["contagion", "piracy"] },
];

const ICON: Record<string, string> = {
  bank: "🏦", war: "⚔", crash: "📉", speculation: "📉", estate: "🏭",
  warehouse: "🏬", house: "🛡", succession: "🛡", monopoly: "👑", office: "🏛",
  figure: "🎖", fair: "🎪", pilgrimage: "🕊", temple: "⛪",
  contagion: "☠", marriage: "💍", feud: "🗡", guild_strike: "✊", guildhall: "🏛",
  fashion: "👗", wonder: "🗿", piracy: "🏴", diaspora: "🧭",
};

export function NewsFeedPanel() {
  const open = useUIStore((s) => s.showNews);
  const close = () => useUIStore.getState().setShowNews(false);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;
  const [entries, setEntries] = useState<JournalEntry[]>([]);
  const [cat, setCat] = useState("all");

  useEffect(() => {
    if (!open || !active) return;
    campaignGetJournal(-1, -1).then(setEntries).catch(() => setEntries([]));
  }, [open, active, tick]);

  const year = (t: number) => 1 + Math.floor(t / 365);
  const shown = useMemo(() => {
    const want = CATS.find((c) => c.id === cat)?.kinds ?? [];
    const filtered = want.length === 0 ? entries : entries.filter((e) => want.includes(e.kind));
    return [...filtered].sort((a, b) => b.tick - a.tick).slice(0, 200);
  }, [entries, cat]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.news);
  if (!open) return null;

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ display: "flex", alignItems: "center", marginBottom: 6, cursor: "move" }} onPointerDown={onPointerDown}>
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13 }}>🗞 World News</span>
        <span style={{ flex: 1 }} />
        <span data-no-drag onClick={close} title="Close" style={{ color: "#7090b0", cursor: "pointer", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 6 }}>
        {CATS.map((c) => (
          <span key={c.id} onClick={() => setCat(c.id)}
            style={{ fontSize: 10.5, padding: "2px 8px", borderRadius: 20, cursor: "pointer",
              border: `1px solid ${cat === c.id ? "#3a80c0" : "#22364c"}`,
              background: cat === c.id ? "#11283f" : "transparent",
              color: cat === c.id ? "#cfe2f6" : "#6a86a6" }}>
            {c.icon} {c.label}
          </span>
        ))}
      </div>
      <div style={{ overflowY: "auto", flex: 1 }}>
        {!active && <div style={empty}>Start the campaign to see world news.</div>}
        {active && shown.length === 0 && <div style={empty}>Nothing in this category yet.</div>}
        {shown.map((e, i) => {
          const hub = e.hub >= 0 ? snapshot?.hubs?.[e.hub] : undefined;
          return (
            <div key={i} onClick={() => hub && setSelectedHub(hub.id)}
              style={{ display: "flex", gap: 8, padding: "5px 2px", borderBottom: "1px dashed #18283a",
                cursor: hub ? "pointer" : "default" }}>
              <span style={{ fontSize: 13, width: 20, textAlign: "center" }}>{ICON[e.kind] ?? "•"}</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ color: "#cfe0f4", fontSize: 10.5, lineHeight: 1.35 }}>{e.text}</div>
                <div style={{ color: "#6a86a6", fontSize: 9 }}>
                  Year {year(e.tick)}{hub ? ` · ${hub.name}` : ""}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 70, right: 12, width: 320, maxHeight: "70%",
  display: "flex", flexDirection: "column",
  background: "rgba(12,18,26,0.97)", border: "1px solid #1e2e42", borderRadius: 8,
  padding: "10px 12px", zIndex: 116, boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
const empty: React.CSSProperties = { color: "#6a86a6", fontSize: 11, padding: "8px 2px" };
