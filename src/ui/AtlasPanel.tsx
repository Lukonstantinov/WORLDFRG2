import { useEffect, useMemo, useRef, useState, useCallback } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { useViewportStore } from "../state/viewportStore";
import { useWorldStore } from "../state/worldStore";
import {
  campaignGetJournal, campaignGetTradeBasins, campaignGetEraFrame, campaignGetInequality,
  campaignGetGeoPoints,
} from "../bridge/tauri";
import type {
  CampaignHubBrief, JournalEntry, TradeBasin, InequalitySnapshot, GeoHubPoint,
} from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";
import { T, SERIF } from "./chronicleTheme";

/** Atlas 3.0 · the WORLD ATLAS — a resizable analytics dashboard that gathers,
 *  in ONE place, everything the other panels show piecemeal (economy, finance,
 *  war, inequality, city census, regions) plus graphs and *correlations* you
 *  can't see anywhere else.
 *
 *  · Pulse       — headline KPIs + the world time-series grid + era scrubber.
 *  · Correlations— unique cross-variable scatter plots with a Pearson r and a
 *                  fitted trend line (city scaling / geography / prosperity).
 *  · Economy     — price index, shortage by need tier, merchant-class mix,
 *                  wealth inequality (Gini / top-10% share) over time.
 *  · Turmoil     — wars, crashes, bank shocks and plagues quantified per year
 *                  from the world journal (not just listed — charted).
 *  · Cities      — the living census: sortable, click flies the map there.
 *  · Regions     — named trade basins.
 *  · Records     — the Hall of Records.
 *  · Timeline    — the year-grouped chronicle (filterable). */

type Tab =
  | "pulse" | "correlations" | "economy" | "turmoil"
  | "cities" | "regions" | "records" | "timeline";

export function AtlasPanel() {
  const open = useUIStore((s) => s.showAtlas);
  const setOpen = useUIStore((s) => s.setShowAtlas);
  const toggleOverlay = useUIStore((s) => s.toggleOverlay);
  const heatOn = useUIStore((s) => s.overlayVisibility.tradeHeat ?? false);
  const basinsOverlayOn = useUIStore((s) => s.overlayVisibility.tradeBasins ?? false);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const worldEconomy = useCampaignStore((s) => s.worldEconomy);
  const meta = useWorldStore((s) => s.meta);
  const setSearchPin = useViewportStore((s) => s.setSearchPin);
  const [tab, setTab] = useState<Tab>("pulse");
  const eraFrame = useUIStore((s) => s.eraFrame);
  const setEraFrame = useUIStore((s) => s.setEraFrame);
  const [sortBy, setSortBy] = useState<"population" | "growth" | "trade" | "wealth">("population");
  const [journal, setJournal] = useState<JournalEntry[]>([]);
  const [basins, setBasins] = useState<TradeBasin[]>([]);
  const [ineq, setIneq] = useState<InequalitySnapshot | null>(null);
  const [geo, setGeo] = useState<GeoHubPoint[]>([]);
  const [kindFilter, setKindFilter] = useState<Record<string, boolean>>({
    lifecycle: true, war: true, finance: true, plague: true,
  });

  const active = snapshot?.active === true;
  const year = snapshot ? Math.floor(snapshot.clock.tick / 365) : 0;

  // The journal feeds BOTH the Timeline and the Turmoil charts — fetch it once
  // whenever either is on screen (bounded read), refetch as years pass.
  useEffect(() => {
    if (!open || !active || (tab !== "timeline" && tab !== "turmoil")) return;
    let alive = true;
    campaignGetJournal(-1, -1).then((r) => { if (alive) setJournal(r); }).catch(() => {});
    return () => { alive = false; };
  }, [open, active, tab, year]);

  // Inequality trend powers the Economy tab + a couple of correlations.
  useEffect(() => {
    if (!open || !active || (tab !== "economy" && tab !== "correlations" && tab !== "pulse")) return;
    let alive = true;
    campaignGetInequality().then((r) => { if (alive) setIneq(r); }).catch(() => {});
    return () => { alive = false; };
  }, [open, active, tab, year]);

  // The Correlations tab joins each city to its cell's climate (backend read).
  useEffect(() => {
    if (!open || !active || tab !== "correlations") return;
    let alive = true;
    campaignGetGeoPoints().then((r) => { if (alive) setGeo(r); }).catch(() => {});
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
  const { size, onResizeDown } = useResizable(920, 660);

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

  // ── Correlation data: one point per LIVING city (pop/wealth/trade + geography) ──
  const gridH = meta?.grid_height ?? 0;
  const eq = meta?.equator_offset ?? 0.5;
  const latOf = useCallback((y: number) => {
    if (gridH <= 0) return 0;
    const denom = Math.max(eq, 1 - eq) || 1;
    return Math.min(1, Math.abs(y / gridH - eq) / denom);
  }, [gridH, eq]);
  const cityPoints = useMemo(() =>
    hubs.filter((h) => !h.abandoned && h.population >= 1).map((h) => ({
      name: h.name,
      x: h.x,
      y: h.y,
      pop: Math.max(1, h.population),
      wealth: Math.max(0.01, h.grain_wealth + h.trade_wealth),
      trade: Math.max(0.01, h.trade_volume),
      growth: h.growth,
      mood: h.mood,
      lat: latOf(h.y),
    })), [hubs, latOf]);

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

  // Timeline / Turmoil kinds → filter groups + colours.
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

  // ── Turmoil: quantify shock EVENTS per year from the journal ──
  const turmoil = useMemo(() => {
    const catOf = (kind: string): keyof typeof TURMOIL_CATS | null => {
      if (["war", "levy", "blockade", "reparations"].includes(kind)) return "war";
      if (["crash", "bubble"].includes(kind)) return "crash";
      if (["bank"].includes(kind)) return "bank";
      if (["contagion", "plague", "starvation"].includes(kind)) return "plague";
      return null;
    };
    type TRow = { year: number; war: number; crash: number; bank: number; plague: number };
    const byYear = new Map<number, TRow>();
    for (const e of journal) {
      const c = catOf(e.kind);
      if (!c) continue;
      const y = Math.floor(e.tick / 365);
      if (!byYear.has(y)) byYear.set(y, { year: y, war: 0, crash: 0, bank: 0, plague: 0 });
      byYear.get(y)![c] += 1;
    }
    const rows = [...byYear.values()].sort((a, b) => a.year - b.year);
    const totals: Record<string, number> = { war: 0, crash: 0, bank: 0, plague: 0 };
    for (const r of rows) for (const k of ["war", "crash", "bank", "plague"] as const) totals[k] += r[k];
    return { rows, totals };
  }, [journal]);

  // Every hook (incl. the useMemos above) must run on EVERY render — bail out
  // only AFTER them, never before, or React throws "Rendered more hooks…".
  if (!open) return null;

  const tabBtn = (id: Tab, label: string) => (
    <button key={id} data-no-drag onClick={() => setTab(id)} style={{
      padding: "5px 10px", border: "none", borderBottom: `2px solid ${tab === id ? T.gold : "transparent"}`,
      background: "transparent", color: tab === id ? T.ink : T.inkDim,
      fontWeight: tab === id ? 700 : 400, fontSize: 12, cursor: "pointer", whiteSpace: "nowrap",
    }}>{label}</button>
  );

  return (
    <div data-draggable style={{ ...panel, width: size.w, height: size.h, ...rootStyle }}>
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

      <div style={{ display: "flex", borderBottom: `1px solid ${T.lineSoft}`, padding: "0 6px", overflowX: "auto" }}>
        {tabBtn("pulse", "📈 Pulse")}
        {tabBtn("correlations", "🔗 Correlations")}
        {tabBtn("economy", "💰 Economy")}
        {tabBtn("turmoil", "⚔ Turmoil")}
        {tabBtn("cities", "🏙 Cities")}
        {tabBtn("regions", "🏞 Regions")}
        {tabBtn("records", "🏆 Records")}
        {tabBtn("timeline", "📜 Timeline")}
      </div>

      <div style={{ overflowY: "auto", padding: "8px 10px 12px", flex: 1 }}>
        {!active && <div style={hint}>Begin a campaign to open the Atlas — the world ledger fills in as years pass.</div>}

        {active && tab === "pulse" && (
          <PulseTab {...{ series, last, alive_n, ineq, worldEconomy, eraFrame, setEraFrame, scrubTo, year }} />
        )}

        {active && tab === "correlations" && (
          <CorrelationsTab points={cityPoints} geo={geo} onPick={(x, y) => setSearchPin(x, y)} />
        )}

        {active && tab === "economy" && (
          <EconomyTab worldEconomy={worldEconomy} ineq={ineq} />
        )}

        {active && tab === "turmoil" && (
          <TurmoilTab turmoil={turmoil} records={worldEconomy?.records} />
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
          <RecordsTab records={worldEconomy?.records} />
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

      {/* Resize grip (bottom-right). */}
      <div data-no-drag onPointerDown={onResizeDown} title="Drag to resize" style={{
        position: "absolute", right: 0, bottom: 0, width: 16, height: 16, cursor: "nwse-resize",
        background: `linear-gradient(135deg, transparent 50%, ${T.line} 50%, ${T.line} 62%, transparent 62%, transparent 74%, ${T.line} 74%, ${T.line} 86%, transparent 86%)`,
        borderBottomRightRadius: 8,
      }} />
    </div>
  );
}

/* ─────────────────────────── Pulse tab ─────────────────────────── */

function PulseTab({ series, last, alive_n, ineq, worldEconomy, eraFrame, setEraFrame, scrubTo, year }: {
  series: [number, number, number, number, number, number][];
  last: number[] | null; alive_n: number; ineq: InequalitySnapshot | null;
  worldEconomy: import("../types").WorldEconomy | null;
  eraFrame: import("../types").EraFrame | null;
  setEraFrame: (f: import("../types").EraFrame | null) => void;
  scrubTo: (y: number, maxYear: number) => void; year: number;
}) {
  if (series.length < 2) {
    return <div style={hint}>Advance at least two years — the Atlas graphs read the yearly world ledger.</div>;
  }
  const indexYearly = toYearly(worldEconomy?.index_series ?? [], 1);
  return (
    <>
      <div style={{ display: "flex", gap: 6, marginBottom: 10, flexWrap: "wrap" }}>
        <Tile label="Towns alive" value={String(last ? last[3] : alive_n)} />
        <Tile label="Founded" value={`+${last ? last[4] : 0}`} color="#ffd75e" />
        <Tile label="Abandoned" value={`−${last ? last[5] : 0}`} color="#c0a0a0" />
        <Tile label="Trade (yr)" value={fmtNum(last ? last[2] : 0)} color="#7fd0c0" />
        <Tile label="Population" value={fmtNum(last ? last[1] : 0)} color="#6ab0e8" />
        {ineq && <Tile label="Wealth Gini" value={ineq.gini_now.toFixed(2)} color="#d88a8a" />}
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
        <input type="range" min={series[0][0]} max={year} step={1}
          value={eraFrame ? eraFrame.year : year}
          onChange={(e) => scrubTo(Number(e.target.value), year)}
          style={{ flex: 1, accentColor: "#d8b24a" }}
          title="Drag into the past — the map's markers and heat time-travel" />
        {eraFrame && (
          <button onClick={() => setEraFrame(null)} style={{
            padding: "2px 9px", borderRadius: 5, fontSize: 10, cursor: "pointer",
            border: `1px solid ${T.goldDim}`, background: "transparent", color: T.gold,
          }}>return to present</button>
        )}
      </div>
      <Grid2>
        <Chart title="World population" rows={series.map((r) => [r[0], r[1]])} color="#6ab0e8" fmt={fmtNum} />
        <Chart title="Trade volume (grain-eq / year)" rows={series.map((r) => [r[0], r[2]])} color="#4fd0c0" fmt={fmtNum} />
        <Chart title="Towns alive — ▲ founded · † abandoned" rows={series.map((r) => [r[0], r[3]])}
          color="#d8b24a" fmt={(v) => String(Math.round(v))} markers={lifecycleMarkers(series)} />
        {indexYearly.length >= 2 && (
          <Chart title="World price index" rows={indexYearly} color="#e0a86a" fmt={(v) => v.toFixed(2)} />
        )}
        {ineq && ineq.series.length >= 2 && (
          <Chart title="Wealth inequality (Gini)" rows={ineq.series.map((p) => [p.year, p.gini])}
            color="#d88a8a" fmt={(v) => v.toFixed(2)} />
        )}
      </Grid2>
    </>
  );
}

/* ─────────────────────── Correlations tab ─────────────────────── */

type CityPoint = { name: string; x: number; y: number; pop: number; wealth: number; trade: number; growth: number; mood: number; lat: number };

function CorrelationsTab({ points, geo, onPick }: {
  points: CityPoint[]; geo: GeoHubPoint[]; onPick: (x: number, y: number) => void;
}) {
  if (points.length < 4) {
    return <div style={hint}>Need at least four living cities on the ledger to read correlations.</div>;
  }
  // Zipf: rank cities by population, plot log rank vs log population.
  const zipf = [...points].sort((a, b) => b.pop - a.pop)
    .map((p, i) => ({ x: i + 1, y: p.pop, label: p.name, mx: p.x, my: p.y }));
  const geoOk = geo.length >= 4;

  // Unified per-city vector for the correlation matrix — geography joined to the
  // economic census by name (geo carries climate; the census carries mood/growth).
  const cityByName = new Map(points.map((p) => [p.name, p]));
  const matrixData: Record<string, number>[] = geoOk
    ? geo.map((g) => {
        const c = cityByName.get(g.name);
        return {
          pop: g.population, wealth: g.wealth, trade: g.trade,
          temp: g.temperature, precip: g.precipitation, elev: g.elevation,
          coast: g.distance_to_ocean, fert: g.fertility,
          growth: c ? c.growth : 0, mood: c ? c.mood : 0, lat: c ? c.lat : 0,
        };
      })
    : points.map((p) => ({ pop: p.pop, wealth: p.wealth, trade: p.trade, growth: p.growth, mood: p.mood, lat: p.lat }));
  const matrixVars: { key: string; label: string; log?: boolean }[] = geoOk
    ? [
        { key: "pop", label: "Pop", log: true }, { key: "wealth", label: "Wealth", log: true },
        { key: "trade", label: "Trade", log: true }, { key: "temp", label: "Temp" },
        { key: "precip", label: "Rain" }, { key: "elev", label: "Elev" },
        { key: "coast", label: "Coast" }, { key: "fert", label: "Fert" },
        { key: "lat", label: "Lat" }, { key: "mood", label: "Mood" }, { key: "growth", label: "Grow" },
      ]
    : [
        { key: "pop", label: "Pop", log: true }, { key: "wealth", label: "Wealth", log: true },
        { key: "trade", label: "Trade", log: true }, { key: "lat", label: "Lat" },
        { key: "mood", label: "Mood" }, { key: "growth", label: "Grow" },
      ];
  return (
    <>
      <div style={{ color: T.inkDim, fontSize: 11, marginBottom: 8, lineHeight: 1.5 }}>
        Each dot is one living city — <b style={{ color: T.inkMid }}>click a dot to pin it on the map</b>.
        {" "}<b style={{ color: T.inkMid }}>r</b> is the Pearson correlation (−1…+1); the gold line is the
        least-squares trend. These relationships live nowhere else in the app.
      </div>

      <div style={sectionHdrLocal}>Correlation matrix — every variable pair at a glance</div>
      <CorrMatrix vars={matrixVars} data={matrixData} />

      <div style={{ ...sectionHdrLocal, marginTop: 10 }}>City scaling</div>
      <Grid2>
        <Scatter title="City rank–size (Zipf)" pts={zipf} onPick={onPick}
          xLabel="rank (largest → smallest)" yLabel="population"
          logX logY color="#6ab0e8"
          note="A straight −1 slope = a healthy Zipf hierarchy; a flat top = no dominant primate city." />
        <Scatter title="Population ↔ Wealth" onPick={onPick}
          pts={points.map((p) => ({ x: p.pop, y: p.wealth, label: p.name, mx: p.x, my: p.y }))}
          xLabel="population" yLabel="wealth" logX logY color="#d8b070"
          note="Does size buy wealth? Cities well above the line punch above their weight." />
        <Scatter title="Population ↔ Trade throughput" onPick={onPick}
          pts={points.map((p) => ({ x: p.pop, y: p.trade, label: p.name, mx: p.x, my: p.y }))}
          xLabel="population" yLabel="trade / yr" logX logY color="#4fd0c0"
          note="Trade hubs sit above the line — busier than their size implies." />
        <Scatter title="Trade ↔ Wealth" onPick={onPick}
          pts={points.map((p) => ({ x: p.trade, y: p.wealth, label: p.name, mx: p.x, my: p.y }))}
          xLabel="trade / yr" yLabel="wealth" logX logY color="#c9b96a"
          note="Is commerce the road to riches, or does grain wealth dominate?" />
      </Grid2>

      <div style={{ ...sectionHdrLocal, marginTop: 10 }}>Geography ↔ economy</div>
      {!geoOk && <div style={hint}>Reading the climate under each city…</div>}
      {geoOk && (
        <Grid2>
          <Scatter title="Temperature ↔ Wealth" onPick={onPick}
            pts={geo.map((p) => ({ x: p.temperature, y: Math.max(0.01, p.wealth), label: p.name, mx: p.x, my: p.y }))}
            xLabel="mean temp °C" yLabel="wealth" logY color="#e08a6a"
            note="Do warm or cool cities prosper? Ties wealth to the temperature belts." />
          <Scatter title="Rainfall ↔ Wealth" onPick={onPick}
            pts={geo.map((p) => ({ x: p.precipitation, y: Math.max(0.01, p.wealth), label: p.name, mx: p.x, my: p.y }))}
            xLabel="precip mm/yr" yLabel="wealth" logY color="#6aa9e8"
            note="Wet breadbaskets vs arid trade entrepôts." />
          <Scatter title="Coast distance ↔ Trade" onPick={onPick}
            pts={geo.map((p) => ({ x: Math.max(0.001, p.distance_to_ocean), y: Math.max(0.01, p.trade), label: p.name, mx: p.x, my: p.y }))}
            xLabel="distance to ocean" yLabel="trade / yr" logY color="#7fd0c0"
            note="Expect a NEGATIVE r — ports out-trade the landlocked interior." />
          <Scatter title="Fertility ↔ Population" onPick={onPick}
            pts={geo.map((p) => ({ x: p.fertility, y: Math.max(1, p.population), label: p.name, mx: p.x, my: p.y }))}
            xLabel="soil fertility" yLabel="population" logY color="#7fd08a"
            note="Fertile ground should feed bigger cities." />
          <Scatter title="Elevation ↔ Population" onPick={onPick}
            pts={geo.map((p) => ({ x: p.elevation, y: Math.max(1, p.population), label: p.name, mx: p.x, my: p.y }))}
            xLabel="elevation (0–1)" yLabel="population" logY color="#c9b96a"
            note="Highland vs lowland settlement — usually a negative slope." />
          <Scatter title="Latitude ↔ Wealth" onPick={onPick}
            pts={points.map((p) => ({ x: p.lat, y: p.wealth, label: p.name, mx: p.x, my: p.y }))}
            xLabel="0 equator → 1 pole" yLabel="wealth" logY color="#c08cff"
            note="Do the tropics or the temperate belts hold the riches?" />
        </Grid2>
      )}

      <div style={{ ...sectionHdrLocal, marginTop: 10 }}>Society</div>
      <Grid2>
        <Scatter title="Growth ↔ Mood" onPick={onPick}
          pts={points.map((p) => ({ x: p.mood, y: p.growth * 100, label: p.name, mx: p.x, my: p.y }))}
          xLabel="civic mood" yLabel="growth %/mo" color="#e0a86a"
          note="Happy cities should be growing cities — outliers are worth a look." />
      </Grid2>
    </>
  );
}

/* ───────────────────────── Economy tab ───────────────────────── */

function EconomyTab({ worldEconomy, ineq }: {
  worldEconomy: import("../types").WorldEconomy | null; ineq: InequalitySnapshot | null;
}) {
  const idx = toYearly(worldEconomy?.index_series ?? [], 1);
  const lack = (worldEconomy?.lack_series ?? []);
  const merch = (worldEconomy?.merchant_series ?? []);
  const lackYearly = yearlyRows(lack); // [year, basic, comfort, luxury]
  const merchYearly = yearlyRows(merch); // [year, houses, local, guild]
  const hasAny = idx.length >= 2 || lackYearly.length >= 2 || merchYearly.length >= 2 || (ineq && ineq.series.length >= 2);
  if (!hasAny) return <div style={hint}>Advance a few years — the economy charts read the monthly world ledger.</div>;
  return (
    <>
      <div style={{ display: "flex", gap: 6, marginBottom: 10, flexWrap: "wrap" }}>
        {ineq && <Tile label="Gini (now)" value={ineq.gini_now.toFixed(2)} color="#d88a8a" />}
        {ineq && <Tile label="Top-10% share" value={pct(ineq.top10_share_now)} color="#e0a86a" />}
        {ineq && <Tile label="Active houses" value={String(ineq.active_houses)} color="#7fd08a" />}
        {ineq && <Tile label="Defunct houses" value={String(ineq.defunct_houses)} color="#c0a0a0" />}
        {ineq && <Tile label="Rank churn" value={pct(ineq.rank_churn)} color="#8ab0e0" />}
      </div>
      <Grid2>
        {idx.length >= 2 && (
          <Chart title="World price index" rows={idx} color="#e0a86a" fmt={(v) => v.toFixed(2)} />
        )}
        {lackYearly.length >= 2 && (
          <StackedArea title="Unmet demand by tier (pop-weighted)"
            rows={lackYearly}
            keys={[{ name: "basic", color: "#c0573a" }, { name: "comfort", color: "#d9a441" }, { name: "luxury", color: "#8ab0e0" }]}
            fmt={pct} />
        )}
        {merchYearly.length >= 2 && (
          <StackedArea title="Merchant population by class"
            rows={merchYearly}
            keys={[{ name: "houses", color: "#d8b070" }, { name: "local", color: "#7fd0c0" }, { name: "guild", color: "#c08cff" }]}
            fmt={fmtNum} />
        )}
        {ineq && ineq.series.length >= 2 && (
          <MultiLine title="Inequality over time"
            rows={ineq.series.map((p) => [p.year, p.gini, p.top10_share])}
            keys={[{ name: "Gini", color: "#d88a8a" }, { name: "top-10% share", color: "#e0a86a" }]}
            fmt={(v) => v.toFixed(2)} />
        )}
        {ineq && ineq.series.length >= 2 && (
          <Chart title="Mean house wealth" rows={ineq.series.map((p) => [p.year, p.mean_wealth])}
            color="#d8b070" fmt={fmtNum} />
        )}
      </Grid2>
    </>
  );
}

/* ───────────────────────── Turmoil tab ───────────────────────── */

const TURMOIL_CATS = {
  war: { label: "⚔ War events", color: "#e08080" },
  crash: { label: "📉 Crashes", color: "#8ab0e0" },
  bank: { label: "🏦 Bank shocks", color: "#c9b96a" },
  plague: { label: "☠ Plagues", color: "#c07070" },
} as const;

function TurmoilTab({ turmoil, records }: {
  turmoil: { rows: { year: number; war: number; crash: number; bank: number; plague: number }[]; totals: Record<string, number> };
  records?: import("../types").WorldRecords;
}) {
  if (turmoil.rows.length === 0) {
    return <div style={hint}>No shocks chronicled yet — wars, crashes, bank failures and plagues appear here, charted by year, as history unfolds.</div>;
  }
  const keys = Object.keys(TURMOIL_CATS) as (keyof typeof TURMOIL_CATS)[];
  return (
    <>
      <div style={{ display: "flex", gap: 6, marginBottom: 10, flexWrap: "wrap" }}>
        {keys.map((k) => (
          <Tile key={k} label={TURMOIL_CATS[k].label} value={String(turmoil.totals[k] ?? 0)} color={TURMOIL_CATS[k].color} />
        ))}
      </div>
      <StackedBars title="Shocks per year — the world's turbulence"
        rows={turmoil.rows} keys={keys.map((k) => ({ name: k, color: TURMOIL_CATS[k].color, label: TURMOIL_CATS[k].label }))} />
      {records && (
        <div style={{ marginTop: 10 }}>
          <div style={sectionHdrLocal}>Worst on record</div>
          <RecordRow icon="📉" label="Worst crash" entry={records.worst_crash} fmt={(v) => `${Math.round(v)} cities hit`} />
          <RecordRow icon="☠" label="Deadliest plague strike" entry={records.deadliest_plague} fmt={(v) => `${fmtNum(v)} dead`} />
        </div>
      )}
    </>
  );
}

/* ───────────────────────── Records tab ───────────────────────── */

function RecordsTab({ records }: { records?: import("../types").WorldRecords }) {
  if (!records) return <div style={hint}>Advance time — records are written at each New Year.</div>;
  const rows: { icon: string; label: string; entry: [number, string, number]; fmt: (v: number) => string }[] = [
    { icon: "🏙", label: "Largest city ever", entry: records.largest_city, fmt: fmtNum },
    { icon: "⚜️", label: "Richest house ever", entry: records.richest_house, fmt: fmtNum },
    { icon: "🚢", label: "Greatest trade year", entry: records.biggest_trade_year, fmt: fmtNum },
    { icon: "🏘", label: "Most towns alive", entry: records.most_towns, fmt: (v) => String(Math.round(v)) },
    { icon: "⚭", label: "Longest dynasty", entry: records.longest_dynasty, fmt: (v) => `${Math.round(v)} generations` },
    { icon: "☠", label: "Deadliest plague strike", entry: records.deadliest_plague, fmt: (v) => `${fmtNum(v)} dead` },
    { icon: "📉", label: "Worst crash", entry: records.worst_crash, fmt: (v) => `${Math.round(v)} cities hit` },
  ];
  const set = rows.filter((r) => r.entry && (r.entry[0] > 0 || r.entry[1] !== ""));
  if (set.length === 0) return <div style={hint}>No records yet — advance at least a year.</div>;
  return (
    <>
      <div style={{ color: T.inkDim, fontSize: 11, marginBottom: 8 }}>
        The Hall of Records — all-time bests of this world, set the year they happened.
      </div>
      {set.map((r) => <RecordRow key={r.label} icon={r.icon} label={r.label} entry={r.entry} fmt={r.fmt} />)}
    </>
  );
}

function RecordRow({ icon, label, entry, fmt }: {
  icon: string; label: string; entry: [number, string, number]; fmt: (v: number) => string;
}) {
  if (!entry || (entry[0] <= 0 && entry[1] === "")) return null;
  return (
    <div style={{
      display: "flex", alignItems: "baseline", gap: 8,
      background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 6,
      padding: "7px 10px", marginBottom: 5,
    }}>
      <span style={{ fontSize: 14 }}>{icon}</span>
      <span style={{ color: T.inkDim, fontSize: 11, minWidth: 150 }}>{label}</span>
      <span style={{ fontFamily: SERIF, color: T.parchment, fontSize: 13, fontWeight: 700 }}>{entry[1] || "—"}</span>
      <span style={{ flex: 1 }} />
      <span style={{ color: T.gold, fontSize: 12, fontWeight: 700 }}>{fmt(entry[0])}</span>
      <span style={{ color: T.inkFaint, fontSize: 10 }}>Y{entry[2]}</span>
    </div>
  );
}

/* ───────────────────────── chart primitives ───────────────────────── */

/** Founding/abandonment tick marks derived from the cumulative counters. */
function lifecycleMarkers(series: [number, number, number, number, number, number][]) {
  const marks: { x: number; label: string; color: string }[] = [];
  for (let i = 1; i < series.length; i++) {
    if (series[i][4] > series[i - 1][4]) marks.push({ x: series[i][0], label: "▲", color: "#ffd75e" });
    if (series[i][5] > series[i - 1][5]) marks.push({ x: series[i][0], label: "†", color: "#c0a0a0" });
  }
  return marks;
}

/** Downsample a `[tick, …values]` monthly series to one row per YEAR (last
 *  sample in the year), returning `[year, value@col]`. */
function toYearly(rows: number[][], col: number): [number, number][] {
  const byYear = new Map<number, number>();
  for (const r of rows) byYear.set(Math.floor(r[0] / 365), r[col]);
  return [...byYear.entries()].sort((a, b) => a[0] - b[0]);
}

/** Downsample a `[tick, a, b, c]` monthly series to yearly `[year, a, b, c]`. */
function yearlyRows(rows: number[][]): number[][] {
  const byYear = new Map<number, number[]>();
  for (const r of rows) byYear.set(Math.floor(r[0] / 365), [Math.floor(r[0] / 365), r[1], r[2], r[3]]);
  return [...byYear.values()].sort((a, b) => a[0] - b[0]);
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
      <polyline points={pts} fill="none" stroke={dead ? "#6a6a6a" : "#6ab0e8"} strokeWidth={1} />
    </svg>
  );
}

/** Responsive 2-column card grid (collapses to 1 col in a narrow panel). */
function Grid2({ children }: { children: React.ReactNode }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(360px, 1fr))", gap: 8 }}>
      {children}
    </div>
  );
}

function CardShell({ title, right, children }: { title: string; right?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div style={{ background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 6, padding: "6px 8px" }}>
      <div style={{ display: "flex", alignItems: "baseline", marginBottom: 2 }}>
        <span style={{ color: T.inkDim, fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: 0.5 }}>{title}</span>
        <span style={{ flex: 1 }} />
        {right}
      </div>
      {children}
    </div>
  );
}

/** A titled SVG line chart over (x = year, y = value) rows. */
function Chart({ title, rows, color, fmt, markers }: {
  title: string; rows: [number, number][]; color: string;
  fmt: (v: number) => string; markers?: { x: number; label: string; color: string }[];
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
    <CardShell title={title} right={<span style={{ color, fontSize: 11, fontWeight: 700 }}>{fmt(ys[ys.length - 1])}</span>}>
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
    </CardShell>
  );
}

/** Multiple named series sharing one x axis, with a small legend. `rows` are
 *  `[x, s0, s1, …]`; `keys` names/colours each series column. */
function MultiLine({ title, rows, keys, fmt }: {
  title: string; rows: number[][]; keys: { name: string; color: string }[]; fmt: (v: number) => string;
}) {
  const w = 540, h = 96;
  const xs = rows.map((r) => r[0]);
  const x0 = Math.min(...xs), x1 = Math.max(...xs);
  let lo = Infinity, hi = -Infinity;
  for (const r of rows) for (let k = 0; k < keys.length; k++) { lo = Math.min(lo, r[k + 1]); hi = Math.max(hi, r[k + 1]); }
  const spanX = x1 - x0 || 1, spanY = (hi - lo) || 1;
  const px = (x: number) => ((x - x0) / spanX) * (w - 8) + 4;
  const py = (y: number) => h - 4 - ((y - lo) / spanY) * (h - 12) - 2;
  return (
    <CardShell title={title} right={<Legend keys={keys} />}>
      <svg width="100%" height={h} viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{ display: "block", background: "#0b1622", borderRadius: 4 }}>
        {keys.map((k, ki) => (
          <polyline key={ki} fill="none" stroke={k.color} strokeWidth={1.5}
            points={rows.map((r) => `${px(r[0]).toFixed(1)},${py(r[ki + 1]).toFixed(1)}`).join(" ")} />
        ))}
      </svg>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: T.inkFaint, marginTop: 1 }}>
        <span>Y{x0} · min {fmt(lo)}</span>
        <span>max {fmt(hi)} · Y{x1}</span>
      </div>
    </CardShell>
  );
}

/** A stacked-area chart: `rows` = `[x, a, b, c]`, one band per key. */
function StackedArea({ title, rows, keys, fmt }: {
  title: string; rows: number[][]; keys: { name: string; color: string }[]; fmt: (v: number) => string;
}) {
  const w = 540, h = 96;
  const xs = rows.map((r) => r[0]);
  const x0 = Math.min(...xs), x1 = Math.max(...xs);
  const spanX = x1 - x0 || 1;
  const totals = rows.map((r) => keys.reduce((s, _k, ki) => s + Math.max(0, r[ki + 1]), 0));
  const hi = Math.max(1e-6, ...totals);
  const px = (x: number) => ((x - x0) / spanX) * (w - 8) + 4;
  const py = (v: number) => h - 3 - (v / hi) * (h - 8);
  // Build cumulative bands bottom→top.
  const bands = keys.map((k, ki) => {
    const lowerAt = (r: number[]) => keys.slice(0, ki).reduce((s, _x, j) => s + Math.max(0, r[j + 1]), 0);
    const upperAt = (r: number[]) => lowerAt(r) + Math.max(0, r[ki + 1]);
    const top = rows.map((r) => `${px(r[0]).toFixed(1)},${py(upperAt(r)).toFixed(1)}`);
    const bot = [...rows].reverse().map((r) => `${px(r[0]).toFixed(1)},${py(lowerAt(r)).toFixed(1)}`);
    return { color: k.color, d: `${top.join(" ")} ${bot.join(" ")}` };
  });
  const lastTotal = totals[totals.length - 1];
  return (
    <CardShell title={title} right={<><Legend keys={keys} /><span style={{ color: T.inkMid, fontSize: 10, marginLeft: 6 }}>{fmt(lastTotal)}</span></>}>
      <svg width="100%" height={h} viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{ display: "block", background: "#0b1622", borderRadius: 4 }}>
        {bands.map((b, i) => <polygon key={i} points={b.d} fill={b.color} fillOpacity={0.55} stroke={b.color} strokeWidth={0.6} />)}
      </svg>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: T.inkFaint, marginTop: 1 }}>
        <span>Y{x0}</span><span>Y{x1}</span>
      </div>
    </CardShell>
  );
}

/** Yearly STACKED bar chart (counts). `rows` carry `{ year, [key]: n }`. */
function StackedBars({ title, rows, keys }: {
  title: string; rows: Record<string, number>[]; keys: { name: string; color: string; label: string }[];
}) {
  const w = 540, h = 120;
  const years = rows.map((r) => r.year);
  const x0 = Math.min(...years), x1 = Math.max(...years);
  const spanX = (x1 - x0) || 1;
  const totals = rows.map((r) => keys.reduce((s, k) => s + (r[k.name] || 0), 0));
  const hi = Math.max(1, ...totals);
  const bw = Math.max(2, Math.min(18, (w - 12) / Math.max(1, rows.length) - 2));
  const px = (yr: number) => 6 + ((yr - x0) / spanX) * (w - 12 - bw);
  return (
    <CardShell title={title} right={<Legend keys={keys} />}>
      <svg width="100%" height={h} viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{ display: "block", background: "#0b1622", borderRadius: 4 }}>
        {rows.map((r, ri) => {
          let acc = 0;
          return (
            <g key={ri}>
              {keys.map((k) => {
                const n = r[k.name] || 0;
                if (n <= 0) return null;
                const segH = (n / hi) * (h - 10);
                const yTop = h - 4 - acc - segH;
                acc += segH;
                return <rect key={k.name} x={px(r.year)} y={yTop} width={bw} height={segH} fill={k.color} fillOpacity={0.85} />;
              })}
            </g>
          );
        })}
      </svg>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: T.inkFaint, marginTop: 1 }}>
        <span>Y{x0}</span><span>peak {hi} / yr</span><span>Y{x1}</span>
      </div>
    </CardShell>
  );
}

/** Scatter plot with an optional log axis, a fitted trend line and a Pearson r.
 *  `pts` are `{x, y, label}`; r + fit are computed in the (possibly log) space. */
function Scatter({ title, pts, xLabel, yLabel, logX, logY, color, note, onPick }: {
  title: string; pts: { x: number; y: number; label?: string; mx?: number; my?: number }[];
  xLabel: string; yLabel: string; logX?: boolean; logY?: boolean; color: string; note?: string;
  onPick?: (x: number, y: number) => void;
}) {
  const w = 540, h = 150, padL = 6, padB = 4, padT = 4, padR = 6;
  const tx = (v: number) => (logX ? Math.log10(Math.max(1e-6, v)) : v);
  const ty = (v: number) => (logY ? Math.log10(Math.max(1e-6, v)) : v);
  const X = pts.map((p) => tx(p.x)), Y = pts.map((p) => ty(p.y));
  const x0 = Math.min(...X), x1 = Math.max(...X), y0 = Math.min(...Y), y1 = Math.max(...Y);
  const spanX = (x1 - x0) || 1, spanY = (y1 - y0) || 1;
  const px = (vx: number) => padL + ((tx(vx) - x0) / spanX) * (w - padL - padR);
  const py = (vy: number) => h - padB - ((ty(vy) - y0) / spanY) * (h - padB - padT);
  const r = pearson(X, Y);
  const fit = linFit(X, Y); // in transformed space
  // Trend line endpoints (invert transform for the label positions only).
  const inv = (logAx: boolean | undefined, v: number) => (logAx ? Math.pow(10, v) : v);
  const lx0 = inv(logX, x0), lx1 = inv(logX, x1);
  const ly0 = inv(logY, fit.m * x0 + fit.b), ly1 = inv(logY, fit.m * x1 + fit.b);
  return (
    <CardShell title={title} right={<span style={{ color: rColor(r), fontSize: 11, fontWeight: 700 }}>r = {r.toFixed(2)}</span>}>
      <svg width="100%" height={h} viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{ display: "block", background: "#0b1622", borderRadius: 4 }}>
        {/* trend line */}
        <line x1={px(lx0)} y1={py(ly0)} x2={px(lx1)} y2={py(ly1)} stroke={T.gold} strokeWidth={1.2} strokeDasharray="4 3" opacity={0.8} />
        {pts.map((p, i) => (
          <circle key={i} cx={px(p.x)} cy={py(p.y)} r={2.6} fill={color} fillOpacity={0.75}
            style={onPick && p.mx !== undefined ? { cursor: "pointer" } : undefined}
            onClick={onPick && p.mx !== undefined ? () => onPick(p.mx!, p.my!) : undefined}>
            {p.label && <title>{`${p.label}\n${xLabel}: ${fmtNum(p.x)}\n${yLabel}: ${fmtNum(p.y)}${onPick ? "\n(click to pin on map)" : ""}`}</title>}
          </circle>
        ))}
      </svg>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: T.inkFaint, marginTop: 2 }}>
        <span>{xLabel}{logX ? " (log)" : ""}</span>
        <span style={{ color: rColor(r) }}>{rDesc(r)}</span>
        <span>{yLabel}{logY ? " (log)" : ""}</span>
      </div>
      {note && <div style={{ color: T.inkDim, fontSize: 9.5, marginTop: 3, lineHeight: 1.4 }}>{note}</div>}
    </CardShell>
  );
}

/** A Pearson-r heat grid over every pair of the given variables. Heavy-tailed
 *  columns (flagged `log`) are correlated in log space so a couple of giant
 *  cities don't swamp the signal. Cell colour = sign (green +, red −), opacity =
 *  strength; hover any cell for the exact pair + r. */
function CorrMatrix({ vars, data }: {
  vars: { key: string; label: string; log?: boolean }[]; data: Record<string, number>[];
}) {
  if (data.length < 4 || vars.length < 2) {
    return <div style={hint}>Not enough cities yet for a correlation matrix.</div>;
  }
  const cols = vars.map((v) => data.map((d) => (v.log ? Math.log10(Math.max(1e-6, d[v.key])) : d[v.key])));
  const n = vars.length;
  const r: number[][] = cols.map((ci) => cols.map((cj) => pearson(ci, cj)));
  // Headline: the strongest positive and strongest negative pair (i < j).
  let bestPos = { r: 0, i: -1, j: -1 }, bestNeg = { r: 0, i: -1, j: -1 };
  for (let i = 0; i < n; i++) for (let j = i + 1; j < n; j++) {
    const v = r[i][j];
    if (v > bestPos.r) bestPos = { r: v, i, j };
    if (v < bestNeg.r) bestNeg = { r: v, i, j };
  }
  const cell = 30, labelW = 44, headH = 20;
  const w = labelW + n * cell, h = headH + n * cell;
  const varLabel = (idx: number) => vars[idx]?.label ?? "";
  return (
    <div style={{ overflowX: "auto", background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 6, padding: 6, marginBottom: 4 }}>
      {(bestPos.i >= 0 || bestNeg.i >= 0) && (
        <div style={{ fontSize: 10.5, color: T.inkMid, marginBottom: 6, lineHeight: 1.5 }}>
          {bestPos.i >= 0 && (
            <>Strongest link: <b style={{ color: "#7fd08a" }}>{varLabel(bestPos.i)} ↔ {varLabel(bestPos.j)}</b> (r {bestPos.r.toFixed(2)}, {rDesc(bestPos.r)}).</>
          )}
          {bestNeg.i >= 0 && Math.abs(bestNeg.r) > 0.15 && (
            <> Strongest trade-off: <b style={{ color: "#e08080" }}>{varLabel(bestNeg.i)} ↔ {varLabel(bestNeg.j)}</b> (r {bestNeg.r.toFixed(2)}, {rDesc(bestNeg.r)}).</>
          )}
        </div>
      )}
      <svg width={w} height={h} style={{ display: "block", fontFamily: "inherit" }}>
        {/* column headers */}
        {vars.map((v, j) => (
          <text key={`c${j}`} x={labelW + j * cell + cell / 2} y={headH - 6} textAnchor="middle"
            fontSize={9} fill={T.inkMid}>{v.label}</text>
        ))}
        {vars.map((vi, i) => (
          <g key={`r${i}`}>
            <text x={labelW - 4} y={headH + i * cell + cell / 2 + 3} textAnchor="end" fontSize={9} fill={T.inkMid}>{vi.label}</text>
            {vars.map((_vj, j) => {
              const val = r[i][j];
              const diag = i === j;
              const a = diag ? 0 : Math.min(0.85, Math.abs(val));
              const fill = diag ? "#132030" : (val >= 0 ? "#4cae7a" : "#c0573a");
              return (
                <g key={j}>
                  <rect x={labelW + j * cell} y={headH + i * cell} width={cell - 1.5} height={cell - 1.5}
                    rx={2} fill={fill} fillOpacity={diag ? 1 : a}>
                    {!diag && <title>{`${vars[i].label} ↔ ${vars[j].label}: r = ${val.toFixed(2)} (${rDesc(val)})`}</title>}
                  </rect>
                  <text x={labelW + j * cell + (cell - 1.5) / 2} y={headH + i * cell + (cell - 1.5) / 2 + 3}
                    textAnchor="middle" fontSize={8.5}
                    fill={diag ? T.inkFaint : (Math.abs(val) > 0.45 ? "#0b1420" : T.inkMid)}>
                    {diag ? "—" : val.toFixed(1).replace("0.", ".").replace("-.", "−.")}
                  </text>
                </g>
              );
            })}
          </g>
        ))}
      </svg>
      <div style={{ display: "flex", gap: 10, alignItems: "center", marginTop: 4, fontSize: 9, color: T.inkFaint }}>
        <span style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
          <span style={{ width: 10, height: 10, background: "#c0573a", borderRadius: 2 }} /> negative
        </span>
        <span style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
          <span style={{ width: 10, height: 10, background: "#4cae7a", borderRadius: 2 }} /> positive
        </span>
        <span>· deeper = stronger · hover a cell for the exact r{cols.length && data.length ? ` · n=${data.length} cities` : ""}</span>
      </div>
    </div>
  );
}

function Legend({ keys }: { keys: { name: string; color: string; label?: string }[] }) {
  return (
    <span style={{ display: "inline-flex", gap: 8 }}>
      {keys.map((k) => (
        <span key={k.name} style={{ display: "inline-flex", alignItems: "center", gap: 3, fontSize: 9, color: T.inkMid }}>
          <span style={{ width: 8, height: 8, borderRadius: 2, background: k.color, display: "inline-block" }} />
          {k.label ?? k.name}
        </span>
      ))}
    </span>
  );
}

/* ─────────────────────────── math + fmt ─────────────────────────── */

function pearson(x: number[], y: number[]): number {
  const n = x.length;
  if (n < 2) return 0;
  const mx = x.reduce((a, b) => a + b, 0) / n;
  const my = y.reduce((a, b) => a + b, 0) / n;
  let sxy = 0, sxx = 0, syy = 0;
  for (let i = 0; i < n; i++) { const dx = x[i] - mx, dy = y[i] - my; sxy += dx * dy; sxx += dx * dx; syy += dy * dy; }
  const d = Math.sqrt(sxx * syy);
  return d === 0 ? 0 : Math.max(-1, Math.min(1, sxy / d));
}

function linFit(x: number[], y: number[]): { m: number; b: number } {
  const n = x.length;
  if (n < 2) return { m: 0, b: y[0] ?? 0 };
  const mx = x.reduce((a, b) => a + b, 0) / n;
  const my = y.reduce((a, b) => a + b, 0) / n;
  let sxy = 0, sxx = 0;
  for (let i = 0; i < n; i++) { const dx = x[i] - mx; sxy += dx * (y[i] - my); sxx += dx * dx; }
  const m = sxx === 0 ? 0 : sxy / sxx;
  return { m, b: my - m * mx };
}

function rDesc(r: number): string {
  const a = Math.abs(r), dir = r >= 0 ? "positive" : "negative";
  if (a < 0.15) return "no correlation";
  if (a < 0.35) return `weak ${dir}`;
  if (a < 0.6) return `moderate ${dir}`;
  if (a < 0.8) return `strong ${dir}`;
  return `very strong ${dir}`;
}

function rColor(r: number): string {
  const a = Math.abs(r);
  if (a < 0.15) return T.inkDim;
  return r >= 0 ? "#7fd08a" : "#e08080";
}

function pct(v: number): string { return `${Math.round(v * 100)}%`; }

function fmtNum(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + "M";
  if (n >= 10_000) return (n / 1_000).toFixed(0) + "k";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "k";
  if (n < 1 && n > 0) return n.toFixed(2);
  return Math.round(n).toLocaleString();
}

/* ─────────────────────────── resize hook ─────────────────────────── */

/** Bottom-right resize grip: local width/height with sane minimums. */
function useResizable(initW: number, initH: number) {
  const [size, setSize] = useState({ w: initW, h: initH });
  const drag = useRef<{ sx: number; sy: number; sw: number; sh: number } | null>(null);
  const onResizeDown = useCallback((e: React.PointerEvent) => {
    e.stopPropagation();
    drag.current = { sx: e.clientX, sy: e.clientY, sw: size.w, sh: size.h };
    const move = (ev: PointerEvent) => {
      if (!drag.current) return;
      setSize({
        w: Math.max(620, Math.min(window.innerWidth - 40, drag.current.sw + ev.clientX - drag.current.sx)),
        h: Math.max(420, Math.min(window.innerHeight - 80, drag.current.sh + ev.clientY - drag.current.sy)),
      });
    };
    const up = () => { drag.current = null; window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up); };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    e.preventDefault();
  }, [size]);
  return { size, onResizeDown };
}

/* ─────────────────────────── styles ─────────────────────────── */

/** Census grid template (City · History spark · Pop · Δ · Trade · Wealth · Status). */
const CENSUS_COLS = "1.45fr 62px 0.7fr 0.55fr 0.7fr 0.7fr 1fr";

/** Mirror of the map overlay's basin palette (OverlayManager BASIN_COLORS). */
const BASIN_UI_COLORS = [
  "#4fd0c0", "#d8b24a", "#c08cff", "#7fd08a", "#e08a6a", "#6aa9e8",
  "#e0a0d0", "#c9c96a", "#8ad0e0", "#d88a8a", "#a9c96a", "#c0a06a",
];

function Tile({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div style={{ flex: 1, minWidth: 92, background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 6, padding: "5px 8px" }}>
      <div style={{ color: T.inkDim, fontSize: 8.5, fontWeight: 600, textTransform: "uppercase", letterSpacing: 0.5 }}>{label}</div>
      <div style={{ color: color ?? T.ink, fontSize: 14, fontWeight: 700 }}>{value}</div>
    </div>
  );
}

const num: React.CSSProperties = { textAlign: "right" };
const sectionHdrLocal: React.CSSProperties = {
  color: T.inkDim, fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: 0.6, marginBottom: 4,
};
const panel: React.CSSProperties = {
  position: "absolute", top: 56, left: 270, maxWidth: "94vw", zIndex: 120,
  display: "flex", flexDirection: "column",
  border: `1px solid ${T.line}`, borderRadius: 8,
  boxShadow: "0 12px 34px rgba(0,0,0,0.55)", color: T.ink, fontSize: 12,
};
const header: React.CSSProperties = {
  display: "flex", alignItems: "center", padding: "7px 10px",
  borderBottom: `1px solid ${T.line}`,
};
const hint: React.CSSProperties = { color: T.inkDim, fontSize: 11, padding: 8 };
