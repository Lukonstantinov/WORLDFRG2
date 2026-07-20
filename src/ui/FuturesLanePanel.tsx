import { useUIStore } from "@state/uiStore";
import { useGoodsStore } from "@state/goodsStore";

/** Details for a clicked FUTURES contract lane (the futures map layer): which house
 *  or guild guarantees the supply, the good and monthly volume, the source →
 *  receiver cities, the term and the year it ends — plus a force-majeure flag when a
 *  plague quarantine has it suspended. */
export function FuturesLanePanel() {
  const lane = useUIStore((s) => s.selectedFuturesLane);
  const close = useUIStore((s) => s.setSelectedFuturesLane);
  const goodMeta = useGoodsStore((s) => s.meta);
  if (!lane) return null;
  const m = goodMeta(lane.good);
  const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));

  return (
    <div style={panel}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginBottom: 4 }}>
        <span style={{ width: 10, height: 10, borderRadius: 2, background: lane.color, alignSelf: "center" }} />
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13 }}>{lane.holder}</span>
        {lane.is_guild && <span style={{ fontSize: 8, color: "#7fd0c0" }}>GUILD</span>}
        <span style={{ flex: 1 }} />
        <span onClick={() => close(null)} style={{ color: "#7090b0", cursor: "pointer", fontSize: 16, lineHeight: 1 }} title="Close">×</span>
      </div>
      <div style={{ color: "#ffcf3f", fontSize: 12, marginBottom: 4 }}>
        📜 {lane.term}-year futures contract
        {lane.suspended && <span style={{ color: "#ff9a6a", fontSize: 9 }}> · ☣ suspended (quarantine)</span>}
      </div>
      {/* Relation summary in the owner's colour — e.g. "Venice → Genoa (silk — House Medici)" */}
      <div style={{ fontSize: 11, marginBottom: 5, lineHeight: 1.4 }}>
        <span style={{ color: "#cfe0f4" }}>{lane.a_name}</span>
        <span style={{ color: "#8aa0c0" }}> → </span>
        <span style={{ color: "#cfe0f4" }}>{lane.b_name}</span>
        <span style={{ color: "#8aa0c0" }}>{"  ("}{m.name} — </span>
        <span style={{ color: lane.color, fontWeight: 700 }}>{lane.holder}</span>
        <span style={{ color: "#8aa0c0" }}>{lane.is_guild ? ", guild)" : ")"}</span>
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, color: "#cfe0f4", marginBottom: 4 }}>
        <span style={{ color: "#9ab0c8" }}>{m.icon} {m.name}</span>
        <span style={{ color: "#7fd0a0" }}>{fmt(lane.qty)}/mo</span>
      </div>
      <div style={{ fontSize: 11, color: "#c0d0e0", marginBottom: 2 }}>
        <span style={{ color: "#cfe0f4" }}>{lane.a_name}</span>
        <span style={{ color: "#8aa0c0" }}> ▟ produce/warehouse</span>
      </div>
      <div style={{ fontSize: 11, color: "#c0d0e0" }}>
        → <span style={{ color: "#cfe0f4" }}>{lane.b_name}</span>
        <span style={{ color: "#8aa0c0" }}> ◯ receiver</span>
      </div>
      <div style={{ color: "#6a86a6", fontSize: 10, marginTop: 5 }}>
        expires year {lane.end_year}
      </div>
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", bottom: 40, left: 12, width: 250,
  background: "rgba(12,18,26,0.97)", border: "1px solid #3a3214", borderRadius: 8,
  padding: "9px 11px", zIndex: 115, boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
