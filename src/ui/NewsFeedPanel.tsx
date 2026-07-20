import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetJournal } from "@bridge";
import type { JournalEntry } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";
import { T, FZ, SPACE } from "./chronicleTheme";
import { Panel, PanelHeader, PanelBody, Chip, EmptyNote } from "./kit";

/** Phase D · the filterable World News feed — a global, clickable view over the
 *  campaign chronicle (the same `journal` the per-city/house timelines read).
 *  Clicking an item focuses the relevant settlement. No playback controls.
 *  Built on the shared UI kit (src/ui/kit.tsx). */

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
    <Panel width={320} maxHeight="70%" style={{ top: 70, right: 12, zIndex: 116, ...rootStyle }}>
      <PanelHeader icon="🗞" title="World News" onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: `${SPACE.md}px ${SPACE.lg}px ${SPACE.lg}px` }}>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: SPACE.md }}>
          {CATS.map((c) => (
            <Chip key={c.id} on={cat === c.id} onClick={() => setCat(c.id)}>
              {c.icon} {c.label}
            </Chip>
          ))}
        </div>
        <div style={{ overflowY: "auto", flex: 1 }}>
          {!active && <EmptyNote>Start the campaign to see world news.</EmptyNote>}
          {active && shown.length === 0 && <EmptyNote>Nothing in this category yet.</EmptyNote>}
          {shown.map((e, i) => {
            const hub = e.hub >= 0 ? snapshot?.hubs?.[e.hub] : undefined;
            return (
              <div key={i} onClick={() => hub && setSelectedHub(hub.id)}
                style={{ display: "flex", gap: SPACE.md, padding: "5px 2px", borderBottom: `1px dashed ${T.lineSoft}`,
                  cursor: hub ? "pointer" : "default" }}>
                <span style={{ fontSize: FZ.head, width: 20, textAlign: "center" }}>{ICON[e.kind] ?? "•"}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: T.ink, fontSize: FZ.small, lineHeight: 1.35 }}>{e.text}</div>
                  <div style={{ color: T.inkDim, fontSize: FZ.tiny }}>
                    Year {year(e.tick)}{hub ? ` · ${hub.name}` : ""}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </PanelBody>
    </Panel>
  );
}
