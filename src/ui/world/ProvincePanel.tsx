import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore, decodeProvinceRaster } from "@state/worldStore";
import { useCampaignStore } from "@state/campaignStore";
import {
  simGenerateProvinces, simMergeSmallProvinces, simSplitLargeProvinces, campaignProvinceState,
  campaignProvinceDetail, campaignProvinceLandAll, getProvinceTerrainCrop,
  campaignProvinceGoods, campaignProvincePotential, provinceGoodBeltMasks,
} from "@bridge";
import { GOOD_DEFS } from "@goods";
import { koppenName } from "@ui/world/climate";
import { ProvinceMiniMap, soilWord } from "@ui/world/ProvinceMiniMap";
import type {
  Province, ProvinceLive, ProvinceDetail, ProvinceLand, ProvinceTerrainCrop, PSettlement,
  ProvinceGoodExploit, ProvincePotential, ProvinceGoodMask,
} from "@types";

import { ELEV_WORD, goodEmoji, goodLabel, provinceHistory, stars } from "@ui/world/provinceStory";

// ── Variant B "Split": a ranked/filterable list rail + a rich detail card. The
//    per-province dossier lives in ProvinceInspector (opened by a map click); this
//    stays the BROWSER — sort, filter, compare. Selecting here drives the map
//    highlight and the inspector, and a map click drives the selection here. ──

type SortKey = "total" | "area" | "rural" | "urban" | "quality" | "fertility"
  | "soil" | "surplus" | "unrest" | "woodland";
const SORTS: { key: SortKey; label: string }[] = [
  { key: "total", label: "Total pop" },
  { key: "area", label: "Area" },
  { key: "rural", label: "Rural" },
  { key: "urban", label: "Urban" },
  { key: "quality", label: "Good quality" },
  { key: "fertility", label: "Fertility" },
  // Live land state (FIX_PLAN B1) — only meaningful once a campaign has run a year,
  // and the only sort keys here that CHANGE over a campaign.
  { key: "soil", label: "Soil" },
  { key: "surplus", label: "Surplus" },
  { key: "unrest", label: "Unrest" },
  { key: "woodland", label: "Woodland" },
];

export function ProvincePanel() {
  const open = useUIStore((s) => s.showProvinces);
  const close = () => useUIStore.getState().setShowProvinces(false);
  const setStatus = useUIStore((s) => s.setStatus);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);

  const provinces = useWorldStore((s) => s.provinces);
  const settlements = useWorldStore((s) => s.settlements);
  const rivers = useWorldStore((s) => s.rivers);
  const setProvinces = useWorldStore((s) => s.setProvinces);

  const [sort, setSort] = useState<SortKey>("total");
  const [desc, setDesc] = useState(true);
  const [cultureFilter, setCultureFilter] = useState<string>("");
  const [cityFilter, setCityFilter] = useState<"all" | "cities" | "frontier">("all");
  const [goodFilter, setGoodFilter] = useState<number>(-1);
  const [selId, setSelId] = useState<number | null>(null);
  // Two-way with the map: clicking a province on the map moves this list's
  // selection, and clicking a row here highlights it on the map + opens its dossier.
  const mapSelection = useUIStore((s) => s.selectedProvince);
  const setSelectedProvince = useUIStore((s) => s.setSelectedProvince);
  useEffect(() => { if (mapSelection !== null) setSelId(mapSelection); }, [mapSelection]);
  // Provinces marked for the "affect only these" merge/split. Shift-click on the map or
  // the pin in each list row toggles a mark; an empty set means affect ALL provinces.
  const markedProvinces = useUIStore((s) => s.markedProvinces);
  const toggleMarkedProvince = useUIStore((s) => s.toggleMarkedProvince);
  const clearMarkedProvinces = useUIStore((s) => s.clearMarkedProvinces);
  // A running campaign FREEZES the world: regenerating/merging/splitting provinces would
  // recompact ids and desync the campaign's province/realm state, so those edits are
  // locked (the backend refuses too). Generation is a pre-campaign, world-pipeline step.
  const campaignActive = useCampaignStore((s) => s.snapshot?.active === true);
  const [granularity, setGranularity] = useState(0.5);
  const [busy, setBusy] = useState(false);
  const [live, setLive] = useState<Map<number, ProvinceLive> | null>(null);
  const [lands, setLands] = useState<Map<number, ProvinceLand> | null>(null);
  const [detail, setDetail] = useState<ProvinceDetail | null>(null);
  const provinceRaster = useWorldStore((s) => s.provinceRaster);

  // When open, pull LIVE campaign state (read-only). If a campaign is running its
  // hubs report real urban populations; otherwise we fall back to the worldgen sum.
  useEffect(() => {
    if (!open) return;
    campaignProvinceState()
      .then((rows) => {
        const hasLive = rows.some((r) => r.urban_pop > 0 || r.hub_count > 0);
        setLive(hasLive ? new Map(rows.map((r) => [r.id, r])) : null);
      })
      .catch(() => setLive(null));
    campaignProvinceLandAll()
      .then((rows) => setLands(rows.length > 0 ? new Map(rows.map((r) => [r.id, r])) : null))
      .catch(() => setLands(null));
  }, [open, provinces]);

  // Urban population per province: live campaign hubs when available, else Σ of the
  // worldgen settlements standing in the province.
  const urbanOf = useMemo(() => {
    const byId = new Map(settlements.map((s) => [s.id, s.population]));
    const m = new Map<number, number>();
    for (const p of provinces) {
      const liveU = live?.get(p.id)?.urban_pop;
      if (liveU !== undefined) { m.set(p.id, liveU); continue; }
      let u = 0;
      for (const id of p.settlements) u += byId.get(id) ?? 0;
      m.set(p.id, u);
    }
    return m;
  }, [provinces, settlements, live]);

  const cultures = useMemo(
    () => Array.from(new Set(provinces.map((p) => p.culture).filter(Boolean))).sort(),
    [provinces],
  );

  const rows = useMemo(() => {
    let list = provinces.slice();
    if (cultureFilter) list = list.filter((p) => p.culture === cultureFilter);
    if (cityFilter === "cities") list = list.filter((p) => p.settlements.length > 0);
    if (cityFilter === "frontier") list = list.filter((p) => p.settlements.length === 0);
    if (goodFilter >= 0) list = list.filter((p) => p.goods.some((g) => g.good === goodFilter));
    const val = (p: Province): number => {
      const urban = urbanOf.get(p.id) ?? 0;
      switch (sort) {
        case "area": return p.area_km2;
        case "rural": return live?.get(p.id)?.rural_pop ?? p.rural_pop;
        case "urban": return urban;
        case "quality": return p.goods[0]?.quality ?? 0;
        case "fertility": return p.mean_fertility;
        case "soil": return lands?.get(p.id)?.soil ?? -1;
        case "surplus": return lands?.get(p.id)?.surplus ?? -1;
        case "unrest": return lands?.get(p.id)?.unrest ?? -1;
        case "woodland": return lands?.get(p.id)?.forest ?? -1;
        default: return p.rural_pop + urban;
      }
    };
    list.sort((a, b) => (desc ? val(b) - val(a) : val(a) - val(b)));
    return list;
  }, [provinces, cultureFilter, cityFilter, goodFilter, sort, desc, urbanOf, live, lands]);

  const selected = useMemo(
    () => provinces.find((p) => p.id === selId) ?? rows[0] ?? null,
    [provinces, selId, rows],
  );

  // Fetch the selected province's live detail (settlements + buildings) for the
  // subwindow. Null when no campaign is running (falls back to worldgen settlements).
  useEffect(() => {
    if (!open || !selected) { setDetail(null); return; }
    let stale = false;
    campaignProvinceDetail(selected.id)
      .then((d) => { if (!stale) setDetail(d); })
      .catch(() => { if (!stale) setDetail(null); });
    return () => { stale = true; };
  }, [open, selected?.id, provinces]);

  // v2.0 · what's actually PRODUCED here (actual/potential/yr), not just the frozen
  // worldgen quality shortlist — the browser list used to be the only province view
  // with no real yield numbers at all, unlike the full Inspector dossier.
  const [exploit, setExploit] = useState<ProvinceGoodExploit[]>([]);
  const [potential, setPotential] = useState<ProvincePotential | null>(null);
  useEffect(() => {
    if (!open || !selected) { setExploit([]); setPotential(null); return; }
    let stale = false;
    campaignProvinceGoods(selected.id)
      .then((g) => { if (!stale) setExploit(g); })
      .catch(() => { if (!stale) setExploit([]); });
    campaignProvincePotential(selected.id)
      .then((pot) => { if (!stale) setPotential(pot); })
      .catch(() => { if (!stale) setPotential(null); });
    return () => { stale = true; };
  }, [open, selected?.id]);

  // The minimap's goods plate (belt coverage + quality wash) — the browser's
  // ProvinceMiniMap call never fetched this, so the plate toggled on but drew
  // nothing. Same fetch ProvinceInspector.tsx already makes for its own copy
  // of the survey plate; the two are separate minimap instances.
  const [goodMasks, setGoodMasks] = useState<ProvinceGoodMask[]>([]);
  const beltGoodNames = (potential?.goods ?? []).filter((g) => !g.is_deposit)
    .map((g) => g.name).join(",");
  useEffect(() => {
    if (!open || !selected || beltGoodNames === "") { setGoodMasks([]); return; }
    let stale = false;
    provinceGoodBeltMasks(selected.id, beltGoodNames.split(","))
      .then((m) => { if (!stale) setGoodMasks(m); })
      .catch(() => { if (!stale) setGoodMasks([]); });
    return () => { stale = true; };
  }, [open, selected?.id, beltGoodNames]);

  // The survey plate's real terrain crop (§2.3) — world geography, independent of
  // the campaign join above.
  const [terrain, setTerrain] = useState<ProvinceTerrainCrop | null>(null);
  useEffect(() => {
    if (!open || !selected) { setTerrain(null); return; }
    let stale = false;
    getProvinceTerrainCrop(selected.id)
      .then((t) => { if (!stale) setTerrain(t); })
      .catch(() => { if (!stale) setTerrain(null); });
    return () => { stale = true; };
  }, [open, selected?.id]);

  // Mini-map inputs: live campaign settlements+buildings when available, else the
  // worldgen settlements standing in the province (no buildings before a campaign).
  const miniSettlements: PSettlement[] = useMemo(() => {
    if (detail && detail.id === selected?.id) return detail.settlements;
    if (!selected) return [];
    const ids = new Set(selected.settlements);
    const inProv = settlements.filter((s) => ids.has(s.id));
    const maxPop = inProv.reduce((m, s) => Math.max(m, s.population), 0);
    return inProv.map((s) => ({
      name: s.name, x: s.x, y: s.y, population: s.population,
      seat: s.population === maxPop && maxPop > 0, hub_class: s.hubClass ?? 0, dev_tier: 0,
    }));
  }, [detail, selected, settlements]);

  const generate = async () => {
    if (busy) return;
    if (settlements.length === 0) { setStatus("Generate settlements first (Step 7) — provinces seed from them"); return; }
    setBusy(true);
    setStatus("Partitioning land into provinces…");
    try {
      const res = await simGenerateProvinces(settlements, rivers, granularity);
      setProvinces(res.provinces, decodeProvinceRaster(res));
      setOverlayVisible("provinces", true);
      setSelId(res.provinces[0]?.id ?? null);
      setStatus(`Generated ${res.provinces.length} provinces`);
    } catch (e) {
      setStatus(`Province generation failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  // Post-generation cleanup: absorb the tiny sliver provinces (never islands) into
  // their largest neighbour. Repeatable — each press folds the current crop of
  // sub-threshold provinces.
  const mergeSmall = async () => {
    if (busy || provinces.length === 0) return;
    const before = provinces.length;
    setBusy(true);
    const sel = markedProvinces.length ? markedProvinces : undefined;
    setStatus(sel ? `Merging ${sel.length} marked province${sel.length === 1 ? "" : "s"}…` : "Merging small provinces…");
    try {
      const res = await simMergeSmallProvinces(undefined, sel);
      setProvinces(res.provinces, decodeProvinceRaster(res));
      const removed = before - res.provinces.length;
      setSelId((id) => (res.provinces.some((p) => p.id === id) ? id : (res.provinces[0]?.id ?? null)));
      clearMarkedProvinces(); // ids are recompacted after the operation → old marks are stale
      setStatus(removed > 0 ? `Merged ${removed} province${removed === 1 ? "" : "s"} away`
                            : "No provinces to merge");
    } catch (e) {
      setStatus(`Merge failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  // Post-generation cleanup: split the oversized NON-POLAR provinces (huge deserts,
  // steppes) into compact sub-provinces. Arctic/antarctic are left uniform. Repeatable.
  const splitLarge = async () => {
    if (busy || provinces.length === 0) return;
    const before = provinces.length;
    setBusy(true);
    const sel = markedProvinces.length ? markedProvinces : undefined;
    setStatus(sel ? `Splitting ${sel.length} marked province${sel.length === 1 ? "" : "s"}…` : "Splitting large provinces…");
    try {
      const res = await simSplitLargeProvinces(undefined, rivers, sel);
      setProvinces(res.provinces, decodeProvinceRaster(res));
      const added = res.provinces.length - before;
      setSelId((id) => (res.provinces.some((p) => p.id === id) ? id : (res.provinces[0]?.id ?? null)));
      clearMarkedProvinces(); // ids are recompacted after the operation → old marks are stale
      setStatus(added > 0 ? `Split into ${added} more province${added === 1 ? "" : "s"}`
                          : markedProvinces.length ? "Marked provinces too small to split" : "No oversized non-polar provinces to split");
    } catch (e) {
      setStatus(`Split failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  if (!open) return null;

  const fmt = (n: number) => n.toLocaleString();
  const totalPop = (p: Province) => p.rural_pop + (urbanOf.get(p.id) ?? 0);

  return (
    <div style={{
      position: "absolute", top: 60, right: 60, width: 640, maxHeight: "82vh",
      background: "#0a1620", border: "1px solid #204058", borderRadius: 10,
      color: "#cfe3ef", font: "13px/1.4 system-ui, sans-serif", zIndex: 40,
      display: "flex", flexDirection: "column", boxShadow: "0 8px 30px rgba(0,0,0,.5)",
    }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "9px 12px",
        borderBottom: "1px solid #152535" }}>
        <strong style={{ fontSize: 14 }}>🗺 Provinces</strong>
        <span style={{ opacity: 0.6 }}>{provinces.length}</span>
        <div style={{ flex: 1 }} />
        <button onClick={close} style={btn}>×</button>
      </div>

      {/* Generate controls */}
      <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "8px 12px",
        borderBottom: "1px solid #152535", background: "#0c1a24" }}>
        <label style={{ opacity: 0.8 }}>Granularity</label>
        <input type="range" min={0} max={1} step={0.05} value={granularity}
          onChange={(e) => setGranularity(parseFloat(e.target.value))} style={{ flex: 1 }} />
        <span style={{ width: 74, opacity: 0.7, fontSize: 12 }}>
          {granularity < 0.34 ? "few · large" : granularity > 0.66 ? "many · small" : "balanced"}
        </span>
        <button onClick={generate} disabled={busy || campaignActive}
          title={campaignActive ? "Locked: a campaign is running — provinces are frozen" : ""}
          style={{ ...btn, background: busy || campaignActive ? "#1a2a38" : "#1d4d6b", opacity: campaignActive ? 0.5 : 1 }}>
          {busy ? "…" : provinces.length ? "Regenerate" : "Generate"}
        </button>
        {provinces.length > 0 && (
          <button onClick={mergeSmall} disabled={busy || campaignActive}
            title={campaignActive ? "Locked: a campaign is running — provinces are frozen"
              : markedProvinces.length
              ? `Merge the ${markedProvinces.length} marked province(s) into their neighbours`
              : "Absorb the tiniest sliver provinces (never islands) into their largest neighbour"}
            style={{ ...btn, background: busy || campaignActive ? "#1a2a38" : "#2a3d1d", opacity: campaignActive ? 0.5 : 1 }}>
            {markedProvinces.length ? "Merge marked" : "Merge small"}
          </button>
        )}
        {provinces.length > 0 && (
          <button onClick={splitLarge} disabled={busy || campaignActive}
            title={campaignActive ? "Locked: a campaign is running — provinces are frozen"
              : markedProvinces.length
              ? `Split the ${markedProvinces.length} marked province(s) — organic, feature-following`
              : "Split the oversized non-polar provinces (huge deserts/steppes) into smaller ones; arctic/antarctic left untouched"}
            style={{ ...btn, background: busy || campaignActive ? "#1a2a38" : "#3d2f1d", opacity: campaignActive ? 0.5 : 1 }}>
            {markedProvinces.length ? "Split marked" : "Split large"}
          </button>
        )}
        {campaignActive && (
          <span style={{ fontSize: 11, opacity: 0.6, marginLeft: 4 }}>🔒 frozen during campaign</span>
        )}
      </div>

      {/* Marked-set bar: what the merge/split above will affect. Shift-click a province on
          the map (or the 📌 in a row below) to mark it; empty = affect all. */}
      {provinces.length > 0 && (
        <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 12px",
          borderBottom: "1px solid #152535", background: "#0c1a24", fontSize: 12 }}>
          {markedProvinces.length ? (
            <>
              <span style={{ color: "#5adcf0" }}>◈ {markedProvinces.length} marked</span>
              <span style={{ opacity: 0.6 }}>— merge/split affect only these</span>
              <div style={{ flex: 1 }} />
              <button onClick={clearMarkedProvinces} style={btn}>Clear marks</button>
            </>
          ) : (
            <span style={{ opacity: 0.55 }}>
              Shift-click a province on the map (or 📌 a row) to mark it — merge/split then affect only marked.
            </span>
          )}
        </div>
      )}

      {provinces.length === 0 ? (
        <div style={{ padding: 20, opacity: 0.7 }}>
          No provinces yet. Provinces partition the land into watershed regions —
          they seed from your settlements, so run this after the Settlements step.
        </div>
      ) : (
        <>
          {/* Filters + sort */}
          <div style={{ display: "flex", flexWrap: "wrap", gap: 6, padding: "7px 12px",
            borderBottom: "1px solid #152535", alignItems: "center" }}>
            <select value={cultureFilter} onChange={(e) => setCultureFilter(e.target.value)} style={sel}>
              <option value="">All cultures</option>
              {cultures.map((c) => <option key={c} value={c}>{c}</option>)}
            </select>
            <select value={cityFilter} onChange={(e) => setCityFilter(e.target.value as any)} style={sel}>
              <option value="all">All</option>
              <option value="cities">With cities</option>
              <option value="frontier">Frontier</option>
            </select>
            <select value={goodFilter} onChange={(e) => setGoodFilter(parseInt(e.target.value))} style={sel}>
              <option value={-1}>Any good</option>
              {GOOD_DEFS.map((g, i) => <option key={i} value={i}>{g.label}</option>)}
            </select>
            <div style={{ flex: 1 }} />
            <select value={sort} onChange={(e) => setSort(e.target.value as SortKey)} style={sel}>
              {SORTS.map((s) => <option key={s.key} value={s.key}>{s.label}</option>)}
            </select>
            <button onClick={() => setDesc(!desc)} style={btn}>{desc ? "↓" : "↑"}</button>
          </div>

          {/* Split: list rail + detail card */}
          <div style={{ display: "flex", minHeight: 0, flex: 1 }}>
            {/* List rail */}
            <div style={{ width: 220, overflowY: "auto", borderRight: "1px solid #152535" }}>
              {rows.map((p) => {
                const isSel = selected?.id === p.id;
                const isMarked = markedProvinces.includes(p.id);
                return (
                  <div key={p.id} onClick={() => { setSelId(p.id); setSelectedProvince(p.id); }}
                    style={{ padding: "6px 10px", cursor: "pointer", display: "flex", gap: 6, alignItems: "center",
                      background: isSel ? "#12293a" : "transparent",
                      borderLeft: isSel ? "3px solid #3d9bd4" : isMarked ? "3px solid #5adcf0" : "3px solid transparent" }}>
                    <span onClick={(e) => { e.stopPropagation(); toggleMarkedProvince(p.id); }}
                      title={isMarked ? "Marked for merge/split — click to unmark" : "Mark for merge/split"}
                      style={{ cursor: "pointer", opacity: isMarked ? 1 : 0.3, fontSize: 12 }}>📌</span>
                    <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {p.name}
                    </span>
                    <span style={{ opacity: 0.55, fontSize: 12 }}>
                      {p.settlements.length === 0 ? "—" : fmt(Math.round(totalPop(p) / 1000)) + "k"}
                    </span>
                  </div>
                );
              })}
            </div>

            {/* Detail card */}
            <div style={{ flex: 1, overflowY: "auto", padding: "10px 14px" }}>
              {selected && (() => {
                const urban = urbanOf.get(selected.id) ?? 0;
                const towns = selected.settlements
                  .map((id) => settlements.find((s) => s.id === id))
                  .filter(Boolean)
                  .sort((a, b) => (b!.population - a!.population));
                return (
                  <div>
                    <div style={{ fontSize: 17, fontWeight: 600 }}>{selected.name}</div>
                    <div style={{ opacity: 0.7, marginBottom: 8 }}>
                      {selected.culture} · {ELEV_WORD[selected.elevation_class]} · {koppenName(selected.koppen)}
                      {selected.coastal ? " · coastal" : ""}
                    </div>

                    <Row k="Area" v={`${fmt(selected.area_km2)} km²`} />
                    <Row k="Rural" v={fmt(live?.get(selected.id)?.rural_pop ?? selected.rural_pop)} />
                    <Row k="Urban" v={urban ? fmt(urban) : "—"} />
                    {(() => {
                      const nm = live?.get(selected.id)?.net_migration ?? 0;
                      if (nm >= 0) return null;
                      return <Row k="Migration" v={`↗ ${fmt(-nm)}/yr to cities`} />;
                    })()}
                    <Row k="Total" v={fmt(totalPop(selected))} />
                    <Row k="Fertility" v={selected.mean_fertility.toFixed(2)} />

                    {/* Live land state (B1). Only present once a campaign has run a
                        year — before then the browser shows the frozen geography only. */}
                    {(() => {
                      const L = lands?.get(selected.id);
                      if (!L) return null;
                      return (
                        <>
                          <Row k="Soil" v={`${soilWord(L.soil)} (${L.soil.toFixed(2)})`} />
                          <Row k="Woodland" v={`${Math.round(L.forest * 100)}%`} />
                          <Row k="Arable" v={`${Math.round(L.arable * 100)}%`} />
                          <Row k="Surplus" v={`${Math.round(L.surplus).toLocaleString()} /yr`} />
                          {L.unrest > 0.1 && (
                            <Row k="Rural unrest" v={`${Math.round(L.unrest * 100)}%`} />
                          )}
                        </>
                      );
                    })()}

                    <div style={{ marginTop: 10, marginBottom: 4, opacity: 0.8, fontWeight: 600 }}>Holdings</div>
                    <ProvinceMiniMap
                      province={selected}
                      raster={provinceRaster}
                      settlements={miniSettlements}
                      buildings={detail && detail.id === selected.id ? detail.buildings : []}
                      land={lands?.get(selected.id) ?? null}
                      riverCells={selected.river_cells}
                      terrain={terrain}
                      rivers={rivers}
                      deposits={potential?.deposits ?? []}
                      localities={potential?.localities ?? []}
                      goodMasks={goodMasks}
                    />

                    <div style={{ marginTop: 10, marginBottom: 4, opacity: 0.8, fontWeight: 600 }}>
                      Goods produced
                    </div>
                    {(() => {
                      // v2.0 · real yield, not just the frozen quality shortlist: actual
                      // production where the campaign has any, live potential otherwise,
                      // falling all the way back to the worldgen quality list on a world
                      // with no campaign running yet (or a good below its top-6 shortlist).
                      const exploitByGood = new Map(exploit.map((g) => [g.good, g]));
                      const potByGood = new Map((potential?.goods ?? []).map((g) => [g.good, g]));
                      const rows = potential && potential.goods.length > 0
                        ? potential.goods.slice().sort((a, b) => {
                            const qa = a.is_deposit && a.workings > 0 ? a.mean_grade
                              : (a.actual > 1e-4 ? 1 : 0) * 10 + a.belt;
                            const qb = b.is_deposit && b.workings > 0 ? b.mean_grade
                              : (b.actual > 1e-4 ? 1 : 0) * 10 + b.belt;
                            return qb - qa;
                          })
                        : null;
                      const dep = potential?.deposits ?? [];
                      if (!rows) {
                        return selected.goods.length === 0 ? (
                          <div style={{ opacity: 0.5 }}>no notable produce</div>
                        ) : selected.goods.map((g) => (
                          <div key={g.good} style={{ display: "flex", gap: 8, alignItems: "center", padding: "1px 0" }}>
                            <span style={{ width: 130 }}>{goodEmoji(g.good)} {goodLabel(g.good)}</span>
                            <span style={{ color: "#e3c14a", letterSpacing: 1 }}>{stars(g.quality)}</span>
                          </div>
                        ));
                      }
                      if (rows.length === 0) return <div style={{ opacity: 0.5 }}>no notable produce</div>;
                      return (
                        <>
                          {dep.length > 0 && (
                            <div style={{ opacity: 0.7, fontSize: 11, marginBottom: 3 }}>
                              💎 {dep.length} deposit{dep.length === 1 ? "" : "s"} · mean grade{" "}
                              {Math.round(dep.reduce((s, d) => s + d.grade, 0) / dep.length * 100)}%
                            </div>
                          )}
                          {rows.slice(0, 10).map((g) => {
                            const ex = exploitByGood.get(g.good);
                            const q = g.is_deposit && g.workings > 0 ? g.mean_grade : g.belt;
                            return (
                              <div key={g.good} style={{ display: "flex", gap: 8, alignItems: "center", padding: "1px 0" }}>
                                <span style={{ width: 130 }}>{goodEmoji(g.good)} {goodLabel(g.good)}</span>
                                <span style={{ color: "#e3c14a", letterSpacing: 1, fontSize: 11 }}>{stars(q)}</span>
                                <span style={{ flex: 1 }} />
                                <span style={{ opacity: 0.6, fontSize: 11 }}>
                                  {ex && ex.actual > 1e-4
                                    ? `${Math.round(ex.actual).toLocaleString()}/yr`
                                    : g.actual > 1e-4
                                    ? `${Math.round(g.actual).toLocaleString()}/yr`
                                    : `~${Math.round(g.potential).toLocaleString()}/yr potential`}
                                </span>
                              </div>
                            );
                          })}
                        </>
                      );
                    })()}

                    <div style={{ marginTop: 10, marginBottom: 4, opacity: 0.8, fontWeight: 600 }}>
                      Settlements {towns.length ? `(${towns.length})` : ""}
                    </div>
                    {towns.length === 0 ? (
                      <div style={{ opacity: 0.5 }}>frontier — no towns</div>
                    ) : towns.map((s, i) => (
                      <div key={s!.id} style={{ padding: "1px 0" }}>
                        {i === 0 ? "★ " : "· "}{s!.name} <span style={{ opacity: 0.55 }}>{fmt(s!.population)}</span>
                        {i === 0 ? <span style={{ opacity: 0.5 }}> (seat)</span> : null}
                      </div>
                    ))}

                    <div style={{ marginTop: 12, padding: "8px 10px", background: "#0c1a24",
                      border: "1px solid #152535", borderRadius: 6 }}>
                      <div style={{ opacity: 0.8 }}>🌍 <b>Looks most like</b></div>
                      <div style={{ marginBottom: 8 }}>{selected.analog}</div>
                      <div style={{ opacity: 0.8 }}>📜 <b>History</b></div>
                      <div style={{ fontStyle: "italic", opacity: 0.9 }}>{provinceHistory(selected, urban)}</div>
                    </div>
                  </div>
                );
              })()}
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function Row({ k, v }: { k: string; v: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", padding: "1px 0" }}>
      <span style={{ opacity: 0.65 }}>{k}</span><span>{v}</span>
    </div>
  );
}

const btn: React.CSSProperties = {
  background: "#152535", color: "#cfe3ef", border: "1px solid #204058",
  borderRadius: 5, padding: "3px 9px", cursor: "pointer", fontSize: 13,
};
const sel: React.CSSProperties = {
  background: "#0c1a24", color: "#cfe3ef", border: "1px solid #204058",
  borderRadius: 5, padding: "3px 6px", fontSize: 12,
};
