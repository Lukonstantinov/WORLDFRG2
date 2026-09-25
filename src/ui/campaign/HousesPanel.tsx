import { useEffect, useState } from "react";
import { useCampaignStore } from "@state/campaignStore";
import { useUIStore } from "@state/uiStore";
import { CoatOfArms } from "@ui/heraldry/CoatOfArms";
import { CoinIcon } from "@ui/heraldry/CoinIcon";
import { HouseDetail, HouseTimeline } from "@ui/campaign/HouseDossier";
import { HouseCompareWindow } from "@ui/campaign/HouseCompare";
import { goodIcon, TIER_META, tierOf, dull } from "@ui/campaign/houseShared";
import { clarifyGemLabel } from "@goods";
import { campaignGetHouseHistory, campaignHouseBumpChart, campaignGetInequality, campaignGetJournal, campaignHouseAtlas } from "@bridge";
import type { HouseHistory, CampaignDiagnostics, HouseBrief, BumpChart, BumpLine, InequalitySnapshot, JournalEntry, HouseAtlas, AtlasGoodBook } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";

/** ⚜️ Merchant Houses / 🏛 Merchant Companies — the two BROWSE windows.
 *
 *  They used to be one window with a filter chip, a bump chart, a ticker and
 *  every one of several thousand cards rendered at once, which read as a mess
 *  (user report). Now: one window per kind (`companies` picks which), a search
 *  box, a sort, tiers that page 40 cards at a time, and no chart. The dossier
 *  itself is app-wide (`HouseDossierHost`), so the Government tab, a charter or
 *  a feud can open a house without this window being open. */
export function HousesPanel({ companies = false }: { companies?: boolean }) {
  const open = useUIStore((s) => companies ? s.showCompanies : s.showHouses);
  const houses = useCampaignStore((s) => s.houses);
  const diag = useCampaignStore((s) => s.diagnostics);
  const [history, setHistory] = useState<HouseHistory | null>(null);
  const [compareOpen, setCompareOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<"wealth" | "standing" | "name">("wealth");
  const [collapsedTiers, setCollapsedTiers] = useState<Record<number, boolean>>({ 3: true, 4: true });
  const [shown, setShown] = useState<Record<string, number>>({});
  const [showFallen, setShowFallen] = useState(false);
  const toggleTier = (t: number) => setCollapsedTiers((c) => ({ ...c, [t]: !c[t] }));
  const PAGE = 40;
  const more = (key: string) => setShown((m) => ({ ...m, [key]: (m[key] ?? PAGE) + PAGE }));
  const selectHouse = (h: HouseBrief | null) => useUIStore.getState().setDossierHouse(h?.idx ?? null);
  const close = () => companies ? useUIStore.getState().setShowCompanies(false) : useUIStore.getState().setShowHouses(false);
  const openTimeline = (name: string) => {
    campaignGetHouseHistory(name).then((h) => setHistory(h)).catch(() => setHistory(null));
  };
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.houses);
  if (!open) return null;

  const q = query.trim().toLowerCase();
  const matches = (h: HouseBrief) => !q
    || h.name.toLowerCase().includes(q)
    || (h.home_name ?? "").toLowerCase().includes(q)
    || (h.head_name ?? "").toLowerCase().includes(q)
    || h.specialties.some((g) => g.toLowerCase().includes(q));
  const kind = houses.filter((h) => companies === !!h.is_guild);
  const active = kind.filter((h) => !h.defunct && matches(h));
  const gone = kind.filter((h) => h.defunct && matches(h));
  const sorter = (a: HouseBrief, b: HouseBrief) =>
    sort === "name" ? a.name.localeCompare(b.name)
    : sort === "standing" ? (b.standing ?? 0) - (a.standing ?? 0)
    : b.wealth - a.wealth;
  active.sort(sorter);
  const maxWealth = Math.max(1, ...active.map((h) => h.wealth));
  const fmtWealth = (v: number) => (v >= 1e6 ? `${(v / 1e6).toFixed(1)}M` : v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));

  const renderHouseCard = (h: HouseBrief, i: number) => {
    const accent = h.is_guild ? dull(h.color ?? "") : (h.color ?? "#888");
    const sea = h.fleet_sea ?? 0, river = h.fleet_river ?? 0, car = h.fleet_caravan ?? 0;
    const fleetTotal = sea + river + car;
    const goods = (h.top_goods && h.top_goods.length ? h.top_goods : h.specialties).slice(0, 3);
    return (
      <div key={h.name + i} style={{ ...card, cursor: "pointer" }} onClick={() => selectHouse(h)}
        onMouseEnter={(e) => { e.currentTarget.style.background = "#152234"; }}
        onMouseLeave={(e) => { e.currentTarget.style.background = "#101a26"; }}
        title="Open this family's dossier">
        <span style={{ position: "absolute", left: 0, top: 6, bottom: 6, width: 3, borderRadius: 2, background: accent }} />
        <CoatOfArms name={h.name} size={28} guild={h.is_guild} />
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span style={{ color: "#f0e4c8", fontWeight: 700, fontSize: 12.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {h.name}
            </span>
            {h.owns_bank && <span title="Owns a chartered bank" style={{ fontSize: 10.5 }}>🏦</span>}
            {h.coin_name && <CoinIcon issuer={h.name} value={h.coin_value} size={12} title={`Mints the ${h.coin_name}`} />}
            {h.dominant && <span title="Controls its seat city" style={{ fontSize: 10.5 }}>⚖</span>}
            <span style={{ flex: 1 }} />
            <span style={{ color: "#e0c060", fontWeight: 700, fontSize: 11.5, fontVariantNumeric: "tabular-nums" }}>{fmtWealth(h.wealth)}</span>
          </div>
          <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginTop: 1, fontSize: 10, color: "#7a90a8",
            overflow: "hidden", whiteSpace: "nowrap" }}>
            <span>🏙 {h.home_name}</span>
            <span style={{ color: "#465870" }}>· {h.head_name}</span>
            <span style={{ flex: 1 }} />
            {goods.map((g) => <span key={g} title={g}>{goodIcon(g)}</span>)}
            {fleetTotal > 0 && <span title="fleet">{sea > 0 && `🚢${sea}`}{river > 0 && ` 🛶${river}`}{car > 0 && ` 🐫${car}`}</span>}
            {(h.offices?.length ?? 0) > 0 && <span title={`${h.offices!.length} offices abroad`}>🏢{h.offices!.length}</span>}
            {h.rivals.length > 0 && <span title={`Feuding: ${h.rivals.join(", ")}`} style={{ color: "#e0a89a" }}>⚔{h.rivals.length}</span>}
          </div>
          <div style={{ height: 2, background: "#0a1018", borderRadius: 2, overflow: "hidden", marginTop: 3 }}>
            <div style={{ width: `${(h.wealth / maxWealth) * 100}%`, height: "100%", background: accent }} />
          </div>
        </div>
      </div>
    );
  };

  const pageOf = (key: string, list: HouseBrief[]) => {
    const n = shown[key] ?? PAGE;
    return (
      <>
        {list.slice(0, n).map((h, i) => renderHouseCard(h, i))}
        {list.length > n && (
          <div data-no-drag onClick={() => more(key)}
            style={{ textAlign: "center", color: "#7fb2d8", fontSize: 10.5, padding: "5px 0", cursor: "pointer" }}>
            show {Math.min(PAGE, list.length - n)} more of {list.length - n} ▾
          </div>
        )}
      </>
    );
  };

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }} onPointerDown={onPointerDown}>
      {history && <HouseTimeline history={history} onClose={() => setHistory(null)} />}
      {compareOpen && <HouseCompareWindow houses={houses} onClose={() => setCompareOpen(false)} />}
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>{companies ? "🏛 Merchant Companies" : "⚜️ Merchant Houses"}
          <span style={{ color: "#6a7e96", fontWeight: 400, fontSize: 11 }}> · {kind.filter((h) => !h.defunct).length} active</span>
        </span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>
      {companies && (
        <div style={{ padding: "5px 10px", fontSize: 10, color: "#6f8aa6", borderBottom: "1px solid #1e2e42" }}>
          Civic firms chartered by their city — they trade in the city&apos;s interest, draw a civic subsidy and never go bankrupt.
        </div>
      )}
      {/* Search · sort · links — one compact row */}
      <div data-no-drag style={{ display: "flex", alignItems: "center", gap: 6, padding: "6px 10px", borderBottom: "1px solid #1e2e42" }}>
        <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="🔎 name, city, head or good…"
          style={{ flex: 1, minWidth: 0, background: "#0b131d", border: "1px solid #24384e", borderRadius: 5,
            color: "#dfe8f2", fontSize: 11, padding: "4px 7px" }} />
        <select value={sort} onChange={(e) => setSort(e.target.value as typeof sort)}
          style={{ background: "#0b131d", border: "1px solid #24384e", borderRadius: 5, color: "#cfe2f6", fontSize: 10.5, padding: "3px 4px" }}>
          <option value="wealth">wealth</option>
          <option value="standing">standing</option>
          <option value="name">name</option>
        </select>
        {!companies && (
          <div onClick={() => useUIStore.getState().setShowFeuds(true)} title="Open the world Feuds & Alliances board"
            style={{ padding: "3px 8px", cursor: "pointer", fontSize: 10.5, color: "#e0a89a", border: "1px solid #3a2420", borderRadius: 5 }}>
            ⚔ Feuds
          </div>
        )}
        <div onClick={() => setCompareOpen(true)} title="Compare two houses side by side"
          style={{ padding: "3px 8px", cursor: "pointer", fontSize: 10.5, color: "#cfe2f6", border: "1px solid #2e4864", borderRadius: 5 }}>
          ⚖ Compare
        </div>
      </div>
      {diag && !companies && <TradeDiagnostics diag={diag} />}
      <div style={{ overflowY: "auto", padding: "4px 8px 10px" }}>
        {houses.length === 0 && (
          <div style={empty}>Begin the campaign (Step 11) — trading families rise as goods start to move.</div>
        )}
        {houses.length > 0 && active.length === 0 && (
          <div style={empty}>{q ? "Nothing matches that search." : companies
            ? "No civic companies yet (cities form one at 50,000 people)." : "No private houses yet."}</div>
        )}
        {!companies && !q ? (
          ([1, 2, 3, 4] as const).map((t) => {
            const group = active.filter((h) => tierOf(h) === t);
            if (group.length === 0) return null;
            const meta = TIER_META[t];
            const collapsed = collapsedTiers[t];
            return (
              <div key={`tier-${t}`}>
                <div data-no-drag onClick={() => toggleTier(t)} title={meta.band}
                  style={{ display: "flex", alignItems: "center", gap: 7, cursor: "pointer",
                    margin: "8px 0 5px", padding: "4px 9px", borderRadius: 6,
                    background: t === 1 ? "linear-gradient(90deg, #1c1608, #120e06)" : "#0e1622",
                    border: `1px solid ${t === 1 ? "#3a2e10" : "#1a2838"}` }}>
                  <span style={{ color: "#c9a227", fontSize: 12 }}>{meta.glyph}</span>
                  <span style={{ color: t === 1 ? "#e8d090" : "#9ab0c8", fontSize: 10.5, fontWeight: 700, letterSpacing: 0.4 }}>
                    {meta.name.toUpperCase()}
                  </span>
                  <span style={{ color: "#4e6480", fontSize: 9.5 }}>({group.length})</span>
                  <span style={{ flex: 1 }} />
                  <span style={{ color: "#5a6a7e", fontSize: 9 }}>{collapsed ? "▸" : "▾"}</span>
                </div>
                {!collapsed && pageOf(`t${t}`, group)}
              </div>
            );
          })
        ) : (
          pageOf("all", active)
        )}
        {gone.length > 0 && (
          <>
            <div data-no-drag onClick={() => setShowFallen((v) => !v)}
              style={{ color: "#5a6a7e", fontSize: 9.5, fontWeight: 700, letterSpacing: 0.4, cursor: "pointer",
                margin: "12px 0 6px", padding: "0 2px", textTransform: "uppercase" }}>
              🪦 Fallen ({gone.length}) {showFallen ? "▾" : "▸"}
            </div>
            {showFallen && gone.slice(0, shown.fallen ?? PAGE).map((h, i) => (
              <div key={"d" + i} style={{ ...card, padding: "5px 10px", opacity: 0.6, cursor: "pointer" }} onClick={() => openTimeline(h.name)} title="View this family's timeline">
                <CoatOfArms name={h.name} size={20} guild={h.is_guild} />
                <div style={{ flex: 1, display: "flex", alignItems: "baseline", gap: 6, minWidth: 0 }}>
                  <span style={{ color: "#9aa6b4", fontSize: 11, textDecoration: "line-through", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{h.name}</span>
                  <span style={{ color: "#4a5c72", fontSize: 9.5 }}>once of {h.home_name}</span>
                </div>
              </div>
            ))}
            {showFallen && gone.length > (shown.fallen ?? PAGE) && (
              <div data-no-drag onClick={() => more("fallen")}
                style={{ textAlign: "center", color: "#7fb2d8", fontSize: 10.5, padding: "5px 0", cursor: "pointer" }}>
                show more ▾
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

/** The House Dossier, app-wide. Any view can open a house by setting
 *  `uiStore.dossierHouse` (the Government tab's charters, the Houses and
 *  Companies lists, a feud…), without the browse window having to be open. */
export function HouseDossierHost() {
  const idx = useUIStore((s) => s.dossierHouse);
  const houses = useCampaignStore((s) => s.houses);
  const setSelectedHouse = useCampaignStore((s) => s.setSelectedHouse);
  const [history, setHistory] = useState<HouseHistory | null>(null);
  useEffect(() => {
    setSelectedHouse(idx);
    // Show the House Control layer so the focused house's trade web is visible.
    if (idx != null) useUIStore.getState().setOverlayVisible("houseControl", true);
  }, [idx, setSelectedHouse]);
  const h = idx == null ? null : houses.find((x) => x.idx === idx) ?? null;
  const set = (x: HouseBrief | null) => useUIStore.getState().setDossierHouse(x?.idx ?? null);
  return (
    <>
      {history && <HouseTimeline history={history} onClose={() => setHistory(null)} />}
      {h && h.idx != null && <HouseLanesWindow houseIdx={h.idx} houseName={h.name} />}
      {h && <HouseDetail h={h} onClose={() => set(null)}
        onChronicle={(name) => campaignGetHouseHistory(name).then(setHistory).catch(() => setHistory(null))}
        onSelectHouse={set} />}
    </>
  );
}

/** S12a · the top band: a bump chart of the top houses' wealth RANK over the last
 *  ~50 years (`campaign_house_bump_chart`), plus quiet gauges beside it. A line
 *  climbing needs no caption; a line that stops is a house that died.
 *
 *  Scoping note (rule 36 discipline, matching this plan's own §9 risk-register
 *  warning about the chart's own data window): the plan asks for FOUR sparkline
 *  gauges — standing/founded/died/top-10% share. Only two of those have a real
 *  YEARLY series behind them (`InequalitySnapshot.series`'s `active` and
 *  `top10_share`); founded/died totals are cumulative counters with no per-year
 *  series persisted anywhere. Rather than fabricate a history the data does not
 *  hold, founded/died ship as plain totals and the two that DO have a series
 *  ship as real sparklines — never the other way around. */
function BumpBand({ bump, ineq, onSelect }: { bump: BumpChart | null; ineq: InequalitySnapshot | null; onSelect: (houseIdx: number) => void }) {
  if (!bump || bump.years.length < 2) return null;
  const W = 300, H = 92, PAD = 4;
  const n = bump.years.length;
  const maxRank = Math.max(1, ...bump.lines.flatMap((l) => l.ranks.filter((r) => r > 0)));
  const xAt = (i: number) => PAD + (i / (n - 1)) * (W - PAD * 2);
  const yAt = (r: number) => PAD + ((r - 1) / Math.max(1, maxRank - 1)) * (H - PAD * 2);
  const pathOf = (ranks: number[]) => {
    let d = "";
    let drawing = false;
    ranks.forEach((r, i) => {
      if (r <= 0) { drawing = false; return; }
      d += `${drawing ? "L" : "M"}${xAt(i).toFixed(1)},${yAt(r).toFixed(1)} `;
      drawing = true;
    });
    return d.trim();
  };
  const lastRanked = (l: BumpLine) => {
    for (let i = l.ranks.length - 1; i >= 0; i--) if (l.ranks[i] > 0) return i;
    return -1;
  };

  return (
    <div style={{ display: "flex", gap: 8, padding: "8px 10px", borderBottom: "1px solid #1e2e42", background: "#0a1119" }}>
      <div style={{ flex: "0 0 auto" }} title="The top houses' wealth rank over the last decades — a line crossing above another is a house overtaking it; a line that stops is a house that died">
        <svg width={W} height={H} style={{ display: "block" }}>
          {bump.lines.map((l) => {
            const end = lastRanked(l);
            return (
              <g key={l.house} style={{ cursor: "pointer" }} onClick={() => onSelect(l.house)}>
                <path d={pathOf(l.ranks)} fill="none" stroke={l.color} strokeWidth={l.tier === 1 ? 2 : 1.2}
                  opacity={l.defunct ? 0.35 : 0.9} strokeLinecap="round" strokeLinejoin="round" />
                {end >= 0 && (
                  <circle cx={xAt(end)} cy={yAt(l.ranks[end])} r={l.defunct ? 2 : 2.6} fill={l.color}
                    opacity={l.defunct ? 0.5 : 1}>
                    <title>{`${l.name} — rank ${l.ranks[end]} in ${bump.years[end]}${l.defunct ? " (fallen)" : ""}`}</title>
                  </circle>
                )}
              </g>
            );
          })}
        </svg>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 8.5, color: "#4a5c72", padding: "0 4px" }}>
          <span>{bump.years[0]}</span>
          <span title="Wealth rank, oldest → newest">rank over time</span>
          <span>{bump.years[n - 1]}</span>
        </div>
      </div>
      {ineq && (
        <div style={{ flex: 1, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 5, minWidth: 0 }}>
          <Gauge label="families" value={String(ineq.active_houses)} series={ineq.series.map((p) => p.active)} color="#8fc0e8" />
          <Gauge label="top-10% share" value={`${Math.round(ineq.top10_share_now * 100)}%`} series={ineq.series.map((p) => p.top10_share)} color="#e0c060" />
          <PlainStat label="founded (all-time)" value={String(ineq.founded_total)} color="#9fe0b8" />
          <PlainStat label="fallen (all-time)" value={String(ineq.defunct_houses)} color="#e0a09a" />
        </div>
      )}
    </div>
  );
}

/** A quiet stat WITH a history — never a number alone (§6 principle 1). */
function Gauge({ label, value, series, color }: { label: string; value: string; series: number[]; color: string }) {
  const w = 76, h = 22;
  const vals = series.filter((v) => Number.isFinite(v));
  const lo = Math.min(...vals, 0), hi = Math.max(...vals, 1);
  const pts = vals.map((v, i) => {
    const x = vals.length > 1 ? (i / (vals.length - 1)) * w : 0;
    const y = h - ((v - lo) / Math.max(1e-6, hi - lo)) * h;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  return (
    <div style={gaugeCell} title={`${label}, over the campaign so far`}>
      <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
        <span style={{ color, fontWeight: 700, fontSize: 12 }}>{value}</span>
      </div>
      {vals.length > 1 && (
        <svg width={w} height={h} style={{ display: "block", marginTop: 1 }}>
          <polyline points={pts} fill="none" stroke={color} strokeWidth={1.3} opacity={0.85} />
        </svg>
      )}
      <div style={{ color: "#5a6a7e", fontSize: 8.5 }}>{label}</div>
    </div>
  );
}

/** A quiet stat with NO history to show (founded/fallen totals — see BumpBand's
 *  own scoping note above for why these two are plain, not sparklines). */
function PlainStat({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div style={gaugeCell}>
      <span style={{ color, fontWeight: 700, fontSize: 12 }}>{value}</span>
      <div style={{ color: "#5a6a7e", fontSize: 8.5, marginTop: 1 }}>{label}</div>
    </div>
  );
}

/** S12a · the bottom pulse — a thin scroll of the last handful of house-ish
 *  world events, colour-coded by kind. Not the chronicle (that is the per-house
 *  Chronicle tab); a heartbeat so the window feels alive while time advances.
 *  Reuses the SAME `campaign_get_journal(-1,-1)` world feed `NewsFeedPanel`
 *  already reads — a sixth caller of an existing mechanism, not a new one. */
const PULSE_KINDS = new Set(["house", "succession", "monopoly", "office", "feud", "guild_founded", "guild_dissolved"]);
const PULSE_ICON: Record<string, string> = {
  house: "🛡", succession: "🛡", monopoly: "👑", office: "🏛", feud: "⚔",
  guild_founded: "🔨", guild_dissolved: "🔨",
};
function PulseTicker({ entries }: { entries: JournalEntry[] }) {
  const recent = entries.filter((e) => PULSE_KINDS.has(e.kind)).slice(-8).reverse();
  if (recent.length === 0) return null;
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 10px",
      borderTop: "1px solid #1a2a3e", background: "#080d13", overflowX: "auto", whiteSpace: "nowrap" }}
      title="The pulse — recent house events world-wide">
      <span style={{ color: "#3e4e64", fontSize: 9 }}>♥</span>
      {recent.map((e, i) => (
        <span key={i} style={{ fontSize: 9.5, color: "#7a90a8", display: "inline-flex", alignItems: "center", gap: 3 }}>
          <span>{PULSE_ICON[e.kind] ?? "•"}</span>
          <span style={{ color: "#9ab0c8" }}>{e.text}</span>
        </span>
      ))}
    </div>
  );
}

/** "Is trade actually moving?" — a compact health strip above the houses list.
 *  Answers the core merchant-house question: are shipments flowing, how many are
 *  financed by houses vs. local guilds, are voyages being lost, and how much of
 *  the world do houses actually control. */
function TradeDiagnostics({ diag }: { diag: CampaignDiagnostics }) {
  const fleet = diag.fleet_sea + diag.fleet_river + diag.fleet_caravan;
  const moving = diag.shipments_last > 0;
  const housePct = diag.shipments_last > 0
    ? Math.round((diag.by_house / diag.shipments_last) * 100) : 0;
  const stat = (label: string, value: string, color = "#cfe0f4", title?: string) => (
    <div style={diagCell} title={title}>
      <div style={{ color, fontWeight: 700, fontSize: 12 }}>{value}</div>
      <div style={{ color: "#6a86a6", fontSize: 9 }}>{label}</div>
    </div>
  );
  return (
    <div style={diagBar}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
        <span style={{ width: 7, height: 7, borderRadius: "50%", background: moving ? "#5fd08a" : "#d06a5f" }} />
        <span style={{ color: moving ? "#9fe0b8" : "#e0a09a", fontSize: 10, fontWeight: 600 }}>
          {moving ? "Trade is flowing" : "No shipments last advance"}
        </span>
        <span style={{ flex: 1 }} />
        <span style={{ color: "#6a86a6", fontSize: 9 }}>year {diag.year}</span>
      </div>
      <div style={{ display: "flex", gap: 4 }}>
        {stat("shipped", String(diag.shipments_last), "#cfe0f4", "Shipments dispatched last advance")}
        {stat("by houses", `${housePct}%`, housePct > 0 ? "#e0c060" : "#7a90a8", `${diag.by_house} financed by a house, ${diag.by_guild} by local guilds`)}
        {stat("lost", String(diag.lost_last), diag.lost_last > 0 ? "#e0a09a" : "#7a90a8", "Voyages lost to storm/ambush last advance")}
        {stat("in transit", String(diag.in_transit), "#9ab0c8", "Shipments currently in flight")}
      </div>
      <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
        {stat("controls", String(diag.controlled_settlements), diag.controlled_settlements > 0 ? "#9fe0b8" : "#d06a5f", "Settlements a house controls (>=50% of trade throughput)")}
        {stat("ships", String(fleet), "#9ab0c8", `${diag.fleet_sea} sea · ${diag.fleet_river} river · ${diag.fleet_caravan} caravan`)}
        {stat("houses", `${diag.houses_active}`, "#cfe0f4", `${diag.houses_active} active · ${diag.houses_defunct} ruined`)}
        {stat("wealth", diag.total_house_wealth >= 100 ? `${Math.round(diag.total_house_wealth)}` : diag.total_house_wealth.toFixed(1), "#e0c060", "Combined wealth of all active houses")}
      </div>
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 396, maxHeight: "80vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #24364e", borderRadius: 10,
  boxShadow: "0 12px 36px rgba(0,0,0,0.55)", zIndex: 40,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "10px 12px", borderBottom: "1px solid #1a2a3e",
  color: "#e8dcc0", fontWeight: 700, fontSize: 13, letterSpacing: 0.2,
};
/** One house's card: a raised tile (not just a bottom-rule row) so a dense list of
 *  36 families still reads as distinct entries rather than one long scroll of text.
 *  A left accent bar in the house's own colour carries identity even before the
 *  reader reaches the coat of arms — the same trick a kanban/ticket card uses. */
const card: React.CSSProperties = {
  display: "flex", gap: 10, alignItems: "flex-start", padding: "9px 10px 9px 12px",
  margin: "0 0 6px", borderRadius: 7, background: "#101a26",
  border: "1px solid #1c2c3f", position: "relative", cursor: "default",
  transition: "background 0.12s, border-color 0.12s",
};
/** A small rounded label — the unit every "fact" (fleet count, office count, a
 *  monopoly share) renders as, instead of a bare run of icon+text on its own line.
 *  Consistent chip styling is what turns six stacked sentences into one scannable row. */
const chip: React.CSSProperties = {
  display: "inline-flex", alignItems: "center", gap: 3, fontSize: 9.5,
  padding: "1.5px 6px", borderRadius: 9, background: "#0a1420",
  border: "1px solid #1e3048", color: "#9ab0c8", lineHeight: 1.5, whiteSpace: "nowrap",
};
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "14px 6px", textAlign: "center" };
const diagBar: React.CSSProperties = {
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e", background: "#0a1119",
};
const diagCell: React.CSSProperties = {
  flex: 1, textAlign: "center", padding: "4px 2px", borderRadius: 5,
  background: "#101c28", border: "1px solid #16222e",
};
const gaugeCell: React.CSSProperties = {
  padding: "4px 6px", borderRadius: 5, background: "#101c28", border: "1px solid #16222e",
  minWidth: 0,
};

/** 🧭 Trade lanes by good — the focused house's web on the map is red and whole
 *  by default; picking a good here narrows the map to ONLY the lanes that carry
 *  it (in the good's own colour) and lists where it is bought and sold. */
function HouseLanesWindow({ houseIdx, houseName }: { houseIdx: number; houseName: string }) {
  const [atlas, setAtlas] = useState<HouseAtlas | null>(null);
  const pick = useUIStore((s) => s.houseLaneGood);
  const setPick = useUIStore((s) => s.setHouseLaneGood);
  const [min, setMin] = useState(false);
  const year = useCampaignStore((s) => s.diagnostics?.year);
  useEffect(() => {
    let alive = true;
    campaignHouseAtlas(houseIdx).then((a) => { if (alive) setAtlas(a); }).catch(() => { if (alive) setAtlas(null); });
    return () => { alive = false; };
  }, [houseIdx, year]);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.houses);
  const goods = [...(atlas?.goods ?? [])].sort((a, b) => b.volume - a.volume);
  const hubName = (hub: number) => atlas?.partners.find((p) => p.hub === hub)?.name ?? `#${hub}`;
  const lanesFor = (g: AtlasGoodBook) => (atlas?.partners ?? []).filter((p) =>
    p.goods.includes(g.good) || g.bought_at.includes(p.hub) || g.sold_at.includes(p.hub));
  const book = pick != null ? goods.find((g) => g.good === pick) : undefined;
  const fmtV = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));
  return (
    <div data-draggable onPointerDown={onPointerDown}
      style={{ ...rootStyle, position: "fixed", right: 16, top: 90, width: 270, maxHeight: "60vh", zIndex: 60,
        display: "flex", flexDirection: "column", border: "1px solid #3a2a22", borderRadius: 8,
        boxShadow: "0 8px 24px rgba(0,0,0,0.5)", fontSize: 11, color: "#d8e2ee" }}>
      <div style={{ ...header, cursor: "move", fontSize: 11.5 }}>
        <span>🧭 Trade lanes by good</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={() => setMin((v) => !v)}>{min ? "▸" : "▾"}</span>
      </div>
      {!min && (
        <div data-no-drag style={{ overflowY: "auto", padding: "6px 8px" }}>
          <div style={{ color: "#6f8aa6", fontSize: 10, marginBottom: 5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {houseName}
          </div>
          <div onClick={() => setPick(null)}
            style={{ display: "flex", alignItems: "center", gap: 6, padding: "4px 6px", borderRadius: 5, cursor: "pointer",
              background: pick == null ? "#3a1616" : "transparent", border: `1px solid ${pick == null ? "#eb4646" : "#2a1c1c"}` }}>
            <span style={{ width: 16, height: 3, background: "rgba(235,70,70,0.95)", borderRadius: 2 }} />
            <span style={{ flex: 1, fontWeight: 600 }}>All trade lanes</span>
            <span style={{ color: "#8aa0c0" }}>{atlas?.partners.length ?? 0} cities</span>
          </div>
          {atlas == null && <div style={{ color: "#6a7e96", padding: 6 }}>Loading…</div>}
          {atlas && goods.length === 0 && <div style={{ color: "#6a7e96", padding: 6 }}>No goods recorded for this house yet.</div>}
          {goods.map((g) => {
            const on = pick === g.good;
            const n = lanesFor(g).length;
            return (
              <div key={g.good} onClick={() => setPick(on ? null : g.good)}
                style={{ display: "flex", alignItems: "center", gap: 6, padding: "3px 6px", marginTop: 3, borderRadius: 5, cursor: "pointer",
                  background: on ? "#1c2636" : "transparent", border: `1px solid ${on ? "#c9a227" : "transparent"}` }}>
                <span style={{ width: 16, textAlign: "center" }}>{goodIcon(g.name)}</span>
                <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                  color: on ? "#f0d890" : "#d8e2ee" }}>{g.name}</span>
                <span style={{ color: "#8aa0c0", fontSize: 10 }}>{n} lane{n === 1 ? "" : "s"}</span>
                <span style={{ width: 40, textAlign: "right", color: "#e0c060", fontSize: 10 }}>{fmtV(g.volume)}</span>
              </div>
            );
          })}
          {book && (
            <div style={{ marginTop: 8, padding: "6px 7px", background: "#0d1622", border: "1px solid #24405e", borderRadius: 6 }}>
              <div style={{ fontWeight: 700, color: "#f0d890", marginBottom: 3 }}>{goodIcon(book.name)} {book.name}</div>
              <div style={{ color: "#8aa0c0", fontSize: 10 }}>
                profit {fmtV(book.profit)} · moved {fmtV(book.volume)}
              </div>
              {book.bought_at.length > 0 && (
                <div style={{ marginTop: 4, fontSize: 10 }}>
                  <span style={{ color: "#5fd0ff" }}>◀ bought at </span>{book.bought_at.map(hubName).join(", ")}
                </div>
              )}
              {book.sold_at.length > 0 && (
                <div style={{ marginTop: 2, fontSize: 10 }}>
                  <span style={{ color: "#ffce5f" }}>▶ sold at </span>{book.sold_at.map(hubName).join(", ")}
                </div>
              )}
              <div style={{ marginTop: 4, color: "#6a7e96", fontSize: 9.5 }}>
                Only this good&apos;s lanes are drawn on the map, in its colour.
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
