import { useUIStore } from "@state/uiStore";
import { useGoodsStore } from "@state/goodsStore";

/** Details for a clicked merchant route (the merchant map layer): which family or
 *  guild runs it, between which cities, and the goods it carries each way — the
 *  round-trip picture (out and back). */
export function MerchantRoutePanel() {
  const route = useUIStore((s) => s.selectedMerchantRoute);
  const close = useUIStore((s) => s.setSelectedMerchantRoute);
  const goodMeta = useGoodsStore((s) => s.meta);
  if (!route) return null;
  const icon = (id: string) => goodMeta(id).icon;
  const label = (id: string) => goodMeta(id).name;
  const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));

  const Leg = ({ title, from, to, goods }:
    { title: string; from: string; to: string; goods: [string, number][] }) => (
    <div style={{ marginBottom: 6 }}>
      <div style={{ color: "#9ab0c8", fontSize: 10 }}>
        {title} <span style={{ color: "#cfe0f4" }}>{from}</span> → <span style={{ color: "#cfe0f4" }}>{to}</span>
      </div>
      {goods.length === 0 && <div style={{ color: "#56708e", fontSize: 9, fontStyle: "italic" }}>nothing this way yet</div>}
      {goods.slice(0, 8).map(([g, v]) => (
        <div key={g} style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#c0d0e0" }}>
          <span>{icon(g)} {label(g)}</span>
          <span style={{ color: "#7fd0a0" }}>{fmt(v)}</span>
        </div>
      ))}
    </div>
  );

  return (
    <div style={panel}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginBottom: 4 }}>
        <span style={{ width: 10, height: 10, borderRadius: 2, background: route.color, alignSelf: "center" }} />
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13 }}>{route.holder}</span>
        {route.is_guild && <span style={{ fontSize: 8, color: "#7fd0c0" }}>GUILD</span>}
        <span style={{ flex: 1 }} />
        <span onClick={() => close(null)} style={{ color: "#7090b0", cursor: "pointer", fontSize: 16, lineHeight: 1 }} title="Close">×</span>
      </div>
      <div style={{ color: "#8aa0c0", fontSize: 10, marginBottom: 6 }}>
        {route.sea ? "🚢 sea route" : "🐫 overland route"} · {route.a_name} ⇄ {route.b_name}
        <span style={{ color: "#6a86a6" }}> · volume {fmt(route.volume)}</span>
      </div>
      <Leg title="Outbound" from={route.a_name} to={route.b_name} goods={route.out_goods} />
      <Leg title="Return" from={route.b_name} to={route.a_name} goods={route.ret_goods} />
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", bottom: 40, left: 12, width: 250,
  background: "rgba(12,18,26,0.97)", border: "1px solid #24364e", borderRadius: 8,
  padding: "9px 11px", zIndex: 115, boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
