import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetGuilds, campaignGuildAtlas } from "@bridge";
import type { GuildBrief, GuildAtlas, AtlasPartner } from "@types";
import { GOOD_DEFS } from "@goods";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE, SERIF, RADIUS } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Chip, Meter, EmptyNote, Stat, StatGrid, Tabs, FootNote } from "@ui/kit";

/** Crafts & Guilds — every craft guild as a production dashboard.
 *  Top: what the world's workshops make, by craft. Then either the guild list
 *  (ranked, searchable) or the crafts view (one row per good, all its guilds
 *  pooled). A guild row opens its FLOW: the raws arriving, the workshop, and
 *  where the finished good is shipped — read live from `campaign_guild_atlas`,
 *  so it shows cargo actually on the road, never an invented lane. */

type Sort = "quality" | "output" | "strength" | "tradition";
type View = "guilds" | "crafts";
const SORTS: { id: Sort; label: string }[] = [
  { id: "output", label: "Most made" },
  { id: "quality", label: "Finest" },
  { id: "strength", label: "Strongest" },
];
const PAGE = 60;

const BY_NAME = new Map(GOOD_DEFS.map((g) => [g.name, g]));
const goodLabel = (n: string) => BY_NAME.get(n)?.label ?? prettify(n);
const goodEmoji = (n: string) => BY_NAME.get(n)?.emoji ?? "🏭";
const goodColor = (n: string) => BY_NAME.get(n)?.color ?? T.gold;
const labelAt = (i: number) => GOOD_DEFS[i]?.label ?? `good ${i}`;
const emojiAt = (i: number) => GOOD_DEFS[i]?.emoji ?? "📦";

/** The app's own grade vocabulary (`deposits::grade_label`) — a word reads faster than 94% vs 97%. */
function grade(q: number): { word: string; color: string } {
  if (q >= 0.9) return { word: "exquisite", color: "#e8c35a" };
  if (q >= 0.75) return { word: "fine", color: "#9fd07a" };
  if (q >= 0.55) return { word: "good", color: "#7ab8e0" };
  if (q >= 0.35) return { word: "ordinary", color: T.inkMid };
  return { word: "coarse", color: T.inkDim };
}

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
  const [view, setView] = useState<View>("guilds");
  const [sort, setSort] = useState<Sort>("output");
  const [luxuryOnly, setLuxuryOnly] = useState(false);
  const [signedOnly, setSignedOnly] = useState(false);
  const [craft, setCraft] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [limit, setLimit] = useState(PAGE);
  const [expanded, setExpanded] = useState<number | null>(null);
  const [atlas, setAtlas] = useState<GuildAtlas | null>(null);

  useEffect(() => {
    if (!open || !active) return;
    campaignGetGuilds().then(setRows).catch(() => setRows([]));
  }, [open, active, tick]);

  useEffect(() => {
    if (expanded == null) { setAtlas(null); return; }
    let alive = true;
    setAtlas(null);
    campaignGuildAtlas(expanded).then((a) => { if (alive) setAtlas(a); }).catch(() => setAtlas(null));
    return () => { alive = false; };
  }, [expanded, tick]);

  useEffect(() => { setLimit(PAGE); }, [sort, luxuryOnly, signedOnly, craft, query, view]);

  const crafts = useMemo(() => {
    const m = new Map<string, { good: string; guilds: number; output: number; best: GuildBrief; luxury: boolean; signed: number }>();
    for (const g of rows) {
      const e = m.get(g.good_name);
      if (!e) m.set(g.good_name, { good: g.good_name, guilds: 1, output: g.output, best: g, luxury: g.luxury, signed: g.signature ? 1 : 0 });
      else {
        e.guilds++; e.output += g.output;
        if (g.signature) e.signed++;
        if (g.quality > e.best.quality) e.best = g;
      }
    }
    return [...m.values()].sort((a, b) => b.output - a.output);
  }, [rows]);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    let r = rows.filter((g) =>
      (!luxuryOnly || g.luxury) && (!signedOnly || !!g.signature) && (!craft || g.good_name === craft) &&
      (!q || g.city.toLowerCase().includes(q) || goodLabel(g.good_name).toLowerCase().includes(q) ||
        (g.culture ?? "").toLowerCase().includes(q) || (g.signature ?? "").toLowerCase().includes(q)));
    r = [...r].sort((a, b) => {
      switch (sort) {
        case "output": return b.output - a.output;
        case "strength": return b.strength - a.strength;
        default: return b.quality - a.quality || b.output - a.output;
      }
    });
    return r;
  }, [rows, sort, luxuryOnly, signedOnly, craft, query]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.guild);
  if (!open) return null;

  const total = rows.reduce((a, g) => a + g.output, 0);
  const signed = rows.filter((g) => g.signature).length;
  const cities = new Set(rows.map((g) => g.hub)).size;
  const maxOut = Math.max(1, ...shown.map((g) => g.output));
  const focusCity = (hubIdx: number) => {
    const id = snapshot?.hubs?.[hubIdx]?.id;
    if (id != null) setSelectedHub(id);
  };
  const topCrafts = crafts.slice(0, 8);
  const craftMax = Math.max(1, ...topCrafts.map((c) => c.output));

  return (
    <Panel onPointerDown={onPointerDown} width={430} maxHeight="82%" style={{ top: 70, right: 12, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="🔨" title="Crafts & Guilds" onDragStart={onPointerDown} onClose={close}
        right={
          <label data-no-drag style={{ display: "flex", alignItems: "center", gap: 4, cursor: "pointer", fontSize: FZ.tiny, color: T.inkMid, marginRight: 6 }}>
            <input type="checkbox" checked={showCities} onChange={(e) => setOverlayVisible("guildCities", e.target.checked)}
              style={{ accentColor: T.gold, width: 11, height: 11 }} />
            on map
          </label>
        } />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: `${SPACE.md}px ${SPACE.lg}px ${SPACE.lg}px` }}>
        {!active && <EmptyNote>Start the campaign to see craft guilds.</EmptyNote>}
        {active && rows.length === 0 && <EmptyNote>No craft guilds yet — they form once a city has practised a craft for a few years.</EmptyNote>}

        {active && rows.length > 0 && (
          <>
            <StatGrid cols={4} style={{ marginBottom: SPACE.md }}>
              <Stat label="Guilds" value={rows.length} hint={`in ${cities} cities`} />
              <Stat label="Output" value={fmt(total)} hint="units / day" tone="gold" />
              <Stat label="Crafts" value={crafts.length} hint="distinct goods" />
              <Stat label="Signatures" value={signed} hint="earned names" tone={signed ? "good" : undefined} />
            </StatGrid>

            {/* What the world's workshops make — a ranked bar per craft, click to filter. */}
            <div style={{
              padding: "7px 9px 6px", marginBottom: SPACE.md, borderRadius: RADIUS.md,
              border: `1px solid ${T.lineSoft}`, background: "linear-gradient(135deg, rgba(216,178,74,0.06), rgba(10,16,24,0.5))",
            }}>
              <div style={{ display: "flex", alignItems: "center", marginBottom: 5 }}>
                <span style={{ color: T.inkDim, fontSize: FZ.tiny, fontWeight: 600, textTransform: "uppercase", letterSpacing: 0.6 }}>
                  What the workshops make
                </span>
                <span style={{ flex: 1 }} />
                {craft && (
                  <span data-no-drag onClick={() => setCraft(null)} style={{ fontSize: FZ.tiny, color: T.gold, cursor: "pointer" }}>
                    ✕ clear filter
                  </span>
                )}
              </div>
              {topCrafts.map((c) => {
                const on = craft === c.good;
                const share = total > 0 ? c.output / total : 0;
                return (
                  <div key={c.good} data-no-drag onClick={() => { setCraft(on ? null : c.good); setView("guilds"); }}
                    title={`${c.guilds} guild${c.guilds === 1 ? "" : "s"} · finest at ${c.best.city}`}
                    style={{
                      display: "grid", gridTemplateColumns: "18px 92px 1fr 64px", gap: 6, alignItems: "center",
                      padding: "2px 3px", cursor: "pointer", borderRadius: RADIUS.sm,
                      background: on ? "rgba(255,255,255,0.07)" : "transparent",
                      opacity: craft && !on ? 0.45 : 1,
                    }}>
                    <span style={{ fontSize: FZ.base, textAlign: "center" }}>{goodEmoji(c.good)}</span>
                    <span style={{ fontSize: FZ.tiny, color: T.ink, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {goodLabel(c.good)}
                    </span>
                    <div style={{ height: 8, background: T.card, borderRadius: 3, overflow: "hidden" }}>
                      <div style={{
                        width: `${(c.output / craftMax) * 100}%`, height: "100%",
                        background: `linear-gradient(90deg, ${goodColor(c.good)}, ${goodColor(c.good)}aa)`,
                      }} />
                    </div>
                    <span style={{ fontSize: FZ.tiny, color: T.inkMid, textAlign: "right", fontVariantNumeric: "tabular-nums" }}>
                      {fmt(c.output)} <span style={{ color: T.inkFaint }}>{Math.round(share * 100)}%</span>
                    </span>
                  </div>
                );
              })}
            </div>

            <Tabs<View> tabs={[["guilds", `Guilds (${shown.length})`], ["crafts", `By craft (${crafts.length})`]]}
              active={view} onSelect={setView} style={{ marginBottom: SPACE.md }} />

            {view === "guilds" && (
              <div data-no-drag style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: SPACE.md, alignItems: "center" }}>
                <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search city, craft, people…"
                  style={{
                    flex: "1 1 140px", minWidth: 120, background: T.card, color: T.ink, border: `1px solid ${T.line}`,
                    borderRadius: RADIUS.pill, padding: "3px 10px", fontSize: FZ.small, outline: "none",
                  }} />
                {SORTS.map((s) => <Chip key={s.id} on={sort === s.id} onClick={() => setSort(s.id)}>{s.label}</Chip>)}
                <Chip on={luxuryOnly} onClick={() => setLuxuryOnly((v) => !v)}>💎 Luxury</Chip>
                <Chip on={signedOnly} onClick={() => setSignedOnly((v) => !v)}>★ Signature</Chip>
              </div>
            )}
          </>
        )}

        <div style={{ overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: 4 }}>
          {active && view === "guilds" && rows.length > 0 && shown.length === 0 && <EmptyNote>No guild matches these filters.</EmptyNote>}
          {active && view === "guilds" && shown.slice(0, limit).map((g, i) => {
            const isOpen = g.idx != null && expanded === g.idx;
            return (
              <GuildCard key={g.idx ?? `${g.hub}-${g.good}`} g={g} rank={i + 1} maxOut={maxOut} open={isOpen}
                onFocus={() => focusCity(g.hub)}
                onToggle={g.idx != null ? () => setExpanded(isOpen ? null : g.idx!) : undefined}
                atlas={isOpen ? atlas : null} onPartner={focusCity} />
            );
          })}
          {active && view === "guilds" && shown.length > limit && (
            <button data-no-drag onClick={() => setLimit((l) => l + PAGE)}
              style={{
                margin: "4px auto", padding: "4px 14px", background: "transparent", color: T.inkMid,
                border: `1px solid ${T.line}`, borderRadius: RADIUS.pill, fontSize: FZ.tiny, cursor: "pointer",
              }}>
              Show {Math.min(PAGE, shown.length - limit)} more · {shown.length - limit} hidden
            </button>
          )}

          {active && view === "crafts" && crafts.map((c) => (
            <div key={c.good} data-no-drag onClick={() => { setCraft(c.good); setView("guilds"); }}
              style={{
                display: "grid", gridTemplateColumns: "30px 1fr auto", gap: SPACE.md, alignItems: "center",
                padding: "6px 8px", borderRadius: RADIUS.md, border: `1px solid ${T.lineSoft}`,
                borderLeft: `3px solid ${goodColor(c.good)}`, background: T.card, cursor: "pointer", flexShrink: 0,
              }}>
              <span style={{ fontSize: 18, textAlign: "center" }}>{goodEmoji(c.good)}</span>
              <div style={{ minWidth: 0 }}>
                <div style={{ color: T.parchment, fontSize: FZ.body, fontWeight: 600 }}>
                  {goodLabel(c.good)}
                  {c.luxury && <span style={{ color: "#d0a0d0", fontSize: FZ.micro, marginLeft: 5 }}>LUXURY</span>}
                </div>
                <div style={{ color: T.inkDim, fontSize: FZ.tiny }}>
                  {c.guilds} guild{c.guilds === 1 ? "" : "s"} · finest at <span style={{ color: T.inkMid }}>{c.best.city}</span>
                  {c.signed > 0 && <span style={{ color: T.gold }}> · {c.signed} ★</span>}
                </div>
                <Meter value={c.output} max={crafts[0]?.output || 1} color={goodColor(c.good)} height={3} track={T.raised} style={{ marginTop: 3 }} />
              </div>
              <div style={{ textAlign: "right" }}>
                <div style={{ color: T.gold, fontSize: FZ.base, fontWeight: 700, fontFamily: SERIF }}>{fmt(c.output)}</div>
                <div style={{ color: T.inkDim, fontSize: FZ.micro }}>per day</div>
              </div>
            </div>
          ))}
        </div>
      </PanelBody>
    </Panel>
  );
}

function GuildCard({ g, rank, maxOut, open, onFocus, onToggle, atlas, onPartner }: {
  g: GuildBrief; rank: number; maxOut: number; open: boolean;
  onFocus: () => void; onToggle?: () => void; atlas: GuildAtlas | null; onPartner: (hub: number) => void;
}) {
  const gr = grade(g.quality);
  const col = goodColor(g.good_name);
  const title = g.signature || `${g.city} ${goodLabel(g.good_name).toLowerCase()}`;
  return (
    <div style={{
      borderRadius: RADIUS.md, border: `1px solid ${open ? T.lineGold : T.lineSoft}`,
      background: open ? T.raised : T.card, overflow: "hidden", flexShrink: 0,
    }}>
      <div data-no-drag onClick={onToggle ?? onFocus}
        style={{ display: "grid", gridTemplateColumns: "34px 1fr 76px", gap: SPACE.md, alignItems: "center", padding: "6px 8px", cursor: "pointer" }}>
        <div style={{
          position: "relative", width: 34, height: 34, borderRadius: RADIUS.md, display: "grid", placeItems: "center",
          fontSize: 18, background: `radial-gradient(circle at 40% 30%, ${col}55, ${col}14 70%)`, border: `1px solid ${col}66`,
        }}>
          {goodEmoji(g.good_name)}
          <span style={{
            position: "absolute", left: -4, top: -5, fontSize: FZ.micro, color: T.inkDim,
            background: T.panel, border: `1px solid ${T.line}`, borderRadius: 999, padding: "0 3px", lineHeight: 1.4,
          }}>{rank}</span>
        </div>
        <div style={{ minWidth: 0 }}>
          <div style={{
            color: g.signature ? T.gold : T.parchment, fontFamily: g.signature ? SERIF : undefined,
            fontSize: FZ.body, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
          }}>
            {g.signature && <span title="An earned signature — a name the world trades by">★ </span>}
            {cap(title)}
          </div>
          <div style={{ color: T.inkDim, fontSize: FZ.tiny, display: "flex", gap: 5, alignItems: "center", flexWrap: "wrap" }}>
            <span data-no-drag onClick={(e) => { e.stopPropagation(); onFocus(); }}
              style={{ color: T.inkMid, textDecoration: "underline dotted", cursor: "pointer" }} title="Focus this city">
              {g.city}
            </span>
            {g.culture && <span style={{ color: T.inkFaint }}>· {g.culture}</span>}
            {g.luxury && <span style={{ color: "#d0a0d0" }}>· 💎 luxury</span>}
            {g.hall && <span title="Guildhall raised">· 🏛</span>}
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, marginTop: 4 }}>
            <MiniMeter label="output" value={g.output} max={maxOut} color={col} />
            <MiniMeter label="standing" value={g.strength} max={1} color={T.accent} />
          </div>
        </div>
        <div style={{ textAlign: "right" }}>
          <div style={{ color: gr.color, fontSize: FZ.head, fontWeight: 700, fontFamily: SERIF }}>{Math.round(g.quality * 100)}%</div>
          <div style={{ color: gr.color, fontSize: FZ.micro, textTransform: "uppercase", letterSpacing: 0.4 }}>{gr.word}</div>
          <div style={{ color: T.inkMid, fontSize: FZ.tiny, marginTop: 2, fontVariantNumeric: "tabular-nums" }}>
            {fmt(g.output)}<span style={{ color: T.inkFaint }}>/day</span>
          </div>
          {onToggle && <div style={{ color: open ? T.gold : T.inkFaint, fontSize: FZ.micro, marginTop: 1 }}>{open ? "▴ flow" : "▾ flow"}</div>}
        </div>
      </div>
      {open && <CraftFlow atlas={atlas} g={g} onPartner={onPartner} />}
    </div>
  );
}

function MiniMeter({ label, value, max, color }: { label: string; value: number; max: number; color: string }) {
  return (
    <div title={`${label}: ${max === 1 ? Math.round(value * 100) + "%" : fmt(value)}`}>
      <div style={{ fontSize: FZ.micro, color: T.inkFaint, textTransform: "uppercase", letterSpacing: 0.4, lineHeight: 1.2 }}>{label}</div>
      <Meter value={value} max={max} color={color} height={4} track={T.raised} />
    </div>
  );
}

/** The guild's supply chain as a left-to-right flow: raws arriving → the
 *  workshop → where the finished good is going. Bars share one scale so the
 *  two sides compare. Everything here is cargo on the road right now. */
function CraftFlow({ atlas, g, onPartner }: { atlas: GuildAtlas | null; g: GuildBrief; onPartner: (hub: number) => void }) {
  if (!atlas) {
    return <div style={{ color: T.inkFaint, fontSize: FZ.tiny, padding: "6px 10px 10px" }}>Tracing the cargo…</div>;
  }
  const ins = [...atlas.inputs].sort((a, b) => b.weight - a.weight);
  const outs = [...atlas.outputs].sort((a, b) => b.weight - a.weight);
  const inSum = ins.reduce((a, p) => a + p.weight, 0);
  const outSum = outs.reduce((a, p) => a + p.weight, 0);
  const scale = Math.max(1, ...ins.slice(0, 5).map((p) => p.weight), ...outs.slice(0, 5).map((p) => p.weight));
  const col = goodColor(g.good_name);
  const tradition = atlas.tradition_by_year[0];

  return (
    <div style={{ padding: "4px 10px 10px", borderTop: `1px dashed ${T.lineSoft}` }}>
      {atlas.signature && (
        <div style={{
          margin: "6px 0 8px", padding: "5px 8px", borderRadius: RADIUS.sm, fontSize: FZ.small,
          background: "rgba(216,178,74,0.10)", border: `1px solid ${T.lineGold}`, color: T.gold, fontFamily: SERIF,
        }}>
          ★ “{atlas.signature}” — a name the world trades by
        </div>
      )}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 58px 1fr", gap: 6, alignItems: "stretch", marginTop: 6 }}>
        <FlowSide title="Raws arriving" empty="no raws on the road" partners={ins} scale={scale}
          color="#5fd0ff" align="right" onPartner={onPartner} showGoods />
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 2 }}>
          <div style={{ color: "#5fd0ff", fontSize: FZ.tiny }}>▶ {fmt(inSum)}</div>
          <div style={{
            width: 46, height: 46, borderRadius: "50%", display: "grid", placeItems: "center", fontSize: 22,
            background: `radial-gradient(circle, ${col}44, ${T.card} 75%)`, border: `2px solid ${col}`,
            boxShadow: `0 0 12px ${col}44`,
          }}>{goodEmoji(g.good_name)}</div>
          <div style={{ color: T.gold, fontSize: FZ.tiny, fontWeight: 700 }}>{fmt(g.output)}/d</div>
          <div style={{ color: "#ffce5f", fontSize: FZ.tiny }}>{fmt(outSum)} ▶</div>
        </div>
        <FlowSide title={`${goodLabel(g.good_name)} shipped to`} empty="nothing shipping now" partners={outs} scale={scale}
          color="#ffce5f" align="left" onPartner={onPartner} />
      </div>
      <div style={{ display: "flex", gap: SPACE.md, flexWrap: "wrap", marginTop: 8, fontSize: FZ.tiny, color: T.inkMid }}>
        <span>🌍 sold in <b style={{ color: T.ink }}>{atlas.reach.length}</b> other market{atlas.reach.length === 1 ? "" : "s"}</span>
        {tradition != null && <span>📜 <b style={{ color: T.ink }}>{tradition.toFixed(1)}</b> years of tradition</span>}
        <span>⚖ standing <b style={{ color: T.ink }}>{Math.round(g.strength * 100)}%</b></span>
      </div>
      <FootNote>Cargo on the road right now — a quiet lane can simply be between voyages.</FootNote>
    </div>
  );
}

function FlowSide({ title, empty, partners, scale, color, align, onPartner, showGoods }: {
  title: string; empty: string; partners: AtlasPartner[]; scale: number; color: string;
  align: "left" | "right"; onPartner: (hub: number) => void; showGoods?: boolean;
}) {
  const top = partners.slice(0, 5);
  const rest = partners.length - top.length;
  return (
    <div style={{ minWidth: 0 }}>
      <div style={{ fontSize: FZ.micro, color: T.inkDim, textTransform: "uppercase", letterSpacing: 0.5, textAlign: align, marginBottom: 3 }}>
        {title}
      </div>
      {top.length === 0 && <div style={{ fontSize: FZ.tiny, color: T.inkFaint, textAlign: align, fontStyle: "italic" }}>{empty}</div>}
      {top.map((p) => (
        <div key={p.hub} data-no-drag onClick={() => onPartner(p.hub)} title={`${p.name} · ${fmt(p.weight)} in transit`}
          style={{ marginBottom: 4, cursor: "pointer" }}>
          <div style={{
            display: "flex", justifyContent: align === "right" ? "flex-end" : "flex-start", gap: 4,
            fontSize: FZ.tiny, color: T.inkMid, whiteSpace: "nowrap", overflow: "hidden",
          }}>
            {showGoods && align === "right" && <span>{p.goods.slice(0, 3).map(emojiAt).join("")}</span>}
            <span style={{ overflow: "hidden", textOverflow: "ellipsis" }} title={showGoods ? p.goods.map(labelAt).join(", ") : undefined}>{p.name}</span>
            <span style={{ color: T.inkFaint }}>{fmt(p.weight)}</span>
          </div>
          <div style={{ display: "flex", justifyContent: align === "right" ? "flex-end" : "flex-start" }}>
            <div style={{
              width: `${Math.max(4, (p.weight / scale) * 100)}%`, height: 5, borderRadius: 3,
              background: align === "right" ? `linear-gradient(90deg, transparent, ${color})` : `linear-gradient(90deg, ${color}, transparent)`,
            }} />
          </div>
        </div>
      ))}
      {rest > 0 && <div style={{ fontSize: FZ.micro, color: T.inkFaint, textAlign: align }}>+{rest} more</div>}
    </div>
  );
}

function prettify(s: string): string {
  return cap(s.replace(/_/g, " "));
}
function cap(s: string): string {
  return s ? s[0].toUpperCase() + s.slice(1) : s;
}
function fmt(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "k";
  if (n >= 10) return Math.round(n).toString();
  return n.toFixed(1);
}
