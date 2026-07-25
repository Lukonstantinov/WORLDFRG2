import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { simGenerateProvinces, campaignProvinceState, campaignProvinceDetail } from "@bridge";
import { GOOD_DEFS } from "@goods";
import { koppenName } from "@ui/world/climate";
import { ProvinceMiniMap } from "@ui/world/ProvinceMiniMap";
import type { Province, ProvinceLive, ProvinceDetail, PSettlement } from "@types";

// ── Variant B "Split": a ranked/filterable list rail + a rich detail card. ──

type SortKey = "total" | "area" | "rural" | "urban" | "quality" | "fertility";
const SORTS: { key: SortKey; label: string }[] = [
  { key: "total", label: "Total pop" },
  { key: "area", label: "Area" },
  { key: "rural", label: "Rural" },
  { key: "urban", label: "Urban" },
  { key: "quality", label: "Good quality" },
  { key: "fertility", label: "Fertility" },
];

const ELEV_WORD = ["lowland", "hill country", "upland"];

function stars(q: number): string {
  const n = Math.max(0, Math.min(5, Math.round(q * 5)));
  return "★★★★★".slice(0, n) + "☆☆☆☆☆".slice(0, 5 - n);
}
function goodLabel(g: number): string { return GOOD_DEFS[g]?.label ?? `good ${g}`; }
function goodEmoji(g: number): string { return GOOD_DEFS[g]?.emoji ?? "•"; }

/** A short deterministic account of a province — terrain, people, signature good,
 *  and whether it is a frontier or a settled heartland. */
function provinceHistory(p: Province, urban: number): string {
  const terr = ELEV_WORD[p.elevation_class] ?? "country";
  const clim = koppenName(p.koppen).toLowerCase();
  const top = p.goods[0] ? goodLabel(p.goods[0].good).toLowerCase() : "mixed produce";
  const coast = p.coastal ? "coast-fronting " : "";
  const status = p.settlements.length === 0
    ? `It stands as open frontier, its ${terr} still unclaimed by any town.`
    : urban > p.rural_pop
      ? `A crowded ${p.settlements.length > 1 ? "heartland of several towns" : "town-centred region"}, its people press on the land.`
      : `A rural ${terr === "country" ? "province" : terr}, its folk work the fields and pastures.`;
  return `${p.name} is a ${coast}${clim} ${terr} of the ${p.culture} people, long known for its ${top}. ${status}`;
}

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
  const [granularity, setGranularity] = useState(0.5);
  const [busy, setBusy] = useState(false);
  const [live, setLive] = useState<Map<number, ProvinceLive> | null>(null);
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
        default: return p.rural_pop + urban;
      }
    };
    list.sort((a, b) => (desc ? val(b) - val(a) : val(a) - val(b)));
    return list;
  }, [provinces, cultureFilter, cityFilter, goodFilter, sort, desc, urbanOf, live]);

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
      setProvinces(res.provinces, { data: res.raster, w: res.raster_w, h: res.raster_h, gridW: res.grid_w, gridH: res.grid_h });
      setOverlayVisible("provinces", true);
      setSelId(res.provinces[0]?.id ?? null);
      setStatus(`Generated ${res.provinces.length} provinces`);
    } catch (e) {
      setStatus(`Province generation failed: ${e}`);
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
        <button onClick={generate} disabled={busy} style={{ ...btn, background: busy ? "#1a2a38" : "#1d4d6b" }}>
          {busy ? "…" : provinces.length ? "Regenerate" : "Generate"}
        </button>
      </div>

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
                return (
                  <div key={p.id} onClick={() => setSelId(p.id)}
                    style={{ padding: "6px 10px", cursor: "pointer", display: "flex", gap: 6,
                      background: isSel ? "#12293a" : "transparent",
                      borderLeft: isSel ? "3px solid #3d9bd4" : "3px solid transparent" }}>
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

                    <div style={{ marginTop: 10, marginBottom: 4, opacity: 0.8, fontWeight: 600 }}>Holdings</div>
                    <ProvinceMiniMap
                      province={selected}
                      raster={provinceRaster}
                      settlements={miniSettlements}
                      buildings={detail && detail.id === selected.id ? detail.buildings : []}
                    />

                    <div style={{ marginTop: 10, marginBottom: 4, opacity: 0.8, fontWeight: 600 }}>Goods (quality)</div>
                    {selected.goods.length === 0 ? (
                      <div style={{ opacity: 0.5 }}>no notable produce</div>
                    ) : selected.goods.map((g) => (
                      <div key={g.good} style={{ display: "flex", gap: 8, alignItems: "center", padding: "1px 0" }}>
                        <span style={{ width: 130 }}>{goodEmoji(g.good)} {goodLabel(g.good)}</span>
                        <span style={{ color: "#e3c14a", letterSpacing: 1 }}>{stars(g.quality)}</span>
                      </div>
                    ))}

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
