import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { useViewportStore } from "@state/viewportStore";
import { campaignGetJournal, campaignGetTradeBasins, campaignGetEraFrame } from "@bridge";
import type { CampaignHubBrief, JournalEntry, TradeBasin } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, SERIF } from "@ui/campaign/chronicleTheme";

/** Atlas 2.0 · the WORLD ATLAS — the "what happens in the world and why" screen.
 *
 *  · Overview — world graphs from the yearly ledger: population, trade volume,
 *    towns alive (with founding ▲ / abandonment † tick marks on the timeline).
 *  · Cities — the living census: every settlement's population, growth, trade
 *    throughput, wealth and a lifecycle status chip; sortable, click flies the
 *    map there.
 *  · Timeline — the year-grouped chronicle of foundings, abandonments, colonies,
 *    wars, crashes and plagues (filterable), newest year first. */
export function AtlasPanel() {
  const open = useUIStore((s) => s.showAtlas);
  const setOpen = useUIStore((s) => s.setShowAtlas);
  const toggleOverlay = useUIStore((s) => s.toggleOverlay);
  const heatOn = useUIStore((s) => s.overlayVisibility.tradeHeat ?? false);
  const basinsOverlayOn = useUIStore((s) => s.overlayVisibility.tradeBasins ?? false);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const worldEconomy = useCampaignStore((s) => s.worldEconomy);
  const setSearchPin = useViewportStore((s) => s.setSearchPin);
  const [tab, setTab] = useState<"overview" | "cities" | "regions" | "records" | "timeline">("overview");
  const eraFrame = useUIStore((s) => s.eraFrame);
  const setEraFrame = useUIStore((s) => s.setEraFrame);
  const [sortBy, setSortBy] = useState<"population" | "growth" | "trade" | "wealth">("population");
  const [journal, setJournal] = useState<JournalEntry[]>([]);
  const [basins, setBasins] = useState<TradeBasin[]>([]);
  const [kindFilter, setKindFilter] = useState<Record<string, boolean>>({
    lifecycle: true, war: true, finance: true, plague: true,
  });

  const active = snapshot?.active === true;
  const year = snapshot ? Math.floor(snapshot.clock.tick / 365) : 0;

  // The timeline reads the full journal (bounded below) — refetch as years pass.
  useEffect(() => {
    if (!open || !active || tab !== "timeline") return;
    let alive = true;
    campaignGetJournal(-1, -1).then((r) => { if (alive) setJournal(r); }).catch(() => {});
    return () => { alive = false; };
  }, [open, active, tab, year]);

  // The Regions tab reads the named trade basins — refetch as years pass.
  useEffect(() => {
    if (!open || !active || tab !== "regions") return;
    let alive = true;
    campaignGetTradeBasins().then((r) => { if (alive) setBasins(r); }).catch(() => {});
    return () => { alive = false; };
  }, [open, active, tab, year]);

  // Closing the Atlas always returns the map to the present.
  useEffect(() => {
    if (!open) setEraFrame(null);
  }, [open, setEraFrame]);

  // Era scrubber: slide to a past year → the MAP time-travels (markers + heat);
  // sliding back to the far right returns to the live world.
  const scrubTo = (y: number, maxYear: number) => {
    if (y >= maxYear) { setEraFrame(null); return; }
    campaignGetEraFrame(y).then((f) => setEraFrame(f)).catch(() => {});
  };

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.atlas);

  const hubs = useMemo(() => (snapshot?.hubs ?? []).filter((h) => !h.is_estate), [snapshot]);
  const series = worldEconomy?.world_series ?? [];

  const cities = useMemo(() => {
    const wealth = (h: CampaignHubBrief) => h.grain_wealth + h.trade_wealth;
    const key: Record<typeof sortBy, (h: CampaignHubBrief) => number> = {
      population: (h) => h.population,
      growth: (h) => h.growth,
      trade: (h) => h.trade_volume,
      wealth,
    };
    return [...hubs].sort((a, b) => key[sortBy](b) - key[sortBy](a));
  }, [hubs, sortBy]);

  const alive_n = hubs.filter((h) => !h.abandoned && h.population >= 1).length;
  const ruins = hubs.filter((h) => h.abandoned).length;
  const last = series.length > 0 ? series[series.length - 1] : null;

  const status = (h: CampaignHubBrief): { label: string; color: string; bg: string } => {
    const nowTick = snapshot?.clock.tick ?? 0;
    if (h.abandoned || h.population < 100)
      return { label: `† ${h.died_cause || "ruin"}`, color: "#9a9a9a", bg: "#1a1a1a" };
    if (h.founded_tick > 0 && nowTick - h.founded_tick < 15 * 365)
      return { label: "✦ new", color: "#ffd75e", bg: "#2a230e" };
    if (h.starving > 0.5) return { label: "starving", color: T.badInk, bg: "#2a1218" };
    if (h.growth > 0.003) return { label: "growing", color: T.goodInk, bg: "#122a1a" };
    if (h.growth < -0.003) return { label: "declining", color: "#e0b080", bg: "#2a2012" };
    return { label: "steady", color: T.inkDim, bg: "#131c28" };
  };

  // Timeline kinds → filter groups + colours.
  const groupOf = (kind: string): string | null => {
    if (["founding", "abandonment", "colony"].includes(kind)) return "lifecycle";
    if (["war", "levy", "blockade", "reparations"].includes(kind)) return "war";
    if (["crash", "bank", "bubble", "coin"].includes(kind)) return "finance";
    if (["contagion", "plague", "starvation"].includes(kind)) return "plague";
    return null;
  };
  const groupColor: Record<string, string> = {
    lifecycle: T.gold, war: "#e08080", finance: "#8ab0e0", plague: "#c07070",
  };
  const timeline = useMemo(() => {
    const rows = journal
      .map((e) => ({ e, g: groupOf(e.kind) }))
      .filter((r): r is { e: JournalEntry; g: string } => r.g !== null && kindFilter[r.g])
      .slice(-300)
      .reverse();
    const byYear = new Map<number, { e: JournalEntry; g: string }[]>();
    for (const r of rows) {
      const y = Math.floor(r.e.tick / 365);
      if (!byYear.has(y)) byYear.set(y, []);
      byYear.get(y)!.push(r);
    }
    return [...byYear.entries()].sort((a, b) => b[0] - a[0]);
  }, [journal, kindFilter]);

  // Every hook (incl. the `timeline` useMemo above) must run on EVERY render —
  // bail out only AFTER them, never before, or React throws "Rendered more hooks
  // than during the previous render" and the whole panel fails to mount.
  if (!open) return null;

  const tabBtn = (id: typeof tab, label: string) => (
    <button key={id} data-no-drag onClick={() => setTab(id)} style={{
      padding: "5px 12px", border: "none", borderBottom: `2px solid ${tab === id ? T.gold : "transparent"}`,
      background: "transparent", color: tab === id ? T.ink : T.inkDim,
      fontWeight: tab === id ? 700 : 400, fontSize: 12, cursor: "pointer",
    }}>{label}</button>
  );

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }} onPointerDown={onPointerDown}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span style={{ fontFamily: SERIF, color: T.gold, fontWeight: 700, fontSize: 14, letterSpacing: 0.4 }}>
          🗺 World Atlas
        </span>
        <span style={{ color: T.inkFaint, fontSize: 10, marginLeft: 8 }}>
          {active ? `Year ${year} · ${alive_n} towns alive${ruins > 0 ? ` · ${ruins} ruins` : ""}` : "no campaign"}
        </span>
        <span style={{ flex: 1 }} />
        <button data-no-drag onClick={() => toggleOverlay("tradeHeat")} title="Toggle the Trade Heat map overlay"
          style={{
            padding: "2px 8px", borderRadius: 5, fontSize: 10, cursor: "pointer", marginRight: 8,
            border: `1px solid ${heatOn ? "#d9a441" : T.line}`,
            background: heatOn ? "#2a230e" : "transparent", color: heatOn ? "#ffd75e" : T.inkDim,
          }}>🔥 Heat</button>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={() => setOpen(false)}>✕</span>
      </div>

      <div style={{ display: "flex", borderBottom: `1px solid ${T.lineSoft}`, padding: "0 6px" }}>
        {tabBtn("overview", "📈 Overview")}
        {tabBtn("cities", "🏙 Cities")}
        {tabBtn("regions", "🏞 Regions")}
        {tabBtn("records", "🏆 Records")}
        {tabBtn("timeline", "📜 Timeline")}
      </div>

      <div style={{ overflowY: "auto", padding: "8px 10px 12px", flex: 1 }}>
        {!active && <div style={hint}>Begin a campaign to open the Atlas — the world ledger fills in as years pass.</div>}
        {active && series.length < 2 && tab === "overview" && (
          <div style={hint}>Advance at least two years — the Atlas graphs read the yearly world ledger.</div>
        )}

        {active && tab === "overview" && series.length >= 2 && (
          <>
            <div style={{ display: "flex", gap: 6, marginBottom: 10 }}>
              <Tile label="Towns alive" value={String(last ? last[3] : alive_n)} />
              <Tile label="Founded" value={`+${last ? last[4] : 0}`} color="#ffd75e" />
              <Tile label="Abandoned" value={`−${last ? last[5] : 0}`} color="#c0a0a0" />
              <Tile label="Trade (yr)" value={fmtNum(last ? last[2] : 0)} color="#7fd0c0" />
            </div>
            {/* ── Era scrubber: drag into the past; the MAP follows. ── */}
            <div style={{
              display: "flex", alignItems: "center", gap: 8, marginBottom: 10,
              background: eraFrame ? "#241d0c" : T.card,
              border: `1px solid ${eraFrame ? T.goldDim : T.lineSoft}`,
              borderRadius: 6, padding: "6px 10px",
            }}>
              <span style={{ fontFamily: SERIF, color: eraFrame ? T.gold : T.inkDim, fontSize: 12, fontWeight: 700, whiteSpace: "nowrap" }}>
                {eraFrame ? `⏳ Year ${eraFrame.year}` : "⏳ Present"}
              </span>
              <input
                type="range"
                min={series[0][0]}
                max={year}
                step={1}
                value={eraFrame ? eraFrame.year : year}
                onChange={(e) => scrubTo(Number(e.target.value), year)}
                style={{ flex: 1, accentColor: "#d8b24a" }}
                title="Drag into the past — the map's markers and heat time-travel"
              />
              {eraFrame && (
                <button onClick={() => setEraFrame(null)} style={{
                  padding: "2px 9px", borderRadius: 5, fontSize: 10, cursor: "pointer",
                  border: `1px solid ${T.goldDim}`, background: "transparent", color: T.gold,
                }}>return to present</button>
              )}
            </div>
            <Chart title="World population" rows={series.map((r) => [r[0], r[1]])} color="#6ab0e8" fmt={fmtNum} />
            <Chart title="Trade volume (grain-eq / year)" rows={series.map((r) => [r[0], r[2]])} color="#4fd0c0" fmt={fmtNum} />
            <Chart title="Towns alive — ▲ founded · † abandoned" rows={series.map((r) => [r[0], r[3]])}
              color="#d8b24a" fmt={(v) => String(Math.round(v))} markers={lifecycleMarkers(series)} />
          </>
        )}

        {active && tab === "cities" && (
          <>
            <div style={{ display: "flex", gap: 4, marginBottom: 6, alignItems: "center" }}>
              <span style={{ color: T.inkDim, fontSize: 10 }}>sort by</span>
              {(["population", "growth", "trade", "wealth"] as const).map((k) => (
                <button key={k} onClick={() => setSortBy(k)} style={{
                  padding: "2px 8px", borderRadius: 5, fontSize: 10, cursor: "pointer", textTransform: "capitalize",
                  border: `1px solid ${sortBy === k ? T.accent : T.line}`,
                  background: sortBy === k ? T.accentSoft : "transparent",
                  color: sortBy === k ? T.ink : T.inkDim,
                }}>{k}</button>
              ))}
            </div>
            <div style={{ display: "grid", gridTemplateColumns: CENSUS_COLS, gap: 2, fontSize: 10, color: T.inkDim, padding: "2px 4px", textTransform: "uppercase", letterSpacing: 0.4 }}>
              <span>City</span><span>History</span><span style={num}>Pop</span><span style={num}>Δ%/mo</span>
              <span style={num}>Trade/yr</span><span style={num}>Wealth</span><span>Status</span>
            </div>
            {cities.map((h) => {
              const st = status(h);
              return (
                <div key={h.id} onClick={() => setSearchPin(h.x, h.y)}
                  title="Click to pin this city on the map"
                  style={{
                    display: "grid", gridTemplateColumns: CENSUS_COLS, gap: 2, alignItems: "center",
                    fontSize: 11, padding: "3px 4px", cursor: "pointer", borderRadius: 4,
                    borderBottom: `1px solid ${T.lineSoft}`,
                    opacity: h.abandoned ? 0.55 : 1,
                  }}>
                  <span style={{ color: T.ink, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {h.abandoned ? "† " : ""}{h.name}{h.colony_kind === 1 ? " ⛶" : ""}
                  </span>
                  <Spark values={h.pop_spark} dead={h.abandoned} />
                  <span style={{ ...num, color: T.inkMid }}>{fmtNum(h.population)}</span>
                  <span style={{ ...num, color: h.growth > 0 ? T.goodInk : h.growth < 0 ? T.badInk : T.inkDim }}>
                    {(h.growth * 100).toFixed(1)}
                  </span>
                  <span style={{ ...num, color: "#7fd0c0" }}>{fmtNum(h.trade_volume)}</span>
                  <span style={{ ...num, color: "#d8b070" }}>{fmtNum(h.grain_wealth + h.trade_wealth)}</span>
                  <span><span style={{ background: st.bg, color: st.color, borderRadius: 4, padding: "1px 6px", fontSize: 9.5, fontWeight: 600 }}>{st.label}</span></span>
                </div>
              );
            })}
          </>
        )}

        {active && tab === "regions" && (
          <>
            <div style={{ display: "flex", alignItems: "center", marginBottom: 8 }}>
              <span style={{ color: T.inkDim, fontSize: 11 }}>
                Named trade basins — clusters bound by their strongest trade ties.
              </span>
              <span style={{ flex: 1 }} />
              <button onClick={() => toggleOverlay("tradeBasins")} style={{
                padding: "2px 9px", borderRadius: 5, fontSize: 10, cursor: "pointer",
                border: `1px solid ${basinsOverlayOn ? T.gold : T.line}`,
                background: basinsOverlayOn ? "#241d0c" : "transparent",
                color: basinsOverlayOn ? T.gold : T.inkDim,
              }}>🏞 show on map</button>
            </div>
            {basins.length === 0 && <div style={hint}>No basins yet — trade needs a full year on the ledger.</div>}
            {basins.map((b, i) => (
              <div key={b.name + i} onClick={() => setSearchPin(b.cx, b.cy)}
                title="Click to pin this basin's heart on the map"
                style={{
                  display: "flex", alignItems: "baseline", gap: 8, cursor: "pointer",
                  background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 6,
                  borderLeft: `3px solid ${BASIN_UI_COLORS[i % BASIN_UI_COLORS.length]}`,
                  padding: "6px 9px", marginBottom: 5,
                }}>
                <span style={{ fontFamily: SERIF, color: BASIN_UI_COLORS[i % BASIN_UI_COLORS.length], fontWeight: 700, fontSize: 13 }}>
                  {b.name}
                </span>
                <span style={{ color: T.inkDim, fontSize: 10 }}>{b.hub_ids.length} towns</span>
                {b.top_goods.length > 0 && (
                  <span style={{ color: "#c9b96a", fontSize: 10 }}>{b.top_goods.join(" · ")}</span>
                )}
                <span style={{ flex: 1 }} />
                <span style={{ color: T.inkDim, fontSize: 10 }}>busiest</span>
                <span style={{ color: T.inkMid, fontSize: 11 }}>{b.top_city}</span>
                <span style={{ color: "#7fd0c0", fontSize: 12, fontWeight: 700 }}>{fmtNum(b.volume)}</span>
              </div>
            ))}
          </>
        )}

        {active && tab === "records" && (
          <>
            <div style={{ color: T.inkDim, fontSize: 11, marginBottom: 8 }}>
              The Hall of Records — all-time bests of this world, set the year they happened.
            </div>
            {(() => {
              const rec = worldEconomy?.records;
              if (!rec) return <div style={hint}>Advance time — records are written at each New Year.</div>;
              const rows: { icon: string; label: string; entry: [number, string, number]; fmt: (v: number) => string }[] = [
                { icon: "🏙", label: "Largest city ever", entry: rec.largest_city, fmt: fmtNum },
                { icon: "⚜️", label: "Richest house ever", entry: rec.richest_house, fmt: fmtNum },
                { icon: "🚢", label: "Greatest trade year", entry: rec.biggest_trade_year, fmt: fmtNum },
                { icon: "🏘", label: "Most towns alive", entry: rec.most_towns, fmt: (v) => String(Math.round(v)) },
                { icon: "⚭", label: "Longest dynasty", entry: rec.longest_dynasty, fmt: (v) => `${Math.round(v)} generations` },
                { icon: "☠", label: "Deadliest plague strike", entry: rec.deadliest_plague, fmt: (v) => `${fmtNum(v)} dead` },
                { icon: "📉", label: "Worst crash", entry: rec.worst_crash, fmt: (v) => `${Math.round(v)} cities hit` },
              ];
              const set = rows.filter((r) => r.entry && (r.entry[0] > 0 || r.entry[1] !== ""));
              if (set.length === 0) return <div style={hint}>No records yet — advance at least a year.</div>;
              return set.map((r) => (
                <div key={r.label} style={{
                  display: "flex", alignItems: "baseline", gap: 8,
                  background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 6,
                  padding: "7px 10px", marginBottom: 5,
                }}>
                  <span style={{ fontSize: 14 }}>{r.icon}</span>
                  <span style={{ color: T.inkDim, fontSize: 11, minWidth: 150 }}>{r.label}</span>
                  <span style={{ fontFamily: SERIF, color: T.parchment, fontSize: 13, fontWeight: 700 }}>
                    {r.entry[1] || "—"}
                  </span>
                  <span style={{ flex: 1 }} />
                  <span style={{ color: T.gold, fontSize: 12, fontWeight: 700 }}>{r.fmt(r.entry[0])}</span>
                  <span style={{ color: T.inkFaint, fontSize: 10 }}>Y{r.entry[2]}</span>
                </div>
              ));
            })()}
          </>
        )}

        {active && tab === "timeline" && (
          <>
            <div style={{ display: "flex", gap: 4, marginBottom: 8, flexWrap: "wrap" }}>
              {(["lifecycle", "war", "finance", "plague"] as const).map((g) => (
                <button key={g} onClick={() => setKindFilter((f) => ({ ...f, [g]: !f[g] }))} style={{
                  padding: "2px 9px", borderRadius: 10, fontSize: 10, cursor: "pointer", textTransform: "capitalize",
                  border: `1px solid ${kindFilter[g] ? groupColor[g] : T.line}`,
                  background: kindFilter[g] ? "rgba(255,255,255,0.04)" : "transparent",
                  color: kindFilter[g] ? groupColor[g] : T.inkFaint,
                }}>{g === "lifecycle" ? "🏙 lifecycle" : g === "war" ? "⚔ war" : g === "finance" ? "🪙 finance" : "☠ plague"}</button>
              ))}
            </div>
            {timeline.length === 0 && <div style={hint}>Nothing chronicled yet — advance time (or widen the filters).</div>}
            {timeline.map(([y, rows]) => (
              <div key={y} style={{ marginBottom: 8 }}>
                <div style={{ fontFamily: SERIF, color: T.gold, fontSize: 12, fontWeight: 700, borderBottom: `1px solid ${T.lineGold}`, marginBottom: 3 }}>
                  Year {y}
                </div>
                {rows.map((r, i) => (
                  <div key={i} style={{ fontSize: 11, color: groupColor[r.g], marginBottom: 2, lineHeight: 1.4 }}>
                    {r.e.text}
                  </div>
                ))}
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}

/** Founding/abandonment tick marks derived from the cumulative counters. */
function lifecycleMarkers(series: [number, number, number, number, number, number][]) {
  const marks: { x: number; label: string; color: string }[] = [];
  for (let i = 1; i < series.length; i++) {
    if (series[i][4] > series[i - 1][4]) marks.push({ x: series[i][0], label: "▲", color: "#ffd75e" });
    if (series[i][5] > series[i - 1][5]) marks.push({ x: series[i][0], label: "†", color: "#c0a0a0" });
  }
  return marks;
}

/** Tiny inline population sparkline for one census row (≤30 points). */
function Spark({ values, dead }: { values: number[]; dead: boolean }) {
  if (!values || values.length < 2) return <span style={{ color: T.inkFaint, fontSize: 9 }}>—</span>;
  const w = 56, h = 14;
  const lo = Math.min(...values), hi = Math.max(...values);
  const span = hi - lo || 1;
  const pts = values.map((v, i) =>
    `${((i / (values.length - 1)) * w).toFixed(1)},${(h - 2 - ((v - lo) / span) * (h - 4)).toFixed(1)}`
  ).join(" ");
  return (
    <svg width={w} height={h} style={{ display: "block" }}>
      <polyline points={pts} fill="none"
        stroke={dead ? "#6a6a6a" : "#6ab0e8"} strokeWidth={1} />
    </svg>
  );
}

/** Census grid template (City · History spark · Pop · Δ · Trade · Wealth · Status). */
const CENSUS_COLS = "1.45fr 62px 0.7fr 0.55fr 0.7fr 0.7fr 1fr";

/** Mirror of the map overlay's basin palette (OverlayManager BASIN_COLORS). */
const BASIN_UI_COLORS = [
  "#4fd0c0", "#d8b24a", "#c08cff", "#7fd08a", "#e08a6a", "#6aa9e8",
  "#e0a0d0", "#c9c96a", "#8ad0e0", "#d88a8a", "#a9c96a", "#c0a06a",
];

function Tile({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div style={{ flex: 1, background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 6, padding: "5px 8px" }}>
      <div style={{ color: T.inkDim, fontSize: 8.5, fontWeight: 600, textTransform: "uppercase", letterSpacing: 0.5 }}>{label}</div>
      <div style={{ color: color ?? T.ink, fontSize: 14, fontWeight: 700 }}>{value}</div>
    </div>
  );
}

/** A titled SVG line chart over (x = year, y = value) rows, with min/max labels,
 *  first/last year ticks and optional event markers on the x axis. */
function Chart({ title, rows, color, fmt, markers }: {
  title: string; rows: [number, number][]; color: string;
  fmt: (v: number) => string;
  markers?: { x: number; label: string; color: string }[];
}) {
  const w = 540, h = 96, padB = markers ? 14 : 4;
  const xs = rows.map((r) => r[0]), ys = rows.map((r) => r[1]);
  const x0 = Math.min(...xs), x1 = Math.max(...xs);
  const lo = Math.min(...ys), hi = Math.max(...ys);
  const spanX = x1 - x0 || 1, spanY = hi - lo || 1;
  const px = (x: number) => ((x - x0) / spanX) * (w - 8) + 4;
  const py = (y: number) => h - padB - ((y - lo) / spanY) * (h - padB - 6) - 2;
  const pts = rows.map((r) => `${px(r[0]).toFixed(1)},${py(r[1]).toFixed(1)}`).join(" ");
  return (
    <div style={{ background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 6, padding: "6px 8px", marginBottom: 8 }}>
      <div style={{ display: "flex", alignItems: "baseline", marginBottom: 2 }}>
        <span style={{ color: T.inkDim, fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: 0.5 }}>{title}</span>
        <span style={{ flex: 1 }} />
        <span style={{ color, fontSize: 11, fontWeight: 700 }}>{fmt(ys[ys.length - 1])}</span>
      </div>
      <svg width="100%" height={h} viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{ display: "block", background: "#0b1622", borderRadius: 4 }}>
        <polyline points={pts} fill="none" stroke={color} strokeWidth={1.6} />
        {markers?.map((m, i) => (
          <text key={i} x={px(m.x)} y={h - 3} textAnchor="middle" fontSize={9} fill={m.color}>{m.label}</text>
        ))}
      </svg>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: T.inkFaint, marginTop: 1 }}>
        <span>Y{x0} · min {fmt(lo)}</span>
        <span>max {fmt(hi)} · Y{x1}</span>
      </div>
    </div>
  );
}

function fmtNum(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + "M";
  if (n >= 10_000) return (n / 1_000).toFixed(0) + "k";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "k";
  return Math.round(n).toLocaleString();
}

const num: React.CSSProperties = { textAlign: "right" };
const panel: React.CSSProperties = {
  position: "absolute", top: 56, left: 270, width: 580, maxHeight: "80vh", zIndex: 120,
  display: "flex", flexDirection: "column",
  border: `1px solid ${T.line}`, borderRadius: 8,
  boxShadow: "0 12px 34px rgba(0,0,0,0.55)", color: T.ink, fontSize: 12,
};
const header: React.CSSProperties = {
  display: "flex", alignItems: "center", padding: "7px 10px",
  borderBottom: `1px solid ${T.line}`,
};
const hint: React.CSSProperties = { color: T.inkDim, fontSize: 11, padding: 8 };
