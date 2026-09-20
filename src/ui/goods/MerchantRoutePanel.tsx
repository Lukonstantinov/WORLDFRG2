import { useUIStore } from "@state/uiStore";
import { useGoodsStore } from "@state/goodsStore";
import type { MerchantRoute } from "@types";

/** Details for a clicked merchant route (the merchant map layer): which family or
 *  guild runs it, between which cities, the real distance/time/tariff it actually
 *  pays, and the goods it carries each way — the round-trip picture (out and back).
 *
 *  When the route's cheapest path composes through a coastal outlet (`relay_at`,
 *  the entrepôt/"Ostia" case — CLAUDE.md §8.5), the backend serves it as TWO
 *  `MerchantRoute` legs meeting at that port rather than one straight line.
 *  `MapCanvas` looks up the sibling leg at click time (`OverlayManager.
 *  siblingMerchantLeg`) so this panel can show the WHOLE journey — origin →
 *  break-of-bulk port → destination — as one story instead of just the half
 *  that happened to be clicked. */
export function MerchantRoutePanel() {
  const route = useUIStore((s) => s.selectedMerchantRoute);
  const sibling = useUIStore((s) => s.selectedMerchantRouteSibling);
  const close = useUIStore((s) => s.setSelectedMerchantRoute);
  const goodMeta = useGoodsStore((s) => s.meta);
  if (!route) return null;
  const icon = (id: string) => goodMeta(id).icon;
  const label = (id: string) => goodMeta(id).name;
  const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));
  const pct = (v: number) => `${(v * 100).toFixed(1)}%`;
  const modeIcon = (r: MerchantRoute) => (r.sea ? "🚢" : r.river ? "\u{1F6F6}" : "🐫");
  const modeLabel = (r: MerchantRoute) => (r.sea ? "by sea" : r.river ? "by river barge" : "overland by caravan");

  // Order the two legs origin-first regardless of which half was clicked
  // (relay_at 2 = relay sits at THIS leg's `b`, i.e. this leg comes first).
  const legs: MerchantRoute[] = sibling
    ? (route.relay_at === 2 ? [route, sibling] : [sibling, route])
    : [route];
  const relayName = sibling ? legs[0].b_name : null;
  const totalKm = legs.reduce((a, l) => a + (l.km ?? 0), 0);
  const totalDays = legs.reduce((a, l) => a + (l.days ?? 0), 0);
  const originName = legs[0].a_name, destName = legs[legs.length - 1].b_name;

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

  const LegStats = ({ l }: { l: MerchantRoute }) => (
    <div style={{ display: "flex", flexWrap: "wrap", gap: "2px 10px", fontSize: 9.5, color: "#8aa0c0", margin: "2px 0 4px" }}>
      <span>{modeIcon(l)} {modeLabel(l)}</span>
      {(l.km ?? 0) > 0 && <span>{Math.round(l.km!).toLocaleString()} km</span>}
      {(l.days ?? 0) > 0 && <span>{l.days!.toFixed(1)}d</span>}
      {l.risk != null && <span style={{ color: l.risk > 0.15 ? "#e08a5a" : l.risk > 0.06 ? "#d8c060" : "#7fd0a0" }}>
        ⚠ {pct(l.risk)} loss risk
      </span>}
      {(l.tariff_export ?? 0) > 0 && <span>export toll {pct(l.tariff_export!)} at {l.a_name}</span>}
      {(l.tariff_import ?? 0) > 0 && <span>import toll {pct(l.tariff_import!)} at {l.b_name}</span>}
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

      {/* The whole journey, origin to destination, naming the break-of-bulk port
          if there is one — the thing the map's teal ring marks but never names. */}
      <div style={{ color: "#8aa0c0", fontSize: 10, marginBottom: 2 }}>
        {originName}
        {relayName ? <> <span style={{ color: "#2fd1c9" }}>⚓ {relayName}</span> </> : " ⇄ "}
        {relayName && destName}
        <span style={{ color: "#6a86a6" }}> · volume {fmt(route.volume)}</span>
      </div>
      {relayName && (
        <div style={{ color: "#2fd1c9", fontSize: 9, fontStyle: "italic", marginBottom: 4 }}>
          breaks bulk at {relayName} — cargo lands and re-embarks under new terms, not a straight-through voyage
        </div>
      )}
      <div style={{ display: "flex", gap: 10, fontSize: 9.5, color: "#c0d0e0", marginBottom: 6 }}>
        <span>total {Math.round(totalKm).toLocaleString()} km</span>
        <span>{totalDays.toFixed(1)} days</span>
      </div>

      {/* Per-leg breakdown — real distance, time, risk and the tariff each end
          actually charges. A relayed route shows both legs in order; a direct
          route shows its one leg the same way. */}
      {legs.map((l, i) => <LegStats key={i} l={l} />)}

      <Leg title="Outbound" from={originName} to={destName} goods={route.out_goods} />
      <Leg title="Return" from={destName} to={originName} goods={route.ret_goods} />
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", bottom: 40, left: 12, width: 280,
  background: "rgba(12,18,26,0.97)", border: "1px solid #24364e", borderRadius: 8,
  padding: "9px 11px", zIndex: 115, boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
