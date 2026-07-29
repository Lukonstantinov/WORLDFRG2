import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { campaignProvinceDetail, campaignProvinceState } from "@bridge";
import { koppenName } from "@ui/world/climate";
import { ProvinceMiniMap } from "@ui/world/ProvinceMiniMap";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import {
  ELEV_WORD, borderKind, cellsToKm, goodEmoji, goodLabel, provinceFrontiers,
  provinceHistory, stars,
} from "@ui/world/provinceStory";
import type { Province, ProvinceDetail, ProvinceLive, PSettlement } from "@types";

/** 🏞 Province Inspector — the dossier for ONE province, opened by clicking the map
 *  (or a row in the Provinces browser). Everything it shows comes from the frozen
 *  partition, except the live rural/urban/migration figures which are joined from a
 *  running campaign. Fields added after the first province release are optional on
 *  the Rust side, so each block hides itself on a world generated before it. */
export function ProvinceInspector() {
  const open = useUIStore((s) => s.showProvinceInspector);
  const selectedId = useUIStore((s) => s.selectedProvince);
  const setSelected = useUIStore((s) => s.setSelectedProvince);
  const close = () => useUIStore.getState().setShowProvinceInspector(false);

  const provinces = useWorldStore((s) => s.provinces);
  const settlements = useWorldStore((s) => s.settlements);
  const provinceRaster = useWorldStore((s) => s.provinceRaster);
  const meta = useWorldStore((s) => s.meta);

  const [detail, setDetail] = useState<ProvinceDetail | null>(null);
  const [live, setLive] = useState<ProvinceLive | null>(null);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.province);

  const p = useMemo(
    () => provinces.find((q) => q.id === selectedId) ?? null,
    [provinces, selectedId],
  );

  // Live campaign join (settlements + buildings + the province's own rural/urban).
  useEffect(() => {
    if (!open || !p) { setDetail(null); setLive(null); return; }
    let stale = false;
    campaignProvinceDetail(p.id)
      .then((d) => { if (!stale) setDetail(d); })
      .catch(() => { if (!stale) setDetail(null); });
    campaignProvinceState()
      .then((rows) => { if (!stale) setLive(rows.find((r) => r.id === p.id) ?? null); })
      .catch(() => { if (!stale) setLive(null); });
    return () => { stale = true; };
  }, [open, p?.id, provinces]); // eslint-disable-line react-hooks/exhaustive-deps

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
  // One cell's width in km at the equator — the same figure the partition uses for area.
  const cellKm = meta ? 40075 / meta.grid_width : 0;

  const urban = live?.urban_pop ?? p.settlements
    .map((id) => settlements.find((s) => s.id === id)?.population ?? 0)
    .reduce((a, b) => a + b, 0);
  const rural = live?.rural_pop ?? p.rural_pop;
  const cap = p.rural_cap ?? 0;
  const saturation = cap > 0 ? Math.min(1.5, rural / cap) : 0;

  const shares = p.culture_shares ?? [];
  const koppenShares = p.koppen_shares ?? [];
  const nd = p.neighbors_detail ?? [];

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
        </div>

        {/* ── People ── */}
        <Section title="People" />
        <Row k="Rural" v={fmt(rural)} />
        {cap > 0 && (
          <>
            <Row k="Carrying capacity" v={fmt(cap)} />
            <Meter frac={saturation} warn={saturation > 1}
              label={`${Math.round(saturation * 100)}% of what the land supports`} />
          </>
        )}
        <Row k="Urban" v={urban ? fmt(urban) : "—"} />
        <Row k="Total" v={fmt(rural + urban)} />
        {live && live.net_migration < 0 && (
          <Row k="Migration" v={`↗ ${fmt(-live.net_migration)}/yr to the cities`} />
        )}
        {shares.length > 0 && (
          <>
            <ShareBar rows={shares.map(([name, s]) => ({ label: name, share: s }))} />
            <div style={{ opacity: 0.7, fontSize: 12, marginTop: 2 }}>
              {shares.map(([n, s]) => `${Math.round(s * 100)}% ${n}`).join(" · ")}
            </div>
          </>
        )}

        {/* ── Land & climate ── */}
        <Section title="Land & climate" />
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
          <Row k="Arid land" v={`${Math.round(p.arid_frac * 100)}%`} />
        )}
        <Row k="Fertility" v={p.mean_fertility.toFixed(2)} />
        {p.disease_mean !== undefined && p.disease_mean > 0.02 && (
          <Row k="Fever risk" v={`${Math.round(p.disease_mean * 100)}%`} />
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
              {koppenShares.map(([k, s]) => `${Math.round(s * 100)}% ${koppenName(k)}`).join(" · ")}
            </div>
          </>
        )}

        {/* ── Goods ── */}
        <Section title="Goods" />
        {p.goods.length === 0 ? (
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

        {/* ── Borders ── */}
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

        {/* ── Holdings ── */}
        <Section title="Holdings" />
        <ProvinceMiniMap
          province={p}
          raster={provinceRaster}
          settlements={miniSettlements}
          buildings={detail && detail.id === p.id ? detail.buildings : []}
        />
        {miniSettlements.length === 0 ? (
          <div style={{ opacity: 0.5, marginTop: 4 }}>frontier — no towns</div>
        ) : miniSettlements.slice().sort((a, b) => b.population - a.population).map((s, i) => (
          <div key={`${s.name}-${i}`} style={{ padding: "1px 0" }}>
            {s.seat ? "★ " : "· "}{s.name} <span style={{ opacity: 0.55 }}>{fmt(s.population)}</span>
            {s.seat ? <span style={{ opacity: 0.5 }}> (seat)</span> : null}
          </div>
        ))}

        {/* ── Flavour ── */}
        <div style={{ marginTop: 12, padding: "8px 10px", background: "rgba(0,0,0,0.25)",
          border: "1px solid #24382c", borderRadius: 6 }}>
          <div style={{ opacity: 0.8 }}>🌍 <b>Looks most like</b></div>
          <div style={{ marginBottom: 8 }}>{p.analog}</div>
          <div style={{ opacity: 0.8 }}>📜 <b>History</b></div>
          <div style={{ fontStyle: "italic", opacity: 0.9 }}>{provinceHistory(p, urban)}</div>
          {provinceFrontiers(p) && (
            <div style={{ opacity: 0.75, marginTop: 6 }}>{provinceFrontiers(p)}</div>
          )}
        </div>
      </div>
    </div>
  );
}

function Section({ title }: { title: string }) {
  return (
    <div style={{ marginTop: 12, marginBottom: 4, fontWeight: 600, opacity: 0.85,
      borderBottom: "1px solid #22342a", paddingBottom: 2 }}>{title}</div>
  );
}

function Row({ k, v }: { k: string; v: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", padding: "1px 0" }}>
      <span style={{ opacity: 0.65 }}>{k}</span><span>{v}</span>
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
  position: "absolute", top: 80, right: 90, width: 440, maxHeight: "80vh",
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
