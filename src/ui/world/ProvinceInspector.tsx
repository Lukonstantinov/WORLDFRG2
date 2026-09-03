import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import {
  campaignProvinceDetail, campaignProvinceGoods, campaignProvinceLand,
  campaignProvincePotential, campaignProvinceState, campaignSetProvinceTax,
  computeStates, getProvinceTerrainCrop, provinceGoodBeltMasks,
} from "@bridge";
import { koppenName } from "@ui/world/climate";
import {
  DEFAULT_PLATES, PlateToggles, ProvinceMiniMap, soilWord, type PlateKey,
} from "@ui/world/ProvinceMiniMap";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { ProvinceTradeView } from "@ui/world/ProvinceTradeView";
import {
  ELEV_WORD, borderKind, cellsToKm, goodEmoji, goodLabel, provinceFrontiers,
  provinceHistory, stars,
} from "@ui/world/provinceStory";
import { GOOD_DEFS } from "@goods";
import { T, FZ, SPACE, SERIF } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Section, Card, Badge, Meter as KitMeter, Tabs, Button, EmptyNote, FootNote } from "@ui/kit";
import type {
  Province, ProvinceDetail, ProvinceGoodExploit, ProvinceGoodMask, ProvinceLand, ProvinceLive, ProvinceTerrainCrop, PSettlement,
  ProvincePotential, StateRegion,
} from "@types";

/** 🏞 Province Inspector v2.0 — the dossier for ONE province, opened by clicking the
 *  map (or a row in the Provinces browser).
 *
 *  Rebuilt on the shared `@ui/kit` design system (the same tokens/primitives the
 *  Realms panel already uses) so the two read as one designed app instead of two —
 *  a province and the realm that may hold it now share type, colour and spacing.
 *  The manual "begin a work" controls are gone: land improvement (clearance,
 *  drainage, irrigation, roads) is now autonomous — a province under a realm is
 *  funded by its crown, otherwise by its own seat city once advanced enough — so
 *  the Holdings tab shows what's under way as a read-only status, not a button row.
 *
 *  Four tabs over a layered survey plate, with a year slider. The frozen geography
 *  still comes from the partition; the LAND — what is wooded, cropped, worn, held,
 *  taxed and resented — comes from the campaign, and changes. */

type Tab = "land" | "people" | "holdings" | "trade" | "chronicle";
const TABS: [Tab, string][] = [
  ["land", "Land"], ["people", "People"], ["holdings", "Holdings"], ["trade", "Trade"], ["chronicle", "Chronicle"],
];

/** `Realm.rank` — 0 city-state · 1 kingdom · 2 great power · 3 hegemon. */
const RANK_NAMES = ["City-state", "Kingdom", "Great power", "Hegemon"];

export function ProvinceInspector() {
  const open = useUIStore((s) => s.showProvinceInspector);
  const selectedId = useUIStore((s) => s.selectedProvince);
  const setSelected = useUIStore((s) => s.setSelectedProvince);
  const close = () => useUIStore.getState().setShowProvinceInspector(false);
  const setStatus = useUIStore((s) => s.setStatus);
  const setShowStates = useUIStore((s) => s.setShowStates);

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
  const [potential, setPotential] = useState<ProvincePotential | null>(null);
  const [states, setStates] = useState<StateRegion[]>([]);
  // Belt COVERAGE + QUALITY for each of the province's goods, sampled to its raster —
  // what lets the plate draw goods as AREAS + a quality wash like the main map (reads
  // the goods tile column, so it works on any world with no re-gen).
  const [goodMasks, setGoodMasks] = useState<ProvinceGoodMask[]>([]);
  const [goodSort, setGoodSort] = useState<"potential" | "quality">("quality");
  const [depositsOnly, setDepositsOnly] = useState(false);
  // #9 · which belt goods are shown on the minimap "goods" plate (null = all).
  const [goodFilter, setGoodFilter] = useState<Set<string> | null>(null);
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
    if (!open || !p) { setDetail(null); setLive(null); setLand(null); setExploit([]); setPotential(null); return; }
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
    // #9 · the opportunity view — every good the land could yield + ore workings.
    campaignProvincePotential(p.id)
      .then((pot) => { if (!stale) setPotential(pot); })
      .catch(() => { if (!stale) setPotential(null); });
    // v2.0 · cohesion with the Realms panel — which crown (if any) holds this
    // province's sovereignty, read from the SAME persisted state the map tint and
    // the Realms panel both use, never re-derived.
    computeStates()
      .then((rows) => { if (!stale) setStates(rows); })
      .catch(() => { if (!stale) setStates([]); });
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

  // #9 · real per-good QUALITY (0..1) + world rank from the province's own shortlist
  // (`p.goods`); `belt` (coverage) is the fallback for goods below the shortlist. Used
  // by both the Goods panel and the minimap's untapped-goods squares. Kept ABOVE the
  // early return below so the hook count is stable across renders (rules of hooks).
  const goodQ = useMemo(() => new Map((p?.goods ?? []).map((g) => [g.good, g])), [p?.goods]);
  // §2.5 · the live exploitation reading per good (utilisation % + market share +
  // depletion), so the full goods list can show how hard each good is worked inline.
  const exploitMap = useMemo(() => new Map(exploit.map((g) => [g.good, g])), [exploit]);
  // F1 (slice 1) · real extent per good — nothing reported this before. Keyed off
  // the FULL fetched set (`goodMasks`), not the legend-filtered `shownMasks`, so a
  // good's own row always states its own area regardless of what's toggled on the map.
  const goodAreaMap = useMemo(() => new Map(goodMasks.map((m) => [m.good, m])), [goodMasks]);
  // Real quality (0..1): the province's full per-good grade (`good_quality`, best-patch
  // suitability) is the primary source — it differentiates every good; the top-6
  // shortlist rank and belt coverage are only fallbacks for pre-#9 worlds.
  const qualityOf = useMemo(() => {
    const gq = p?.good_quality;
    return (good: number, belt: number) =>
      (gq && gq[good] != null && gq[good] > 0 ? gq[good] : undefined)
      ?? goodQ.get(good)?.quality ?? belt;
  }, [p?.good_quality, goodQ]);
  const beltGoods = useMemo(() =>
    (potential?.goods ?? [])
      .filter((g) => !g.is_deposit)
      .map((g) => ({ good: g.good, name: g.name, quality: qualityOf(g.good, g.belt), marine: g.is_marine }))
      .sort((a, b) => b.quality - a.quality),
    [potential, qualityOf]);
  // The subset actually drawn on the minimap, after the legend filter.
  const shownBeltGoods = useMemo(() =>
    goodFilter ? beltGoods.filter((g) => goodFilter.has(g.name)) : beltGoods,
    [beltGoods, goodFilter]);
  // CLAUDE.md §8.19 (goods localities, shipped) Slice 6 · the REAL terroir localities, filtered by the
  // same goods legend as the symbols they replace. Only what the query actually
  // returned for this province is drawn — a good with no locality here gets no
  // square, and nothing is invented to fill the gap.
  const shownLocalities = useMemo(() =>
    (potential?.localities ?? []).filter((l) => !goodFilter || goodFilter.has(l.good)),
    [potential, goodFilter]);

  // Belt AREAS + quality for this province's goods. Fetched once per province for the
  // goods it is known to carry (`beltGoods`), then filtered client-side by the legend —
  // toggling a single good re-filters without a round trip. A stable key of the good
  // names keeps the effect from re-firing on every render of the memo.
  const beltGoodNames = beltGoods.map((g) => g.name).join(",");
  useEffect(() => {
    if (!open || !p || beltGoodNames === "") { setGoodMasks([]); return; }
    let stale = false;
    provinceGoodBeltMasks(p.id, beltGoodNames.split(","))
      .then((m) => { if (!stale) setGoodMasks(m); })
      .catch(() => { if (!stale) setGoodMasks([]); });
    return () => { stale = true; };
  }, [open, p?.id, beltGoodNames]); // eslint-disable-line react-hooks/exhaustive-deps
  // The subset drawn, after the legend filter (same rule as the localities/legend).
  const shownMasks = useMemo(() =>
    goodMasks.filter((m) => !goodFilter || goodFilter.has(m.good)),
    [goodMasks, goodFilter]);

  // v2.0 · which realm (if any) holds this province's sovereignty — a pure lookup
  // over the SAME `compute_states` read the map tint and the Realms panel use.
  const realm = useMemo(
    () => (p ? states.find((s) => s.province_ids.includes(p.id)) ?? null : null),
    [states, p],
  );

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

  // ── The rural-dues control verb. This is the one player-facing lever left in
  //    the Holdings tab — land improvement itself is autonomous now (v2.0).
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

  const realmColor = realm ? `rgb(${realm.color[0]},${realm.color[1]},${realm.color[2]})` : undefined;

  return (
    <Panel onPointerDown={onPointerDown} width={460} maxHeight="82vh"
      style={{ top: 80, right: 90, zIndex: 41, ...rootStyle }}>
      <PanelHeader icon="🏞" title={p.name} onClose={close} onDragStart={onPointerDown}
        right={<span style={{ color: T.inkFaint, fontSize: FZ.small }}>province</span>} />

      <PanelBody>
        {/* Identity */}
        <div style={{ color: T.inkMid, fontSize: FZ.body, marginBottom: SPACE.md }}>
          {p.culture} · {ELEV_WORD[p.elevation_class] ?? "country"} · {koppenName(p.koppen)}
          {p.coastal ? " · coastal" : ""} · {fmt(p.area_km2)} km²
          {land && land.holder_name
            ? <> · <span title={land.holder_house >= 0
                ? "A merchant house holds this province's writ — the Stato da Mar case"
                : "The city whose writ runs here"}>writ of {land.holder_name}</span></>
            : land ? " · frontier" : ""}
        </div>

        {/* v2.0 · REALM / SOVEREIGNTY — cohesion with the Realms panel: same colour
            swatch, same rank vocabulary, one click to open the full dossier. */}
        {realm && (
          <div onClick={() => setShowStates(true)} title="Open the Realms panel"
            style={{
              display: "flex", alignItems: "center", gap: SPACE.sm, marginBottom: SPACE.md,
              padding: "5px 9px", borderRadius: 6, cursor: "pointer",
              background: T.card, border: `1px solid ${realmColor ?? T.lineSoft}55`,
            }}>
            <span style={{ width: 10, height: 10, borderRadius: 3, background: realmColor, flex: "0 0 auto" }} />
            <span style={{ fontFamily: SERIF, color: T.gold, fontWeight: 700, fontSize: FZ.base }}>
              {realm.title} of {realm.name}
            </span>
            <Badge tone="gold">{RANK_NAMES[realm.rank] ?? "Realm"}</Badge>
            <span style={{ flex: 1 }} />
            <span style={{ color: T.inkFaint, fontSize: FZ.tiny }}>cohesion {pct(realm.cohesion)} ›</span>
          </div>
        )}

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
          deposits={potential?.deposits ?? []}
          beltGoods={shownBeltGoods}
          localities={shownLocalities}
          goodMasks={shownMasks}
        />
        <div style={{ marginTop: 5 }}>
          <PlateToggles plates={plates} setPlates={setPlates}
            disabled={[
              ...((p.river_cells ?? 0) > 0 ? [] : (["water"] as PlateKey[])),
              // "goods" (F5 · merged coverage + quality) is live as soon as the belt
              // masks have any covered cell; localities/beltGoods keep it live on
              // worlds that carry them.
              ...(goodMasks.length > 0 || beltGoods.length > 0 || (potential?.localities.length ?? 0) > 0
                ? [] : (["goods"] as PlateKey[])),
              ...((potential?.deposits.length ?? 0) > 0 ? [] : (["deposits"] as PlateKey[])),
            ]} />
        </div>

        {/* #9 · goods legend + filter — the colour code of each surface good, click ONE
            to ISOLATE it (see only that good's best-quality area); click it again to show
            all. Only when the "goods" plate is on and there are belt goods. */}
        {plates.includes("goods") && beltGoods.length > 0 && (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 5, alignItems: "center" }}>
            <span style={{ fontSize: 10, color: T.inkFaint, marginRight: 2 }}>Goods:</span>
            {beltGoods.map((g) => {
              const isolated = !!goodFilter && goodFilter.size === 1 && goodFilter.has(g.name);
              const shown = !goodFilter || goodFilter.has(g.name);
              const col = GOOD_DEFS.find((d) => d.name === g.name)?.color ?? "#56c8d8";
              return (
                <button key={g.name} onClick={() => setGoodFilter((prev) => {
                  // Click = isolate this good; click the already-isolated good = show all.
                  if (prev && prev.size === 1 && prev.has(g.name)) return null;
                  return new Set([g.name]);
                })}
                  title={`${goodLabel(g.good)} · quality ${Math.round(g.quality * 100)}% — click to ${isolated ? "show all" : "show only this"}`}
                  style={{ display: "inline-flex", alignItems: "center", gap: 3, fontSize: 10, cursor: "pointer",
                    padding: "1px 5px", borderRadius: 10, border: `1px solid ${T.lineSoft}`,
                    background: shown ? T.raised : T.card, color: shown ? T.ink : T.inkFaint, opacity: shown ? 1 : 0.7 }}>
                  <span style={{ width: 8, height: 8, borderRadius: 2, background: col, opacity: shown ? 1 : 0.35, flexShrink: 0 }} />
                  {goodEmoji(g.good)} {goodLabel(g.good)}
                  <span style={{ color: T.gold, letterSpacing: 0.5, fontSize: 8 }}>{stars(g.quality)}</span>
                </button>
              );
            })}
            {goodFilter && (
              <button onClick={() => setGoodFilter(null)} title="Show all goods"
                style={{ fontSize: 10, cursor: "pointer", padding: "1px 6px", borderRadius: 10,
                  border: `1px solid ${TONE_GOOD_LINE}`, background: TONE_GOOD_FILL, color: T.goodInk }}>all</button>
            )}
          </div>
        )}

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
            <span style={{ fontSize: 12, color: T.inkMid, minWidth: 74, textAlign: "right" }}>
              {scrubbing ? `year ${sample!.year}` : "today"}
            </span>
            {scrubbing && (
              <Button variant="ghost" onClick={() => setScrub(null)}>today</Button>
            )}
          </div>
        )}

        {/* Tabs */}
        <Tabs tabs={TABS} active={tab} onSelect={setTab} style={{ margin: "10px 0 4px" }} />

        {/* ── LAND ─────────────────────────────────────────────────────────── */}
        {tab === "land" && (
          <>
            {land ? (
              <>
                <Section title={scrubbing ? `Land use · year ${sample!.year}` : "Land use"}>
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
                </Section>

                {/* v2.0 · read-only work status — replaces the old start/abandon
                    buttons. Land improvement is autonomous now: a realm funds its
                    own provinces in full; otherwise the seat city funds it once
                    advanced enough (see the Holdings tab for who and why). */}
                {land.works.length > 0 && (
                  <Section title="Under way">
                    {land.works.map((w) => (
                      <Card key={w.kind} style={{ marginBottom: SPACE.sm }}>
                        <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: FZ.body, marginBottom: 3 }}>
                          <span style={{ flex: 1, color: T.ink }}>
                            {w.label}{w.stalled && <span style={{ color: T.bad }}> · stalled, unpaid</span>}
                          </span>
                          <span style={{ color: T.inkDim, fontSize: FZ.small }}>
                            {w.years_left < 1 ? "<1y left" : `${Math.round(w.years_left)}y left`}
                          </span>
                        </div>
                        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                          <KitMeter value={w.progress} color={T.good} />
                          <span style={{ color: T.inkFaint, fontSize: FZ.tiny, whiteSpace: "nowrap" }}>
                            {Math.round(w.progress * 100)}%
                          </span>
                        </div>
                        <FootNote>funded by {w.funder || "an unknown patron"} · {fmt(w.yearly_cost)}/yr</FootNote>
                      </Card>
                    ))}
                  </Section>
                )}

                <Section title="The harvest">
                  <Row k="Surplus above subsistence" v={`${fmt(sample?.surplus ?? land.surplus)} /yr`}
                    trend={trend(history, scrub, "surplus")} />
                  <Row k="Dues collected" v={`${fmt(land.revenue)} /yr`} />
                  {land.arrears > 1 && <Row k="Arrears" v={fmt(land.arrears)} />}
                  <div style={{ color: T.inkDim, fontSize: 12, marginTop: 2 }}>
                    {land.holder_house >= 0
                      ? <>Grain still reaches the seat city's granary; the dues go instead
                          to {land.holder_name}'s own treasury.</>
                      : <>What the land grows above what the countryside eats and the
                          holder takes reaches {land.holder_name || "no city"}'s granary.</>}
                  </div>
                </Section>
              </>
            ) : (
              <EmptyNote>
                No live land state — begin the campaign and advance a year, and this
                province's woodland, soil, harvest and dues appear here (and start moving).
              </EmptyNote>
            )}

            {/* The frozen geography, now clearly secondary to the living land. */}
            <Section title="Climate & relief">
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
                  <div style={{ color: T.inkMid, fontSize: 12, marginTop: 2 }}>
                    {koppenShares.map(([k, s]) => `${pct(s)} ${koppenName(k)}`).join(" · ")}
                  </div>
                </>
              )}
            </Section>

            {/* F5 (slice 2) · "Currently worked" is deleted — it was a strict subset
                of "Goods of the region" below, which already prints
                `{actual}/yr of ~{potential}/yr · {exploitation}% worked ·
                {market_share}% to market` per good, plus potential and world rank. */}

            {/* #9 · POTENTIAL & DEPOSITS — every good the land could yield (richest
                first), so a province producing nothing still shows what's there. */}
            {(() => {
              const rows = (potential?.goods ?? []).filter((g) => !depositsOnly || g.is_deposit);
              if (rows.length === 0) {
                return exploit.length === 0
                  ? <><Section title="Goods" /><div style={{ color: T.inkFaint }}>no notable produce</div></>
                  : null;
              }
              // Quality (0..1): the province's own per-good rank where it has one, else
              // belt coverage. Deposits use their mean ore grade.
              const qualOf = (g: typeof rows[number]) =>
                g.is_deposit && g.workings > 0 ? g.mean_grade : qualityOf(g.good, g.belt);
              const sorted = [...rows].sort((a, b) =>
                goodSort === "quality" ? qualOf(b) - qualOf(a) : b.potential - a.potential);
              // Totals for the header: summed potential, mean quality, ore summary.
              const totalPot = rows.reduce((s, g) => s + g.potential, 0);
              const meanQ = rows.reduce((s, g) => s + qualOf(g), 0) / rows.length;
              const dep = potential?.deposits ?? [];
              const meanGrade = dep.length ? dep.reduce((s, d) => s + d.grade, 0) / dep.length : 0;
              const bestDepth = dep.reduce((m, d) => Math.max(m, d.depth), 0);
              return (
                <Section title={`Goods of the region · ${rows.length}`} right={
                  <div style={{ display: "flex", gap: 4 }}>
                    <button onClick={() => setGoodSort((s) => s === "quality" ? "potential" : "quality")}
                      title="Sort by land quality, or by potential yield"
                      style={sortBtn}>{goodSort === "quality" ? "by quality" : "by potential"}</button>
                    <button onClick={() => setDepositsOnly((v) => !v)}
                      title="Show only ore/mineral deposits"
                      style={{ ...sortBtn, color: depositsOnly ? T.gold : T.accent }}>💎 only</button>
                  </div>
                }>
                  {/* TOTAL — the whole province's producible worth at a glance. */}
                  <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "3px 6px", marginBottom: 5,
                    background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 4, fontSize: 12 }}>
                    <b style={{ color: T.goodInk }}>TOTAL</b>
                    <span style={{ color: T.ink }}>~{fmt(totalPot)}/yr potential</span>
                    <span style={{ color: T.gold, letterSpacing: 1 }} title={`mean land quality ${(meanQ * 100).toFixed(0)}%`}>{stars(meanQ)}</span>
                    <span style={{ flex: 1 }} />
                    {dep.length > 0 && (
                      <span style={{ color: T.inkMid, fontSize: 11 }}>
                        💎 {dep.length} deposit{dep.length === 1 ? "" : "s"} · grade {(meanGrade * 100).toFixed(0)}% · best {depthWord(bestDepth)}
                      </span>
                    )}
                  </div>
                  {sorted.map((g) => {
                    const q = qualOf(g);
                    const pg = goodQ.get(g.good);
                    const area = goodAreaMap.get(g.name);
                    return (
                      <div key={g.good} style={{ marginBottom: 4, opacity: g.actual > 1e-4 ? 1 : 0.94 }}>
                        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                          <span style={{ width: 132, color: T.ink }}>{goodEmoji(g.good)} {goodLabel(g.good)}</span>
                          <span style={{ color: T.gold, letterSpacing: 1 }}
                            title={`${g.grade_word} — ${g.is_deposit ? "mean ore grade" : "land quality"} ${(q * 100).toFixed(0)}%`}>{stars(q)}</span>
                          {g.is_deposit && g.workings > 0 ? (
                            <span style={{ color: T.inkDim, fontSize: 11 }}>{g.workings}× · {depthWord(g.best_depth)}</span>
                          ) : pg?.rank ? (
                            // F5 (slice 2) · promoted out of small grey text — a
                            // province's world rank in a good is the single most
                            // legible economic fact it carries.
                            pg.rank === 1
                              ? <Badge tone="gold">finest in the world</Badge>
                              : <Badge tone="neutral">#{pg.rank} of {pg.of}</Badge>
                          ) : null}
                          <span style={{ flex: 1 }} />
                          <span style={{ color: T.inkFaint, fontSize: 11 }}>
                            {fmt(g.potential)}/yr
                            {(() => {
                              const ex = exploitMap.get(g.good);
                              return ex
                                ? <span style={{ color: T.accent }}> · {Math.round(ex.exploitation * 100)}% worked · {Math.round(ex.market_share * 100)}% to market</span>
                                : <span> · untapped</span>;
                            })()}
                          </span>
                        </div>
                        {/* F1 (slice 1) · the good's real extent — nothing reported
                            this before. Only for a belt good (a deposit's extent is
                            already stated as its own workings ×/depth above). */}
                        {!g.is_deposit && area && area.cells > 0 && (
                          <div style={{ color: T.inkFaint, fontSize: 11, marginBottom: 1 }}>
                            {fmt(area.area_km2)} km² · {Math.round(area.land_share * 100)}% of the province's land
                          </div>
                        )}
                        {(() => {
                          const ex = exploitMap.get(g.good);
                          // For a WORKED good, the meter reads utilisation (how hard the
                          // land is being pushed); for an untapped one, its quality.
                          return ex
                            ? <MeterLabel frac={Math.min(1, ex.exploitation)} warn={ex.exploitation > 1}
                                label={`${fmt(ex.actual)}/yr of ~${fmt(g.potential)}/yr · ${Math.round(ex.exploitation * 100)}% worked` +
                                  (ex.depletion > 0.02 ? ` · ${Math.round(ex.depletion * 100)}% depleted` : "")} />
                            : <MeterLabel frac={Math.min(1, q)} label={`quality ${(q * 100).toFixed(0)}% · potential ${fmt(g.potential)}/yr · untapped`} />;
                        })()}
                      </div>
                    );
                  })}
                </Section>
              );
            })()}
          </>
        )}

        {/* ── PEOPLE ───────────────────────────────────────────────────────── */}
        {tab === "people" && (
          <>
            <Section title="People">
              <Row k="Rural" v={fmt(sample?.rural ?? rural)} trend={trend(history, scrub, "rural")} />
              {cap > 0 && (
                <>
                  <Row k="Carrying capacity" v={fmt(land?.rural_cap ?? cap)} />
                  <MeterLabel frac={saturation} warn={saturation > 1}
                    label={`${Math.round(saturation * 100)}% of what the land supports`} />
                </>
              )}
              <Row k="Urban" v={(sample?.urban ?? urban) ? fmt(sample?.urban ?? urban) : "—"}
                trend={trend(history, scrub, "urban")} />
              <Row k="Total" v={fmt((sample?.rural ?? rural) + (sample?.urban ?? urban))} />
              {live && live.net_migration < 0 && (
                <Row k="Migration" v={`↗ ${fmt(-live.net_migration)}/yr to the cities`} />
              )}
            </Section>

            {land && (
              <Section title="Discontent">
                {/* Unrest is rural here for the first time. Every major pre-modern
                    revolt was — Jacquerie, 1381, the Peasants' War — and it was a
                    city-only property in this model until now. */}
                <MeterLabel frac={sample?.unrest ?? land.unrest} warn={(sample?.unrest ?? land.unrest) > 0.5}
                  label={unrestWord(sample?.unrest ?? land.unrest)} />
                <Row k="Dues" v={pct(land.tax_rate)} />
                {land.arrears > 1 && (
                  <div style={{ color: T.inkDim, fontSize: 12 }}>
                    {fmt(land.arrears)} in arrears — dues assessed that never arrived.
                  </div>
                )}
              </Section>
            )}

            {shares.length > 0 && (
              <Section title="Peoples">
                <ShareBar rows={shares.map(([name, s]) => ({ label: name, share: s }))} />
                <div style={{ color: T.inkMid, fontSize: 12, marginTop: 2 }}>
                  {shares.map(([n, s]) => `${pct(s)} ${n}`).join(" · ")}
                </div>
              </Section>
            )}
          </>
        )}

        {/* ── HOLDINGS ─────────────────────────────────────────────────────── */}
        {tab === "holdings" && (
          <>
            <Section title="Settlements">
              {miniSettlements.length === 0 ? (
                <div style={{ color: T.inkFaint }}>frontier — no towns</div>
              ) : miniSettlements.slice().sort((a, b) => b.population - a.population).map((s, i) => (
                <div key={`${s.name}-${i}`} style={{ padding: "1px 0", color: T.ink }}>
                  {s.seat ? "★ " : "· "}{s.name} <span style={{ color: T.inkDim }}>{fmt(s.population)}</span>
                  {s.seat ? <span style={{ color: T.inkFaint }}> (seat)</span> : null}
                </div>
              ))}
            </Section>

            {land && (
              <>
                <Section title="Tenure">
                  {/* Who holds the land is the most consequential single variable in
                      pre-modern economic history, and the model had no answer to it. */}
                  {(["civic & crown", "house & noble", "temple", "common land"] as const).map((lbl, i) => (
                    <Row key={lbl} k={lbl} v={pct(land.tenure[i])} />
                  ))}
                  {land.holders.length > 0 && (
                    <>
                      <div style={{ color: T.inkMid, fontSize: 12, marginTop: 4 }}>Families holding estates here</div>
                      {land.holders.map((h) => (
                        <div key={h.house} style={{ display: "flex", alignItems: "center", gap: 6, padding: "1px 0" }}>
                          <span style={{ width: 9, height: 9, borderRadius: 2, background: h.color }} />
                          <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", color: T.ink }}>
                            {h.name}
                          </span>
                          <span style={{ color: T.inkDim, fontSize: 12 }}>
                            {h.estates} estate{h.estates === 1 ? "" : "s"}
                          </span>
                        </div>
                      ))}
                    </>
                  )}
                </Section>

                {/* ── The one remaining player verb: rural dues. Land improvement
                       itself is autonomous now (v2.0) — see "Under way" on the
                       Land tab for what's happening and who's paying. */}
                <Section title="Control">
                  {!canAct ? (
                    <div style={{ color: T.inkDim, fontSize: 12 }}>
                      No town administers this province — there is nobody to collect dues.
                    </div>
                  ) : (
                    <>
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <span style={{ color: T.inkMid, minWidth: 96 }}>Rural dues</span>
                        <input type="range" min={0} max={Math.round(land.tax_max * 100)}
                          value={Math.round(land.tax_rate * 100)} disabled={busy}
                          onChange={(e) => act(() =>
                            campaignSetProvinceTax(p.id, Number(e.target.value) / 100))}
                          style={{ flex: 1 }} />
                        <span style={{ minWidth: 34, textAlign: "right", color: T.ink }}>{pct(land.tax_rate)}</span>
                      </div>
                      <div style={{ color: T.inkDim, fontSize: 12, marginBottom: 4 }}>
                        Above about 15% the countryside resents it, evades more, and
                        eventually rises.
                      </div>
                      <FootNote>
                        {realm
                          ? `Land improvement here is funded by ${realm.name}'s own treasury — sovereignty grants full capability regardless of the seat's own advancement.`
                          : `Land improvement here begins on its own once ${land.holder_name || "the seat city"} is advanced enough to administer it, funded from its treasury.`}
                      </FootNote>
                    </>
                  )}
                </Section>
              </>
            )}

            {nd.length > 0 && (
              <Section title={`Borders (${nd.length})`}>
                {nd.slice(0, 8).map((b) => {
                  const kind = borderKind(b.kind);
                  const nb = provinces.find((q) => q.id === b.neighbor);
                  return (
                    <div key={b.neighbor} onClick={() => setSelected(b.neighbor)}
                      title={`Divided by ${kind.label}`}
                      style={{ display: "flex", gap: 8, alignItems: "center", padding: "2px 4px",
                        cursor: "pointer", borderRadius: 4 }}
                      onMouseEnter={(e) => (e.currentTarget.style.background = T.raised)}
                      onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}>
                      <span style={{ width: 16, textAlign: "center" }}>{kind.icon}</span>
                      <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", color: T.ink }}>
                        {nb?.name ?? `province ${b.neighbor}`}
                      </span>
                      <span style={{ color: T.inkDim, fontSize: 12 }}>{kind.label}</span>
                      <span style={{ color: T.inkFaint, fontSize: 12, width: 62, textAlign: "right" }}>
                        {cellKm > 0 ? cellsToKm(b.cells, cellKm) : `${b.cells} cells`}
                      </span>
                    </div>
                  );
                })}
              </Section>
            )}
          </>
        )}

        {/* ── TRADE ────────────────────────────────────────────────────────── */}
        {tab === "trade" && (
          <Section title="Commerce of the province">
            <ProvinceTradeView provinceId={p.id} reload={reload} />
          </Section>
        )}

        {/* ── CHRONICLE ────────────────────────────────────────────────────── */}
        {tab === "chronicle" && (
          <>
            {/* A city chronicle is a biography; a province chronicle is a history, and
                the province is the natural unit for one. */}
            <Section title="This country's history">
              {!land || land.events.length === 0 ? (
                <EmptyNote>
                  Nothing recorded yet. Clearances, dearths, revolts and finished works
                  are written down here as the campaign runs.
                </EmptyNote>
              ) : land.events.slice().reverse().map((e, i) => (
                <div key={i} style={{ display: "flex", gap: 8, padding: "2px 0" }}>
                  <span style={{ color: T.inkDim, width: 52, flex: "0 0 auto" }}>yr {e.year}</span>
                  <span style={{ width: 18, flex: "0 0 auto" }}>{eventIcon(e.kind)}</span>
                  <span style={{ flex: 1, color: T.ink }}>{e.text}</span>
                </div>
              ))}
            </Section>

            <Card>
              <div style={{ color: T.inkMid }}>🌍 <b style={{ color: T.ink }}>Looks most like</b></div>
              <div style={{ marginBottom: 8, color: T.ink }}>{p.analog}</div>
              <div style={{ color: T.inkMid }}>📜 <b style={{ color: T.ink }}>Character</b></div>
              <div style={{ fontStyle: "italic", color: T.ink }}>{provinceHistory(p, urban)}</div>
              {provinceFrontiers(p) && (
                <div style={{ color: T.inkMid, marginTop: 6 }}>{provinceFrontiers(p)}</div>
              )}
            </Card>
          </>
        )}
      </PanelBody>
    </Panel>
  );
}

const TONE_GOOD_LINE = "rgba(76,174,122,0.4)";
const TONE_GOOD_FILL = "rgba(76,174,122,0.16)";

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

/** #9 · Deposit depth (0 surface … 3 flooded) → a word for the workings note. */
function depthWord(d: number): string {
  return ["surface", "shallow", "deep", "flooded"][Math.max(0, Math.min(3, d))];
}

const sortBtn: React.CSSProperties = {
  background: T.raised, border: `1px solid ${T.line}`, color: T.accent,
  borderRadius: 4, fontSize: 10, padding: "1px 6px", cursor: "pointer", whiteSpace: "nowrap",
};

function Row({ k, v, trend }: { k: string; v: string; trend?: "up" | "down" | null }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", padding: "1px 0" }}>
      <span style={{ color: T.inkDim }}>{k}</span>
      <span style={{ color: T.ink }}>
        {v}
        {trend === "up" && <span style={{ color: T.good }} title="rising over the last ~20 years"> ▲</span>}
        {trend === "down" && <span style={{ color: T.bad }} title="falling over the last ~20 years"> ▼</span>}
      </span>
    </div>
  );
}

/** A labelled meter, over the shared `Meter` bar primitive. */
function MeterLabel({ frac, label, warn }: { frac: number; label: string; warn?: boolean }) {
  return (
    <div style={{ margin: "3px 0 5px" }}>
      <KitMeter value={Math.min(1, Math.max(0, frac))} color={warn ? T.bad : T.good} />
      <div style={{ color: T.inkDim, fontSize: 12 }}>{label}</div>
    </div>
  );
}

const SHARE_COLORS = [T.good, T.accent, T.gold, "#c98c62", "#9b7fc0"];

/** A single stacked bar for a share breakdown (peoples, climates). */
function ShareBar({ rows }: { rows: { label: string; share: number }[] }) {
  return (
    <div style={{ display: "flex", height: 8, borderRadius: 4, overflow: "hidden",
      marginTop: 6, background: T.card }}>
      {rows.map((r, i) => (
        <div key={r.label} title={`${r.label} ${Math.round(r.share * 100)}%`}
          style={{ width: `${r.share * 100}%`, background: SHARE_COLORS[i % SHARE_COLORS.length] }} />
      ))}
    </div>
  );
}
