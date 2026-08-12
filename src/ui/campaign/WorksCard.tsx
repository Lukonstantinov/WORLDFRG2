import { useEffect, useState } from "react";
import { campaignWorksCard } from "@bridge";
import type { WorksCardInfo } from "@types";
import { useGoodsStore } from "@state/goodsStore";
import { T, FZ, RADIUS } from "@ui/campaign/chronicleTheme";
import { Meter } from "@ui/kit";

function fmt(n: number): string {
  const a = Math.abs(n);
  if (a >= 10_000) return `${(n / 1000).toFixed(1)}k`;
  if (a >= 100) return Math.round(n).toString();
  return n.toFixed(1);
}

/** A tiny sparkline over up to 12 points, no library — a handful of line
 *  segments in an inline SVG. */
function Sparkline({ values, color }: { values: number[]; color: string }) {
  if (values.length < 2) return null;
  const max = Math.max(...values, 1e-6);
  const min = Math.min(...values, 0);
  const span = Math.max(max - min, 1e-6);
  const w = 100, h = 24;
  const pts = values.map((v, i) => {
    const x = (i / (values.length - 1)) * w;
    const y = h - ((v - min) / span) * h;
    return `${x},${y}`;
  }).join(" ");
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} style={{ display: "block" }}>
      <polyline points={pts} fill="none" stroke={color} strokeWidth={1.5} />
    </svg>
  );
}

/** ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.6 (D15/D16/§8.2) · one expandable
 *  works card: the GOOD's own icon leads (D16 — a manufactory shows cloth's
 *  icon, not a shared 🏭), the rank/yield line (D15), condition, the
 *  ownership bar (D1), and the twelve-month output/quality/price curves
 *  (§3). Fetches on mount and whenever `hub`/`tick` changes; renders nothing
 *  while loading or if the hub isn't (or is no longer) an estate. */
export function WorksCard({ hub, tick }: { hub: number; tick: number }) {
  const [card, setCard] = useState<WorksCardInfo | null>(null);
  const goodMeta = useGoodsStore((s) => s.meta);

  useEffect(() => {
    let alive = true;
    campaignWorksCard(hub).then((c) => { if (alive) setCard(c); }).catch(() => { if (alive) setCard(null); });
    return () => { alive = false; };
  }, [hub, tick]);

  if (!card) return <div style={{ fontSize: FZ.small, color: T.inkFaint, padding: "6px 2px" }}>Loading…</div>;

  const meta = goodMeta(card.good_name);
  const outputs = card.monthly.map((m) => m.output);
  const qualities = card.monthly.map((m) => m.quality);
  const prices = card.monthly.map((m) => m.price);
  const conditionColor = card.condition > 0.7 ? T.good : card.condition > 0.35 ? T.warn : T.bad;
  const conditionWord = card.damage <= 0.01 ? "sound" : card.damage < 0.4 ? "worn" : card.damage < 0.75 ? "damaged" : "ruined";

  return (
    <div style={{ border: `1px solid ${T.line}`, borderRadius: RADIUS.md, padding: 8, background: T.raised, marginTop: 4 }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
        <span style={{ fontSize: 18 }}>{meta.icon}</span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ color: T.gold, fontWeight: 700, fontSize: FZ.body }}>{card.name}</div>
          <div style={{ color: T.inkDim, fontSize: FZ.small }}>
            {card.kind_label.toLowerCase()} · tier {card.tier} · {card.good_name}
          </div>
        </div>
      </div>

      <div style={{ marginTop: 4, fontSize: FZ.small, color: T.inkMid }}>
        ⭐ {card.rank}{card.rank === 1 ? "st" : card.rank === 2 ? "nd" : card.rank === 3 ? "rd" : "th"} of {card.rank_of} ·
        {" "}yield {card.yield_index.toFixed(1)}× · <span style={{ color: T.gold, fontWeight: 700 }}>{card.yield_label.toUpperCase()}</span>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 4 }}>
        <span style={{ fontSize: FZ.small, color: conditionColor, width: 70 }}>{conditionWord}</span>
        <Meter value={card.condition} max={1} color={conditionColor} height={5} />
        <span style={{ fontSize: FZ.small, color: T.inkMid, minWidth: 60, textAlign: "right" }}>
          {fmt(card.monthly_output)}/mo {card.output_delta !== 0 && (
            <span style={{ color: card.output_delta > 0 ? T.goodInk : T.badInk }}>
              {card.output_delta > 0 ? "▲" : "▼"}
            </span>
          )}
        </span>
      </div>

      {outputs.length >= 2 && (
        <div style={{ display: "flex", gap: 10, marginTop: 6 }}>
          <div>
            <div style={{ fontSize: 8, color: T.inkFaint }}>output</div>
            <Sparkline values={outputs} color={T.accent} />
          </div>
          <div>
            <div style={{ fontSize: 8, color: T.inkFaint }}>quality</div>
            <Sparkline values={qualities} color={T.gold} />
          </div>
          <div>
            <div style={{ fontSize: 8, color: T.inkFaint }}>price</div>
            <Sparkline values={prices} color={T.goodInk} />
          </div>
        </div>
      )}

      {card.owners.length > 0 && (
        <div style={{ marginTop: 6 }}>
          <div style={{ fontSize: 8, color: T.inkFaint, marginBottom: 2 }}>OWNERSHIP</div>
          <div style={{ display: "flex", width: "100%", height: 8, borderRadius: 3, overflow: "hidden" }}>
            {card.owners.map((o, i) => (
              <div key={i} title={`${o.name} ${Math.round(o.frac * 100)}%`}
                style={{
                  width: `${o.frac * 100}%`,
                  background: o.holder_kind === 0
                    ? `repeating-linear-gradient(45deg, ${o.color}, ${o.color} 3px, transparent 3px, transparent 6px)`
                    : o.color,
                }} />
            ))}
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 3 }}>
            {card.owners.map((o, i) => (
              <span key={i} style={{ fontSize: 8, color: T.inkMid }}>
                <span style={{ color: o.color }}>■</span> {o.name} {Math.round(o.frac * 100)}%
                {o.instrument === 1 && o.term_years > 0 && (
                  <span style={{ color: T.inkFaint }}> ({o.term_years}yr tenancy)</span>
                )}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
