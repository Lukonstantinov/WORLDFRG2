import { useEffect, useMemo, useState } from "react";
import { useCampaignStore } from "@state/campaignStore";
import { useGoodsStore } from "@state/goodsStore";
import { campaignCityWarehouse } from "@bridge";
import type { CityWarehouseGood, CityWarehouseInfo } from "@types";
import { T, FZ, RADIUS } from "@ui/campaign/chronicleTheme";
import { Meter, Tabs } from "@ui/kit";

const BAND_TABS = [
  ["0", "Life"],
  ["1", "Daily"],
  ["2", "Luxury"],
] as const;

const GRADE_COLOR = ["#6a5a3a", "#7fa0c0", "#d8b24a"]; // coarse · common · fine

function fmt(n: number): string {
  const a = Math.abs(n);
  if (a >= 10_000) return `${(n / 1000).toFixed(1)}k`;
  return Math.round(n).toString();
}

function deltaLabel(d: number): { text: string; color: string } {
  if (Math.abs(d) < 0.5) return { text: "——", color: T.inkFaint };
  const up = d > 0;
  return { text: `${up ? "▲" : "▼"}${up ? "+" : ""}${fmt(d)}`, color: up ? T.goodInk : T.badInk };
}

/** ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.3 (D17/§8.1) · the city warehouse slot
 *  grid — one good per slot (icon · amount · month delta · grade strip), grouped
 *  into three band tabs (Life/Daily/Luxury). An EMPTY slot is information: a city
 *  filling eight of thirty-six slots is visibly poorer than one filling thirty.
 *  Embedded inline in HubPanel's own "Warehouse" tab, not a floating window of
 *  its own — it is a property of the settlement being inspected. */
export function CityWarehousePanel({ hub, tick }: { hub: number; tick: number }) {
  const [info, setInfo] = useState<CityWarehouseInfo | null>(null);
  const [band, setBand] = useState<0 | 1 | 2>(0);
  const [selected, setSelected] = useState<number | null>(null);
  const goodMeta = useGoodsStore((s) => s.meta);

  useEffect(() => {
    let alive = true;
    campaignCityWarehouse(hub).then((r) => { if (alive) setInfo(r); }).catch(() => { if (alive) setInfo(null); });
    return () => { alive = false; };
  }, [hub, tick]);

  const bandGoods = useMemo(() => {
    if (!info) return [] as CityWarehouseGood[];
    return info.goods.filter((g) => g.need_tier === band).sort((a, b) => b.amount - a.amount);
  }, [info, band]);

  useEffect(() => { setSelected(null); }, [band, hub]);

  if (!info) {
    return <div style={{ fontSize: FZ.body, color: T.inkDim, padding: "6px 2px" }}>No warehouse data yet.</div>;
  }

  const fillPct = Math.min(1, info.fill_frac) * 100;
  const overflowing = info.fill_frac > 1;
  const topSpoiled = [...info.goods].sort((a, b) => b.spoiled_month - a.spoiled_month).slice(0, 2)
    .filter((g) => g.spoiled_month > 0.5);
  const sel = selected != null ? info.goods.find((g) => g.good === selected) ?? null : null;

  // Pad to a full row of 6 so empty slots stay visibly empty, per D17.
  const slots: (CityWarehouseGood | null)[] = [...bandGoods];
  while (slots.length % 6 !== 0 || slots.length === 0) slots.push(null);
  if (slots.length > 36) slots.length = 36;

  return (
    <div style={{ fontSize: FZ.body }}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
        <div style={{ flex: 1 }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: FZ.small, color: T.inkMid }}>
            <span>fill {fmt(info.used)} / {fmt(info.capacity)}</span>
            <span style={{ color: overflowing ? T.bad : T.inkDim }}>{Math.round(fillPct)}%{overflowing ? " ⚠ overflowing" : ""}</span>
          </div>
          <Meter value={info.used} max={Math.max(info.capacity, 1)} color={overflowing ? T.bad : T.gold} height={5} style={{ marginTop: 2 }} />
        </div>
      </div>
      {info.spoiled_total_month > 0.5 && (
        <div style={{ fontSize: FZ.small, color: T.badInk, marginBottom: 4 }}>
          spoiled this month −{fmt(info.spoiled_total_month)}
          {topSpoiled.length > 0 && (
            <span style={{ color: T.inkFaint }}>
              {" "}({topSpoiled.map((g) => `${goodMeta(g.name).icon} ${g.name} −${fmt(g.spoiled_month)}`).join(" · ")})
            </span>
          )}
        </div>
      )}

      <Tabs tabs={BAND_TABS} active={String(band)} onSelect={(v) => setBand(Number(v) as 0 | 1 | 2)} style={{ marginBottom: 6 }} />

      {bandGoods.length === 0 ? (
        <div style={{ fontSize: FZ.small, color: T.inkFaint, padding: "10px 2px" }}>
          Nothing held in this band — the city's own poverty, made visible.
        </div>
      ) : (
        <div style={{ display: "grid", gridTemplateColumns: "repeat(6, 1fr)", gap: 3 }}>
          {slots.map((g, i) => {
            if (!g) {
              return <div key={`e${i}`} style={{ aspectRatio: "1", border: `1px dashed ${T.lineSoft}`, borderRadius: RADIUS.sm }} />;
            }
            const meta = goodMeta(g.name);
            const d = deltaLabel(g.delta_month);
            const total = Math.max(g.amount, 0.001);
            const on = selected === g.good;
            return (
              <div key={g.good} data-no-drag onClick={() => setSelected(on ? null : g.good)}
                title={g.name}
                style={{
                  cursor: "pointer", padding: "3px 2px", borderRadius: RADIUS.sm,
                  border: `1px solid ${on ? T.gold : T.line}`, background: on ? T.raised : T.card,
                  display: "flex", flexDirection: "column", alignItems: "center", gap: 1,
                }}>
                <div style={{ fontSize: 15, lineHeight: 1 }}>{meta.icon}</div>
                <div style={{ fontSize: 9, color: T.ink, fontWeight: 600 }}>{fmt(g.amount)}</div>
                <div style={{ fontSize: 8, color: d.color }}>{d.text}</div>
                <div style={{ display: "flex", width: "100%", height: 3, borderRadius: 2, overflow: "hidden" }}>
                  <div style={{ width: `${(g.coarse / total) * 100}%`, background: GRADE_COLOR[0] }} />
                  <div style={{ width: `${(g.common / total) * 100}%`, background: GRADE_COLOR[1] }} />
                  <div style={{ width: `${(g.fine / total) * 100}%`, background: GRADE_COLOR[2] }} />
                </div>
              </div>
            );
          })}
        </div>
      )}

      {sel && (
        <div style={{ marginTop: 8, borderTop: `1px solid ${T.line}`, paddingTop: 6 }}>
          <div style={{ color: T.gold, fontWeight: 700, marginBottom: 2 }}>
            {goodMeta(sel.name).icon} {sel.name}
          </div>
          <div style={{ fontSize: FZ.small, color: T.inkMid }}>
            {fmt(sel.amount)} units · cover {sel.cover_months.toFixed(1)} mo
            {sel.spoiled_month > 0.05 && <span style={{ color: T.badInk }}> · spoiled −{fmt(sel.spoiled_month)} this month</span>}
          </div>
          <div style={{ display: "flex", width: "100%", height: 8, borderRadius: 3, overflow: "hidden", marginTop: 4 }}>
            {(["coarse", "common", "fine"] as const).map((k, i) => {
              const v = sel[k];
              const total = Math.max(sel.amount, 0.001);
              return v > 0.5 ? (
                <div key={k} style={{ width: `${(v / total) * 100}%`, background: GRADE_COLOR[i] }}
                  title={`${k} ${fmt(v)}`} />
              ) : null;
            })}
          </div>
          <div style={{ display: "flex", gap: 8, fontSize: 9, color: T.inkFaint, marginTop: 2 }}>
            <span>coarse {fmt(sel.coarse)}</span>
            <span>common {fmt(sel.common)}</span>
            <span>fine {fmt(sel.fine)}</span>
          </div>
        </div>
      )}
    </div>
  );
}
