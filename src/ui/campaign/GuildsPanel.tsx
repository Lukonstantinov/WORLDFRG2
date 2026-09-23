import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetGuilds, campaignGuildAtlas } from "@bridge";
import type { GuildBrief, GuildAtlas } from "@types";
import { GOOD_DEFS } from "@goods";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Chip, Meter, EmptyNote } from "@ui/kit";

/** Phase 6 · Guilds & Crafts — every craft guild, its luxury/manufactured good, the
 *  quality it masters, its output amount and standing, and whether it has raised a
 *  guildhall. Sortable; a row focuses its city (and the guild-city map overlay).
 *  Built on the shared UI kit (src/ui/kit.tsx). */

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
  // S11 · the Craft Atlas — inputs/outputs/reach for one expanded guild row,
  // fetched on demand (not for every row, which would be 12+ queries at once).
  const [expanded, setExpanded] = useState<number | null>(null);
  const [atlas, setAtlas] = useState<GuildAtlas | null>(null);

  useEffect(() => {
    if (!open || !active) return;
    campaignGetGuilds().then(setRows).catch(() => setRows([]));
  }, [open, active, tick]);

  useEffect(() => {
    if (expanded == null) { setAtlas(null); return; }
    let alive = true;
    campaignGuildAtlas(expanded).then((a) => { if (alive) setAtlas(a); }).catch(() => setAtlas(null));
    return () => { alive = false; };
  }, [expanded]);

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
    <Panel onPointerDown={onPointerDown} width={320} maxHeight="72%" style={{ top: 70, right: 12, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="🔨" title="Crafts & Guilds" onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: `${SPACE.md}px ${SPACE.lg}px ${SPACE.lg}px` }}>
        <div style={{ display: "flex", gap: SPACE.md, marginBottom: SPACE.md, fontSize: FZ.small, color: T.inkMid, alignItems: "center" }}>
          <span>{rows.length} guild{rows.length === 1 ? "" : "s"}</span>
          <span>· {halls} guildhall{halls === 1 ? "" : "s"}</span>
          <span style={{ flex: 1 }} />
          <label style={{ display: "flex", alignItems: "center", gap: 3, cursor: "pointer" }} data-no-drag>
            <input type="checkbox" checked={showCities} onChange={(e) => setOverlayVisible("guildCities", e.target.checked)}
              style={{ accentColor: T.gold, width: 11, height: 11 }} />
            <span>Show on map</span>
          </label>
        </div>

        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: SPACE.md }} data-no-drag>
          {SORTS.map((s) => (
            <Chip key={s.id} on={sort === s.id} onClick={() => setSort(s.id)}>{s.label}</Chip>
          ))}
          <Chip on={luxuryOnly} onClick={() => setLuxuryOnly((v) => !v)}>Luxuries</Chip>
        </div>

        <div style={{ overflowY: "auto", flex: 1 }}>
          {!active && <EmptyNote>Start the campaign to see craft guilds.</EmptyNote>}
          {active && shown.length === 0 && <EmptyNote>No craft guilds yet.</EmptyNote>}
          {shown.map((g) => {
            const isOpen = g.idx != null && expanded === g.idx;
            return (
            <div key={`${g.hub}-${g.good}`} style={{ borderBottom: `1px dashed ${T.lineSoft}` }}>
              <div onClick={() => focusCity(g.hub)}
                style={{ display: "flex", alignItems: "center", gap: SPACE.md, padding: "5px 2px", cursor: "pointer" }}>
                <span style={{ fontSize: FZ.title, width: 20, textAlign: "center" }}>{EMOJI[g.good_name] ?? "🏭"}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: g.exceptional ? T.gold : T.parchment, fontSize: FZ.body, fontWeight: g.exceptional ? 700 : 400 }}>
                    {g.exceptional ? g.brand : g.good_name}
                    {g.exceptional && <span title="Exceptional craft — a renowned brand" style={{ color: T.gold, fontSize: FZ.tiny }}> ★</span>}
                    {g.luxury && <span style={{ color: "#d0a0d0", fontSize: FZ.tiny }}> · luxury</span>}
                    {g.hall && <span title="Guildhall raised" style={{ marginLeft: 4 }}>🏛</span>}
                  </div>
                  <div style={{ color: T.inkDim, fontSize: FZ.tiny }}>
                    {g.city}{g.culture ? <span style={{ color: T.inkFaint }}> · {g.culture}</span> : null}
                  </div>
                  <div style={{ marginTop: 3, width: "80%" }}>
                    <Meter value={g.strength} color={T.gold} height={3} track={T.raised} />
                  </div>
                </div>
                <div style={{ textAlign: "right" }}>
                  <div style={{ color: T.gold, fontSize: FZ.base, fontWeight: 700 }}>{Math.round(g.quality * 100)}%</div>
                  <div style={{ color: T.inkDim, fontSize: FZ.micro }}>{fmt(g.output)}/day</div>
                </div>
                {g.idx != null && (
                  <span data-no-drag title="Craft Atlas — where the raws come from, where it goes"
                    onClick={(e) => { e.stopPropagation(); setExpanded(isOpen ? null : g.idx!); }}
                    style={{ fontSize: FZ.tiny, color: isOpen ? T.gold : T.inkFaint, cursor: "pointer", padding: "0 2px" }}>
                    🗺
                  </span>
                )}
              </div>
              {isOpen && <CraftAtlasStrip atlas={atlas} good={g.good_name} />}
            </div>
          );})}
        </div>
      </PanelBody>
    </Panel>
  );
}

/** S11 (HOUSES_GUILDS_AND_MARKET_PLAN.md) · the Craft Atlas, text form — where
 *  a guild's raws come from, where the finished good goes, its reach (other
 *  hubs currently buying it), and its earned signature. `campaign_guild_
 *  atlas` is a pure derived read (§5.6), so this can never move a gate. Same
 *  scoping note as the House Atlas tab: the on-map lane version is queued
 *  (Q19), not attempted here. */
function CraftAtlasStrip({ atlas, good }: { atlas: GuildAtlas | null; good: string }) {
  if (!atlas) {
    return <div style={{ color: T.inkFaint, fontSize: FZ.tiny, padding: "4px 8px 8px 30px" }}>Loading…</div>;
  }
  const names = (ps: { name: string }[]) => ps.slice(0, 4).map((p) => p.name).join(", ") || "none currently";
  return (
    <div style={{ padding: "2px 8px 8px 30px", fontSize: FZ.tiny, color: T.inkMid }}>
      {atlas.signature && (
        <div style={{ color: T.gold, marginBottom: 2 }}>"{atlas.signature}" — an earned signature</div>
      )}
      <div>◂ inputs from: {names(atlas.inputs)}</div>
      <div>▸ {good} shipped to: {names(atlas.outputs)}</div>
      <div style={{ color: T.inkFaint }}>reach: {atlas.reach.length} hub{atlas.reach.length === 1 ? "" : "s"} buying it</div>
    </div>
  );
}

function fmt(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "k";
  return Math.round(n).toString();
}
