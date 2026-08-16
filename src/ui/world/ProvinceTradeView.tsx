import { useEffect, useMemo, useState } from "react";
import { campaignProvinceTrade } from "@bridge";
import { houseColor } from "@ui/heraldry/CoatOfArms";
import { GOOD_DEFS } from "@goods";
import type { ProvinceTrade } from "@types";

/** 💱 Province Trade — the circular-diagram view of a province's commerce, shown in
 *  the Province Inspector's Trade tab.
 *
 *  Three questions, answered from state the campaign already holds (no new sim math):
 *   1. WHO commands the trade here — by merchant house / guild, and by city
 *      (donuts over `House.trade_at` attributed to the province's cities).
 *   2. WHAT crosses the border — per-good exports (out) and imports (in), the exact
 *      per-good tonnage `accrue_flow` accumulates when a shipment crosses a province
 *      boundary.
 *   3. WHO could crown themselves — the house past the ≥20% trade-share threshold
 *      that makes it eligible to proclaim a realm here.
 *
 *  Everything degrades to a quiet "no trade recorded" on a fresh campaign or a
 *  province where nothing has moved yet. */

const CIVIC = "#c7a24a";   // a guild slice — civic gold, distinct from any house hue
const OTHERS = "#5b6b7d";  // the long-tail aggregate

/** Deterministic fallback hue for a city name (cities have no heraldic colour). */
function cityColor(s: string): string {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); }
  return `hsl(${Math.abs(h) % 360} 45% 56%)`;
}

type Slice = { name: string; value: number; color: string; sub?: string; ring?: boolean };

/** A generic donut + legend, the same arc math the culture donut uses. `ring`
 *  draws a highlight ring around a slice (the realm-eligible controller). */
function Donut({ slices, size = 104 }: { slices: Slice[]; size?: number }) {
  const total = slices.reduce((a, s) => a + Math.max(0, s.value), 0);
  const r = size / 2, ir = r * 0.58, cx = r, cy = r;
  let ang = -Math.PI / 2;
  const arcs = slices.filter((s) => s.value > 0).map((s) => {
    const frac = total > 0 ? s.value / total : 0;
    const a0 = ang, a1 = ang + frac * Math.PI * 2; ang = a1;
    const large = a1 - a0 > Math.PI ? 1 : 0;
    const pt = (rr: number, a: number) => [cx + rr * Math.cos(a), cy + rr * Math.sin(a)];
    const [x0, y0] = pt(r, a0), [x1, y1] = pt(r, a1), [x2, y2] = pt(ir, a1), [x3, y3] = pt(ir, a0);
    const d = `M${x0} ${y0} A${r} ${r} 0 ${large} 1 ${x1} ${y1} L${x2} ${y2} A${ir} ${ir} 0 ${large} 0 ${x3} ${y3} Z`;
    return { ...s, d, pct: Math.round(frac * 100) };
  });
  if (arcs.length === 0) return <div style={{ opacity: 0.5, fontSize: 12 }}>no trade recorded</div>;
  return (
    <div style={{ display: "flex", gap: 10, alignItems: "center", margin: "2px 0 8px" }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} style={{ flex: "0 0 auto" }}>
        {arcs.length <= 1 ? (
          <>
            <circle cx={cx} cy={cy} r={r} fill={arcs[0].color} stroke={arcs[0].ring ? "#ffe08a" : "none"} strokeWidth={arcs[0].ring ? 2.5 : 0} />
            <circle cx={cx} cy={cy} r={ir} fill="#0a1018" />
          </>
        ) : arcs.map((a, i) => (
          <path key={i} d={a.d} fill={a.color} stroke={a.ring ? "#ffe08a" : "#0a1018"} strokeWidth={a.ring ? 2.5 : 1} />
        ))}
      </svg>
      <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 2 }}>
        {arcs.map((a, i) => (
          <div key={i} style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 10 }}>
            <span style={{ width: 9, height: 9, borderRadius: 2, background: a.color, flex: "0 0 auto", boxShadow: a.ring ? "0 0 0 1.5px #ffe08a" : "none" }} />
            <span style={{ flex: 1, color: "#cfe2f6", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {a.name}{a.sub ? <span style={{ opacity: 0.55 }}> · {a.sub}</span> : null}
            </span>
            <span style={{ color: "#9ab0c8" }}>{a.pct}%</span>
          </div>
        ))}
      </div>
    </div>
  );
}

const fmt = (v: number) => v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0);

export function ProvinceTradeView({ provinceId, reload }: { provinceId: number; reload: number }) {
  const [trade, setTrade] = useState<ProvinceTrade | null>(null);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let stale = false;
    setLoading(true);
    campaignProvinceTrade(provinceId)
      .then((t) => { if (!stale) { setTrade(t); setLoading(false); } })
      .catch(() => { if (!stale) { setTrade(null); setLoading(false); } });
    return () => { stale = true; };
  }, [provinceId, reload]);

  const holderSlices = useMemo<Slice[]>(() => (trade?.by_holder ?? []).map((h) => ({
    name: h.name,
    value: h.volume,
    color: h.kind === 1 ? CIVIC : h.kind === 2 ? OTHERS : houseColor(h.name),
    sub: h.kind === 1 ? "guild" : h.eligible ? "controller" : undefined,
    ring: h.eligible,
  })), [trade]);

  const citySlices = useMemo<Slice[]>(() => (trade?.by_city ?? []).map((c) => ({
    name: c.name, value: c.volume, color: cityColor(c.name),
  })), [trade]);

  if (loading) return <div style={{ opacity: 0.5, padding: "6px 0" }}>reading the ledgers…</div>;
  if (!trade || (trade.total <= 0 && trade.export_total <= 0 && trade.import_total <= 0)) {
    return (
      <div style={{ opacity: 0.6, fontSize: 12, padding: "6px 0" }}>
        No trade has been recorded in this province yet — advance the campaign a year, or
        this is frontier with no merchant houses working it.
      </div>
    );
  }

  const goodRow = (g: { good: number; amount: number }, denom: number) => {
    const def = GOOD_DEFS[g.good];
    return (
      <div key={g.good} style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, padding: "1px 0" }}>
        <span style={{ flex: "0 0 auto" }}>{def?.emoji ?? "•"}</span>
        <span style={{ flex: 1, minWidth: 0, color: "#cfe2f6", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {def?.label ?? `good ${g.good}`}
        </span>
        <span style={{ flex: "0 0 auto", width: 70, height: 5, background: "#182534", borderRadius: 3, overflow: "hidden" }}>
          <span style={{ display: "block", height: "100%", width: `${denom > 0 ? Math.round((g.amount / denom) * 100) : 0}%`, background: def?.color ?? "#6d88a8" }} />
        </span>
        <span style={{ flex: "0 0 auto", width: 44, textAlign: "right", color: "#9ab0c8" }}>{fmt(g.amount)}</span>
      </div>
    );
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {trade.controller_house >= 0 && (
        <div style={{ background: "rgba(255,224,138,0.10)", border: "1px solid rgba(255,224,138,0.35)", borderRadius: 6, padding: "5px 8px", fontSize: 11.5, color: "#f0dfae" }}>
          👑 <b>{trade.controller_name}</b> commands {Math.round(trade.controller_share * 100)}% of this
          province's trade — eligible to proclaim a realm here (needs ≥ {Math.round(trade.control_threshold * 100)}%).
        </div>
      )}

      <div style={{ opacity: 0.7, fontSize: 12, marginTop: 4 }}>Who commands the trade</div>
      <Donut slices={holderSlices} />

      <div style={{ opacity: 0.7, fontSize: 12 }}>Trade by city</div>
      <Donut slices={citySlices} />

      <div style={{ display: "flex", gap: 12 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ opacity: 0.7, fontSize: 12 }}>
            Exported <span style={{ opacity: 0.55 }}>({fmt(trade.export_total)})</span>
          </div>
          {trade.exports.length === 0 ? <div style={{ opacity: 0.5, fontSize: 11 }}>nothing leaves</div>
            : trade.exports.slice(0, 8).map((g) => goodRow(g, trade.exports[0].amount))}
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ opacity: 0.7, fontSize: 12 }}>
            Imported <span style={{ opacity: 0.55 }}>({fmt(trade.import_total)})</span>
          </div>
          {trade.imports.length === 0 ? <div style={{ opacity: 0.5, fontSize: 11 }}>nothing enters</div>
            : trade.imports.slice(0, 8).map((g) => goodRow(g, trade.imports[0].amount))}
        </div>
      </div>
      <div style={{ opacity: 0.45, fontSize: 10, marginTop: 2 }}>
        Shares are of organized merchant-house trade at this province's cities. Exports/imports
        are last year's goods crossing the province boundary.
      </div>
    </div>
  );
}
