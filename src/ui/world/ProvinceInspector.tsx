import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import {
  campaignCancelProvinceWork, campaignProvinceDetail, campaignProvinceGoods, campaignProvinceLand,
  campaignProvinceState, campaignSetProvinceTax, campaignStartProvinceWork,
  getProvinceTerrainCrop,
} from "@bridge";
import { koppenName } from "@ui/world/climate";
import {
  DEFAULT_PLATES, PlateToggles, ProvinceMiniMap, soilWord, type PlateKey,
} from "@ui/world/ProvinceMiniMap";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import {
  ELEV_WORD, borderKind, cellsToKm, goodEmoji, goodLabel, provinceFrontiers,
  provinceHistory, stars,
} from "@ui/world/provinceStory";
import type {
  Province, ProvinceDetail, ProvinceGoodExploit, ProvinceLand, ProvinceLive, ProvinceTerrainCrop, PSettlement,
} from "@types";

/** 🏞 Province Inspector — the dossier for ONE province, opened by clicking the map
 *  (or a row in the Provinces browser).
 *
 *  This used to be a flat list of twenty-five `<Row>`s: no hierarchy (coastline length
 *  had the same visual weight as fertility), no time, no comparison, and no agency. And
 *  everything except four live numbers was frozen at worldgen, so a province looked
 *  identical in year 1 and year 500.
 *
 *  It is now four tabs over a layered survey plate, with a year slider. The frozen
 *  geography still comes from the partition; the LAND — what is wooded, cropped, worn,
 *  held, taxed and resented — comes from the campaign, and changes. Fields added after
 *  the first province release are optional on the Rust side, so each block hides itself
 *  on a world generated before it, and the whole land section hides on a campaign that
 *  has not run a year. */

type Tab = "land" | "people" | "holdings" | "chronicle";
const TABS: [Tab, string][] = [
  ["land", "Land"], ["people", "People"], ["holdings", "Holdings"], ["chronicle", "Chronicle"],
];

const WORK_LABEL = ["Clear woodland", "Drain the marsh", "Build irrigation", "Make a road"];
const WORK_BLURB = [
  "woodland → arable, at the cost of the wood",
  "waste to crop, and a little less fever",
  "a durable lift to what the land yields",
  "dues arrive instead of falling into arrears",
];

export function ProvinceInspector() {
  const open = useUIStore((s) => s.showProvinceInspector);
  const selectedId = useUIStore((s) => s.selectedProvince);
  const setSelected = useUIStore((s) => s.setSelectedProvince);
  const close = () => useUIStore.getState().setShowProvinceInspector(false);
  const setStatus = useUIStore((s) => s.setStatus);

  const provinces = useWorldStore((s) => s.provinces);
  const settlements = useWorldStore((s) => s.settlements);
  const provinceRaster = useWorldStore((s) => s.provinceRaster);
  const worldRivers = useWorldStore((s) => s.rivers);
  const meta = useWorldStore((s) => s.meta);

  const [detail, setDetail] = useState<ProvinceDetail | null>(null);
  const [live, setLive] = useState<ProvinceLive | null>(null);
  const [land, setLand] = useState<ProvinceLand | null>(null);
  const [terrain, setTerrain] = useState<ProvinceTerrainCrop | null>(null);
  const [exploit, setExploit] = useState<ProvinceGoodExploit[]>([]);
  const [tab, setTab] = useState<Tab>("land");
  const [plates, setPlates] = useState<PlateKey[]>(DEFAULT_PLATES);
  /** Index into `land.history`; null = today. The slider is the whole point of the
   *  land layer — a plate at year 1 and year 500 that DIFFER is the visible proof
   *  that the campaign and the world are one simulation. */
  const [scrub, setScrub] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [reload, setReload] = useState(0);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.province);

  const p = useMemo(
    () => provinces.find((q) => q.id === selectedId) ?? null,
    [provinces, selectedId],
  );

  // Live campaign join (settlements + buildings + the province's own rural/urban/land).
  useEffect(() => {
    if (!open || !p) { setDetail(null); setLive(null); setLand(null); setExploit([]); return; }
    let stale = false;
    campaignProvinceDetail(p.id)
      .then((d) => { if (!stale) setDetail(d); })
      .catch(() => { if (!stale) setDetail(null); });
    campaignProvinceState()
      .then((rows) => { if (!stale) setLive(rows.find((r) => r.id === p.id) ?? null); })
      .catch(() => { if (!stale) setLive(null); });
    campaignProvinceLand(p.id)
      .then((l) => { if (!stale) setLand(l); })
      .catch(() => { if (!stale) setLand(null); });
    // §2.5 · the live exploitation reading, replacing the frozen "which goods +
    // quality" list once a campaign is actually producing something here.
    campaignProvinceGoods(p.id)
      .then((g) => { if (!stale) setExploit(g); })
      .catch(() => { if (!stale) setExploit([]); });
    return () => { stale = true; };
  }, [open, p?.id, provinces, reload]); // eslint-disable-line react-hooks/exhaustive-deps

  // The survey plate's real terrain crop (§2.3) — independent of the campaign join
  // above (world geography, not campaign state), so it fetches on province change
  // alone and doesn't need to re-run on `reload`.
  useEffect(() => {
    if (!open || !p) { setTerrain(null); return; }
    let stale = false;
    getProvinceTerrainCrop(p.id)
      .then((t) => { if (!stale) setTerrain(t); })
      .catch(() => { if (!stale) setTerrain(null); });
    return () => { stale = true; };
  }, [open, p?.id]);

  // A different province resets the scrub — a year index means nothing across provinces.
  useEffect(() => { setScrub(null); }, [p?.id]);

  const miniSettlements: PSettlement[] = useMemo(() => {
    if (detail && p && detail.id === p.id && detail.settlements.length > 0) return detail.settlements;
    if (!p) return [];
    const ids = new Set(p.settlements);
    const inProv = settlements.filter((s) => ids.has(s.id));
    const maxPop = inProv.reduce((m, s) => Math.max(m, s.population), 0);
    return inProv.map((s) => ({
      name: s.name, x: s.x, y: s.y, population: s.population,
      seat: s.population === maxPop && maxPop > 0, hub_class: s.hubClass ?? 0, dev_tier: 0,
    }));
  }, [detail, p, settlements]);

  if (!open || !p) return null;

  const fmt = (n: number) => Math.round(n).toLocaleString();
  const pct = (n: number) => `${Math.round(n * 100)}%`;
  // One cell's width in km at the equator — the same figure the partition uses for area.
  const cellKm = meta ? 40075 / meta.grid_width : 0;

  const history = land?.history ?? [];
  const sample = scrub !== null && history[scrub] ? history[scrub] : null;
  const scrubbing = sample !== null;

  const urban = live?.urban_pop ?? p.settlements
    .map((id) => settlements.find((s) => s.id === id)?.population ?? 0)
    .reduce((a, b) => a + b, 0);
  const rural = live?.rural_pop ?? p.rural_pop;
  const cap = p.rural_cap ?? 0;
  const saturation = land ? land.saturation : cap > 0 ? Math.min(1.5, rural / cap) : 0;

  const shares = p.culture_shares ?? [];
  const koppenShares = p.koppen_shares ?? [];
  const nd = p.neighbors_detail ?? [];

  // ── Control verbs. Only the holder may act, so a frontier province is read-only.
  const holderHub = land?.holder_hub ?? -1;
  const canAct = !!land && holderHub >= 0;
  const act = async (fn: () => Promise<string | number | void>) => {
    setBusy(true);
    try {
      const r = await fn();
      if (typeof r === "string") setStatus(r);
      setReload((n) => n + 1);
    } catch (e) {
      setStatus(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }} onPointerDown={onPointerDown}>
      {/* Header (drag handle) */}
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <strong style={{ fontSize: 14 }}>🏞 {p.name}</strong>
        <span style={{ opacity: 0.55, fontSize: 12 }}>province</span>
        <div style={{ flex: 1 }} />
        <button data-no-drag onClick={close} style={btn}>×</button>
      </div>

      <div style={{ overflowY: "auto", padding: "10px 14px", minHeight: 0 }}>
        {/* Identity */}
        <div style={{ opacity: 0.75, marginBottom: 8 }}>
          {p.culture} · {ELEV_WORD[p.elevation_class] ?? "country"} · {koppenName(p.koppen)}
          {p.coastal ? " · coastal" : ""} · {fmt(p.area_km2)} km²
          {land && land.holder_name
            ? <> · <span title={land.holder_house >= 0
                ? "A merchant house holds this province's writ — the Stato da Mar case"
                : "The city whose writ runs here"}>writ of {land.holder_name}</span></>
            : land ? " · frontier" : ""}
        </div>

        {/* ── The survey plate ───────────────────────────────────────────────── */}
        <ProvinceMiniMap
          province={p}
          raster={provinceRaster}
          settlements={miniSettlements}
          buildings={detail && detail.id === p.id ? detail.buildings : []}
          land={land}
          sample={sample}
          plates={plates}
          width={280}
          riverCells={p.river_cells}
          terrain={terrain}
          rivers={worldRivers}
        />
        <div style={{ marginTop: 5 }}>
          <PlateToggles plates={plates} setPlates={setPlates}
            disabled={[
              ...(land ? [] : (["landuse", "tenure"] as PlateKey[])),
              ...((p.river_cells ?? 0) > 0 ? [] : (["water"] as PlateKey[])),
            ]} />
        </div>

        {/* Year slider — only meaningful once the campaign has run some years. */}
        {history.length > 1 && (
          <div style={{ marginTop: 6, display: "flex", alignItems: "center", gap: 7 }}>
            <input type="range" min={0} max={history.length - 1}
              value={scrub ?? history.length - 1}
              onChange={(e) => {
                const v = Number(e.target.value);
                setScrub(v === history.length - 1 ? null : v);
              }}
              style={{ flex: 1 }} />
            <span style={{ fontSize: 12, opacity: 0.8, minWidth: 74, textAlign: "right" }}>
              {scrubbing ? `year ${sample!.year}` : "today"}
            </span>
            {scrubbing && (
              <button onClick={() => setScrub(null)} style={btnSm}>today</button>
            )}
          </div>
        )}

        {/* Tabs */}
        <div style={{ display: "flex", gap: 2, margin: "10px 0 4px", borderBottom: "1px solid #22342a" }}>
          {TABS.map(([id, label]) => (
            <div key={id} onClick={() => setTab(id)} style={{
              padding: "3px 9px", cursor: "pointer", fontSize: 12,
              fontWeight: tab === id ? 700 : 400,
              color: tab === id ? "#e6f2ea" : "#7f9a8a",
              borderBottom: tab === id ? "2px solid #7fb069" : "2px solid transparent",
            }}>{label}</div>
          ))}
        </div>

        {/* ── LAND ─────────────────────────────────────────────────────────── */}
        {tab === "land" && (
          <>
            {land ? (
              <>
                <Section title={scrubbing ? `Land use · year ${sample!.year}` : "Land use"} />
                <Row k="Woodland" v={pct(sample?.forest ?? land.forest)}
                  trend={trend(history, scrub, "forest")} />
                <Row k="Arable" v={pct(sample?.arable ?? land.arable)}
                  trend={trend(history, scrub, "arable")} />
                <Row k="Pasture" v={pct(sample?.pasture ?? land.pasture)} />
                {(sample?.irrigated ?? land.irrigated) > 0.005 && (
                  <Row k="Irrigated" v={pct(sample?.irrigated ?? land.irrigated)} />
                )}
                <Row k="Soil" v={`${soilWord(sample?.soil ?? land.soil)} (${(sample?.soil ?? land.soil).toFixed(2)})`}
                  trend={trend(history, scrub, "soil")} />

                <Section title="The harvest" />
                <Row k="Surplus above subsistence" v={`${fmt(sample?.surplus ?? land.surplus)} /yr`}
                  trend={trend(history, scrub, "surplus")} />
                <Row k="Dues collected" v={`${fmt(land.revenue)} /yr`} />
                {land.arrears > 1 && <Row k="Arrears" v={fmt(land.arrears)} />}
                <div style={{ opacity: 0.6, fontSize: 12, marginTop: 2 }}>
                  {land.holder_house >= 0
                    ? <>Grain still reaches the seat city's granary; the dues go instead
                        to {land.holder_name}'s own treasury.</>
                    : <>What the land grows above what the countryside eats and the
                        holder takes reaches {land.holder_name || "no city"}'s granary.</>}
                </div>
              </>
            ) : (
              <div style={{ opacity: 0.6, fontSize: 12, padding: "4px 0" }}>
                No live land state — begin the campaign and advance a year, and this
                province's woodland, soil, harvest and dues appear here (and start moving).
              </div>
            )}

            {/* The frozen geography, now clearly secondary to the living land. */}
            <Section title="Climate & relief" />
            {p.elev_max_m !== undefined && (
              <Row k="Elevation" v={`${fmt(p.elev_min_m ?? 0)} – ${fmt(p.elev_max_m)} m (mean ${fmt(p.elev_mean_m ?? 0)} m)`} />
            )}
            {p.relief_m !== undefined && <Row k="Relief" v={`${fmt(p.relief_m)} m`} />}
            {p.temp_mean !== undefined && <Row k="Temperature" v={`${p.temp_mean.toFixed(1)} °C`} />}
            {p.season_amp !== undefined && p.season_amp > 0 && (
              <Row k="Seasonality" v={`±${p.season_amp.toFixed(1)} °C`} />
            )}
            {p.precip_mean !== undefined && p.precip_mean > 0 && (
              <Row k="Rainfall" v={`${fmt(p.precip_mean)} mm/yr`} />
            )}
            {p.arid_frac !== undefined && p.arid_frac > 0.02 && (
              <Row k="Arid land" v={pct(p.arid_frac)} />
            )}
            <Row k="Fertility" v={p.mean_fertility.toFixed(2)} />
            {p.disease_mean !== undefined && p.disease_mean > 0.02 && (
              <Row k="Fever risk" v={pct(p.disease_mean)} />
            )}
            {cellKm > 0 && (p.coast_cells ?? 0) > 0 && (
              <Row k="Coastline" v={cellsToKm(p.coast_cells!, cellKm)} />
            )}
            {cellKm > 0 && (p.river_cells ?? 0) > 0 && (
              <Row k="Rivers" v={`${cellsToKm(p.river_cells!, cellKm)}${p.navigable_river ? " · navigable" : ""}`} />
            )}
            {(p.lake_cells ?? 0) > 0 && <Row k="Lakeshore" v={`${fmt(p.lake_cells!)} cells`} />}
            {koppenShares.length > 1 && (
              <>
                <ShareBar rows={koppenShares.map(([k, s]) => ({ label: koppenName(k), share: s }))} />
                <div style={{ opacity: 0.7, fontSize: 12, marginTop: 2 }}>
                  {koppenShares.map(([k, s]) => `${pct(s)} ${koppenName(k)}`).join(" · ")}
                </div>
              </>
            )}

            <Section title="Goods" />
            {land && exploit.length > 0 ? (
              // §2.5 · LIVE exploitation — only what is actually produced here,
              // no unexploited-opportunity view. Replaces the frozen quality/rank
              // shortlist the moment a campaign is actually growing something.
              exploit.map((g) => (
                <div key={g.good} style={{ marginBottom: 5 }}>
                  <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                    <span style={{ width: 132 }}>{goodEmoji(g.good)} {goodLabel(g.good)}</span>
                    <span style={{ opacity: 0.7, fontSize: 12 }}>{fmt(g.actual)}/yr</span>
                    <span style={{ flex: 1 }} />
                    <span style={{ opacity: 0.6, fontSize: 12 }} title="share leaving the province via trade">
                      {Math.round(g.market_share * 100)}% to market
                    </span>
                  </div>
                  <Meter frac={Math.min(1, g.exploitation)} warn={g.exploitation > 1}
                    label={`${Math.round(g.exploitation * 100)}% worked` +
                      (g.depletion > 0.02 ? ` · ${Math.round(g.depletion * 100)}% depleted` : "")} />
                </div>
              ))
            ) : p.goods.length === 0 ? (
              <div style={{ opacity: 0.5 }}>no notable produce</div>
            ) : p.goods.map((g) => (
              <div key={g.good} style={{ display: "flex", gap: 8, alignItems: "center", padding: "1px 0" }}>
                <span style={{ width: 132 }}>{goodEmoji(g.good)} {goodLabel(g.good)}</span>
                <span style={{ color: "#e3c14a", letterSpacing: 1 }}>{stars(g.quality)}</span>
                {g.rank ? (
                  <span style={{ opacity: 0.65, fontSize: 12 }}>
                    {g.rank === 1
                      ? <b style={{ color: "#e3c14a" }}>finest in the world</b>
                      : `#${g.rank} of ${g.of}`}
                  </span>
                ) : null}
              </div>
            ))}
          </>
        )}

        {/* ── PEOPLE ───────────────────────────────────────────────────────── */}
        {tab === "people" && (
          <>
            <Section title="People" />
            <Row k="Rural" v={fmt(sample?.rural ?? rural)} trend={trend(history, scrub, "rural")} />
            {cap > 0 && (
              <>
                <Row k="Carrying capacity" v={fmt(land?.rural_cap ?? cap)} />
                <Meter frac={saturation} warn={saturation > 1}
                  label={`${Math.round(saturation * 100)}% of what the land supports`} />
              </>
            )}
            <Row k="Urban" v={(sample?.urban ?? urban) ? fmt(sample?.urban ?? urban) : "—"}
              trend={trend(history, scrub, "urban")} />
            <Row k="Total" v={fmt((sample?.rural ?? rural) + (sample?.urban ?? urban))} />
            {live && live.net_migration < 0 && (
              <Row k="Migration" v={`↗ ${fmt(-live.net_migration)}/yr to the cities`} />
            )}

            {land && (
              <>
                <Section title="Discontent" />
                {/* Unrest is rural here for the first time. Every major pre-modern
                    revolt was — Jacquerie, 1381, the Peasants' War — and it was a
                    city-only property in this model until now. */}
                <Meter frac={sample?.unrest ?? land.unrest} warn={(sample?.unrest ?? land.unrest) > 0.5}
                  label={unrestWord(sample?.unrest ?? land.unrest)} />
                <Row k="Dues" v={pct(land.tax_rate)} />
                {land.arrears > 1 && (
                  <div style={{ opacity: 0.65, fontSize: 12 }}>
                    {fmt(land.arrears)} in arrears — dues assessed that never arrived.
                  </div>
                )}
              </>
            )}

            {shares.length > 0 && (
              <>
                <Section title="Peoples" />
                <ShareBar rows={shares.map(([name, s]) => ({ label: name, share: s }))} />
                <div style={{ opacity: 0.7, fontSize: 12, marginTop: 2 }}>
                  {shares.map(([n, s]) => `${pct(s)} ${n}`).join(" · ")}
                </div>
              </>
            )}
          </>
        )}

        {/* ── HOLDINGS ─────────────────────────────────────────────────────── */}
        {tab === "holdings" && (
          <>
            <Section title="Settlements" />
            {miniSettlements.length === 0 ? (
              <div style={{ opacity: 0.5 }}>frontier — no towns</div>
            ) : miniSettlements.slice().sort((a, b) => b.population - a.population).map((s, i) => (
              <div key={`${s.name}-${i}`} style={{ padding: "1px 0" }}>
                {s.seat ? "★ " : "· "}{s.name} <span style={{ opacity: 0.55 }}>{fmt(s.population)}</span>
                {s.seat ? <span style={{ opacity: 0.5 }}> (seat)</span> : null}
              </div>
            ))}

            {land && (
              <>
                <Section title="Tenure" />
                {/* Who holds the land is the most consequential single variable in
                    pre-modern economic history, and the model had no answer to it. */}
                {(["civic & crown", "house & noble", "temple", "common land"] as const).map((lbl, i) => (
                  <Row key={lbl} k={lbl} v={pct(land.tenure[i])} />
                ))}
                {land.holders.length > 0 && (
                  <>
                    <div style={{ opacity: 0.7, fontSize: 12, marginTop: 4 }}>Families holding estates here</div>
                    {land.holders.map((h) => (
                      <div key={h.house} style={{ display: "flex", alignItems: "center", gap: 6, padding: "1px 0" }}>
                        <span style={{ width: 9, height: 9, borderRadius: 2, background: h.color }} />
                        <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                          {h.name}
                        </span>
                        <span style={{ opacity: 0.6, fontSize: 12 }}>
                          {h.estates} estate{h.estates === 1 ? "" : "s"}
                        </span>
                      </div>
                    ))}
                  </>
                )}

                {/* ── CONTROL. Before this, campaign_advance was the only command that
                       mutated a running simulation. A province is the right place to
                       break that: a decision here has a PLACE the player can see. */}
                <Section title="Control" />
                {!canAct ? (
                  <div style={{ opacity: 0.6, fontSize: 12 }}>
                    No town administers this province — there is nobody to collect dues
                    or fund works.
                  </div>
                ) : (
                  <>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <span style={{ opacity: 0.65, minWidth: 96 }}>Rural dues</span>
                      <input type="range" min={0} max={Math.round(land.tax_max * 100)}
                        value={Math.round(land.tax_rate * 100)} disabled={busy}
                        onChange={(e) => act(() =>
                          campaignSetProvinceTax(p.id, Number(e.target.value) / 100))}
                        style={{ flex: 1 }} />
                      <span style={{ minWidth: 34, textAlign: "right" }}>{pct(land.tax_rate)}</span>
                    </div>
                    <div style={{ opacity: 0.6, fontSize: 12, marginBottom: 6 }}>
                      Above about 15% the countryside resents it, evades more, and
                      eventually rises.
                    </div>

                    {land.works.length > 0 && (
                      <>
                        <div style={{ opacity: 0.7, fontSize: 12 }}>Under way</div>
                        {land.works.map((w) => (
                          <div key={w.kind} style={{ marginBottom: 5 }}>
                            <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12 }}>
                              <span style={{ flex: 1 }}>
                                {w.label}
                                {w.stalled && <span style={{ color: "#c96a4a" }}> · stalled, unpaid</span>}
                              </span>
                              <span style={{ opacity: 0.6 }}>
                                {w.years_left < 1 ? "<1y" : `${Math.round(w.years_left)}y left`}
                              </span>
                              <button style={btnSm} disabled={busy}
                                onClick={() => act(() => campaignCancelProvinceWork(p.id, w.kind))}>
                                abandon
                              </button>
                            </div>
                            <Meter frac={w.progress} label={`${Math.round(w.progress * 100)}% · ${fmt(w.yearly_cost)}/yr from ${w.funder}`} />
                          </div>
                        ))}
                      </>
                    )}

                    <div style={{ opacity: 0.7, fontSize: 12, marginTop: 4 }}>Begin a work</div>
                    <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                      {WORK_LABEL.map((lbl, kind) => {
                        const running = land.works.some((w) => w.kind === kind);
                        return (
                          <button key={kind} style={{ ...btnSm, opacity: running || busy ? 0.45 : 1 }}
                            disabled={running || busy}
                            title={running ? "already under way" : WORK_BLURB[kind]}
                            onClick={() => act(() =>
                              campaignStartProvinceWork(p.id, kind, holderHub, -1))}>
                            {lbl}
                          </button>
                        );
                      })}
                    </div>
                    <div style={{ opacity: 0.55, fontSize: 12, marginTop: 3 }}>
                      Funded from {land.holder_house >= 0 ? "the seat city's" : `${land.holder_name}'s`} treasury, over years. Unpaid work
                      stalls rather than failing.
                    </div>
                  </>
                )}
              </>
            )}

            {nd.length > 0 && (
              <>
                <Section title={`Borders (${nd.length})`} />
                {nd.slice(0, 8).map((b) => {
                  const kind = borderKind(b.kind);
                  const nb = provinces.find((q) => q.id === b.neighbor);
                  return (
                    <div key={b.neighbor} onClick={() => setSelected(b.neighbor)}
                      title={`Divided by ${kind.label}`}
                      style={{ display: "flex", gap: 8, alignItems: "center", padding: "2px 4px",
                        cursor: "pointer", borderRadius: 4 }}
                      onMouseEnter={(e) => (e.currentTarget.style.background = "#1b2b22")}
                      onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}>
                      <span style={{ width: 16, textAlign: "center" }}>{kind.icon}</span>
                      <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {nb?.name ?? `province ${b.neighbor}`}
                      </span>
                      <span style={{ opacity: 0.55, fontSize: 12 }}>{kind.label}</span>
                      <span style={{ opacity: 0.5, fontSize: 12, width: 62, textAlign: "right" }}>
                        {cellKm > 0 ? cellsToKm(b.cells, cellKm) : `${b.cells} cells`}
                      </span>
                    </div>
                  );
                })}
              </>
            )}
          </>
        )}

        {/* ── CHRONICLE ────────────────────────────────────────────────────── */}
        {tab === "chronicle" && (
          <>
            {/* A city chronicle is a biography; a province chronicle is a history, and
                the province is the natural unit for one. */}
            <Section title="This country's history" />
            {!land || land.events.length === 0 ? (
              <div style={{ opacity: 0.6, fontSize: 12 }}>
                Nothing recorded yet. Clearances, dearths, revolts and finished works
                are written down here as the campaign runs.
              </div>
            ) : land.events.slice().reverse().map((e, i) => (
              <div key={i} style={{ display: "flex", gap: 8, padding: "2px 0" }}>
                <span style={{ opacity: 0.55, width: 52, flex: "0 0 auto" }}>yr {e.year}</span>
                <span style={{ width: 18, flex: "0 0 auto" }}>{eventIcon(e.kind)}</span>
                <span style={{ flex: 1 }}>{e.text}</span>
              </div>
            ))}

            <div style={{ marginTop: 12, padding: "8px 10px", background: "rgba(0,0,0,0.25)",
              border: "1px solid #24382c", borderRadius: 6 }}>
              <div style={{ opacity: 0.8 }}>🌍 <b>Looks most like</b></div>
              <div style={{ marginBottom: 8 }}>{p.analog}</div>
              <div style={{ opacity: 0.8 }}>📜 <b>Character</b></div>
              <div style={{ fontStyle: "italic", opacity: 0.9 }}>{provinceHistory(p, urban)}</div>
              {provinceFrontiers(p) && (
                <div style={{ opacity: 0.75, marginTop: 6 }}>{provinceFrontiers(p)}</div>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

/** Change in a land series between the shown year and ~20 years earlier. A trend arrow
 *  is what turns a number into a reading — "38% arable" says nothing, "38% and rising"
 *  says the countryside is clearing. */
type SeriesKey = "forest" | "arable" | "soil" | "surplus" | "rural" | "urban";
function trend(
  history: { year: number; forest: number; arable: number; soil: number; surplus: number; rural: number; urban: number }[],
  scrub: number | null, key: SeriesKey,
): "up" | "down" | null {
  if (history.length < 4) return null;
  const i = scrub ?? history.length - 1;
  const j = Math.max(0, i - 20);
  if (j === i) return null;
  const a = history[j][key], b = history[i][key];
  const scale = Math.max(1e-4, Math.abs(a));
  const rel = (b - a) / scale;
  if (rel > 0.04) return "up";
  if (rel < -0.04) return "down";
  return null;
}

function unrestWord(u: number): string {
  if (u >= 0.72) return "in open revolt";
  if (u >= 0.5) return "seething";
  if (u >= 0.3) return "grumbling";
  if (u >= 0.12) return "restless";
  return "quiet";
}

function eventIcon(kind: string): string {
  switch (kind) {
    case "revolt": return "🔥";
    case "dearth": return "🌾";
    case "clearance": return "🪓";
    case "drainage": return "💧";
    case "irrigation": return "🚰";
    case "road": return "🛣";
    case "tax": return "💰";
    case "holder": return "🏛";
    default: return "·";
  }
}

function Section({ title }: { title: string }) {
  return (
    <div style={{ marginTop: 12, marginBottom: 4, fontWeight: 600, opacity: 0.85,
      borderBottom: "1px solid #22342a", paddingBottom: 2 }}>{title}</div>
  );
}

function Row({ k, v, trend }: { k: string; v: string; trend?: "up" | "down" | null }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", padding: "1px 0" }}>
      <span style={{ opacity: 0.65 }}>{k}</span>
      <span>
        {v}
        {trend === "up" && <span style={{ color: "#7fb069" }} title="rising over the last ~20 years"> ▲</span>}
        {trend === "down" && <span style={{ color: "#c96a4a" }} title="falling over the last ~20 years"> ▼</span>}
      </span>
    </div>
  );
}

/** Rural population against what the land can actually feed. */
function Meter({ frac, label, warn }: { frac: number; label: string; warn?: boolean }) {
  return (
    <div style={{ margin: "3px 0 5px" }}>
      <div style={{ height: 6, background: "#16241b", borderRadius: 3, overflow: "hidden" }}>
        <div style={{ width: `${Math.min(100, frac * 100)}%`, height: "100%",
          background: warn ? "#c96a4a" : "#7fb069" }} />
      </div>
      <div style={{ opacity: 0.6, fontSize: 12 }}>{label}</div>
    </div>
  );
}

const SHARE_COLORS = ["#7fb069", "#5a9bd4", "#e3c14a", "#c98c62", "#9b7fc0"];

/** A single stacked bar for a share breakdown (peoples, climates). */
function ShareBar({ rows }: { rows: { label: string; share: number }[] }) {
  return (
    <div style={{ display: "flex", height: 8, borderRadius: 4, overflow: "hidden",
      marginTop: 6, background: "#16241b" }}>
      {rows.map((r, i) => (
        <div key={r.label} title={`${r.label} ${Math.round(r.share * 100)}%`}
          style={{ width: `${r.share * 100}%`, background: SHARE_COLORS[i % SHARE_COLORS.length] }} />
      ))}
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 80, right: 90, width: 460, maxHeight: "82vh",
  border: "1px solid #24382c", borderRadius: 10,
  color: "#d6e6da", font: "13px/1.45 system-ui, sans-serif", zIndex: 41,
  display: "flex", flexDirection: "column", boxShadow: "0 8px 30px rgba(0,0,0,.5)",
};
const header: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 8, padding: "9px 12px",
  borderBottom: "1px solid #1b2b22",
};
const btn: React.CSSProperties = {
  background: "#1b2b22", color: "#d6e6da", border: "1px solid #24382c",
  borderRadius: 5, padding: "3px 9px", cursor: "pointer", fontSize: 13,
};
const btnSm: React.CSSProperties = { ...btn, padding: "1px 7px", fontSize: 11 };
