import { useEffect, useRef } from "react";
import { useGoodsStore } from "@state/goodsStore";
import type { HubDetail } from "@types";
import { sceneFromDetail, renderSettlement, fmtPop } from "./settlementArt";

/** The City / Estate tab: an isometric building schematic (climate ground +
 *  latitude vegetation + sea for coastal places) with each building's owner and
 *  tier, plus the settlement's stats. Drawn once per opened settlement from the
 *  live HubDetail — off the simulation hot path. */
export function SettlementScene({ detail }: { detail: HubDetail }) {
  const goodMeta = useGoodsStore((s) => s.meta);
  const ref = useRef<SVGSVGElement>(null);

  useEffect(() => {
    if (!ref.current) return;
    const sc = sceneFromDetail(detail, (g) => goodMeta(g).icon || "📦");
    renderSettlement(ref.current, sc, 360);
  }, [detail, goodMeta]);

  const wealth = (detail.grain_wealth ?? 0) + (detail.trade_wealth ?? 0);
  const isEstate = detail.is_estate;
  return (
    <div>
      <svg ref={ref} style={{ width: "100%", height: "auto", display: "block", borderRadius: 8 }} />
      {!isEstate && (
        <>
          <div style={chipRow}>
            <span style={chip}>pop <b style={chipB}>{fmtPop(detail.population)}</b></span>
            <span style={chip}>wealth <b style={chipB}>{wealth.toFixed(0)}</b></span>
            {detail.sold != null && <span style={chip}>sold <b style={chipB}>{Math.round(detail.sold)}</b></span>}
            {detail.bought != null && <span style={chip}>bought <b style={chipB}>{Math.round(detail.bought)}</b></span>}
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 10, marginTop: 6 }}>
            <Bar label="Mood" v={detail.mood} color="#7fb069" />
            <Bar label="Food" v={detail.sent_food} color="#e0b250" />
            <Bar label="Prosperity" v={detail.sent_prosperity} color="#5fa8d0" />
            <Bar label="Stability" v={detail.sent_stability} color="#9a8fd0" />
          </div>
        </>
      )}
      {isEstate && (
        <div style={chipRow}>
          {detail.estate_good && <span style={chip}>produces <b style={chipB}>{detail.estate_good}</b></span>}
          {detail.estate_owner && <span style={chip}>owner <b style={chipB}>{detail.estate_owner}</b></span>}
          <span style={chip}>pop <b style={chipB}>{fmtPop(detail.population)}</b></span>
        </div>
      )}
    </div>
  );
}

function Bar({ label, v, color }: { label: string; v: number; color: string }) {
  return (
    <span style={{ fontSize: 10.5, color: "#8da0b8" }}>
      {label}
      <span style={{ display: "inline-block", width: 54, height: 6, background: "#1b2230", borderRadius: 3, verticalAlign: "middle", marginLeft: 4, overflow: "hidden" }}>
        <span style={{ display: "block", height: "100%", width: `${Math.round(Math.max(0, Math.min(1, v || 0)) * 100)}%`, background: color }} />
      </span>
    </span>
  );
}

const chipRow: React.CSSProperties = { display: "flex", flexWrap: "wrap", gap: 6, marginTop: 8 };
const chip: React.CSSProperties = { background: "#0e141b", border: "1px solid #232c37", borderRadius: 5, padding: "3px 7px", fontSize: 11, color: "#b9c8da" };
const chipB: React.CSSProperties = { color: "#e7cf90", fontWeight: 700 };
