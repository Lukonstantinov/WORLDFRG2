import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useGoodsStore } from "../state/goodsStore";

/** Hub inspector (Phase 3): click a hub → its traded goods; click a received
 *  good → its supply-chain road with the price at each hop. */
export function HubPanel() {
  const selectedHub = useUIStore((s) => s.selectedHub);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const selectedChain = useUIStore((s) => s.selectedChain);
  const setSelectedChain = useUIStore((s) => s.setSelectedChain);
  const economy = useWorldStore((s) => s.economy);
  const goodMeta = useGoodsStore((s) => s.meta);

  if (selectedHub === null || !economy) return null;
  const hub = economy.hubs.find((h) => h.id === selectedHub);
  if (!hub) return null;

  const iconFor = (id: string) => goodMeta(id).icon;
  const labelFor = (id: string) => goodMeta(id).name;
  const hubName = (id: number) => economy.hubs.find((h) => h.id === id)?.name ?? `Hub ${id}`;
  const chain = selectedChain !== null ? economy.chains.find((c) => c.id === selectedChain) ?? null : null;

  return (
    <div style={panel}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
        <div>
          <div style={{ color: "#e8d8b0", fontSize: 15, fontWeight: 700 }}>{hub.name}</div>
          <div style={{ color: "#8aa0c0", fontSize: 10 }}>
            <span style={{ color: "#ffd24a" }}>{"★".repeat(Math.max(1, Math.min(5, hub.stars)))}</span>
            {"  "}wealth {Math.round(hub.wealth * 100)}% · pop {hub.population.toLocaleString()}
          </div>
        </div>
        <span onClick={() => setSelectedHub(null)}
          style={{ color: "#7090b0", cursor: "pointer", fontSize: 18, lineHeight: 1 }} title="Close">×</span>
      </div>

      {/* Produced */}
      <div style={sectionHdr}>Produces</div>
      {hub.produces.length === 0 && <div style={emptyTxt}>nothing of note</div>}
      {hub.produces.slice(0, 12).map((p) => (
        <div key={`p${p.good}`} style={row}>
          <span style={{ minWidth: 16 }}>{iconFor(p.good_name)}</span>
          <span style={{ flex: 1, color: "#c0d0e0" }}>
            {labelFor(p.good_name)}
            {p.flavor && <span style={{ color: "#8a7a5a", fontSize: 9, fontStyle: "italic" }}> · {p.flavor}</span>}
          </span>
          <span style={{ color: "#9ab0c8", fontSize: 9 }}>{p.grade}</span>
          <span style={{ color: "#e0c060", minWidth: 34, textAlign: "right" }}>{p.price.toFixed(1)}×</span>
        </div>
      ))}

      {/* Received */}
      <div style={{ ...sectionHdr, marginTop: 8 }}>Receives (click to trace the road)</div>
      {hub.receives.length === 0 && <div style={emptyTxt}>self-sufficient — no imports</div>}
      {hub.receives.slice(0, 16).map((r) => {
        const active = selectedChain === r.chain;
        return (
          <div key={`r${r.chain}-${r.good}`} onClick={() => setSelectedChain(active ? null : r.chain)}
            style={{ ...row, cursor: "pointer", background: active ? "#1a2c40" : "transparent", borderRadius: 3 }}>
            <span style={{ minWidth: 16 }}>{iconFor(r.good_name)}</span>
            <span style={{ flex: 1, color: active ? "#e8d8b0" : "#c0d0e0" }}>{labelFor(r.good_name)}</span>
            <span style={{ color: "#7a90a8", fontSize: 9 }}>from {hubName(r.from_hub)}</span>
            <span style={{ color: "#ff9a6a", minWidth: 34, textAlign: "right" }}>{r.price.toFixed(1)}×</span>
          </div>
        );
      })}

      {/* Selected chain breakdown */}
      {chain && (
        <div style={{ marginTop: 8, padding: "6px 8px", background: "#0b1622", borderRadius: 5, border: "1px solid #1e3550" }}>
          <div style={{ color: "#9ab0c8", fontSize: 10, marginBottom: 4 }}>
            {iconFor(chain.good_name)} {labelFor(chain.good_name)} — price along the road
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 4, fontSize: 11 }}>
            {chain.stops.map((s, i) => (
              <span key={i} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                {i > 0 && <span style={{ color: "#5a7090" }}>→</span>}
                <span style={{
                  color: i === 0 ? "#80dc8c" : i === chain.stops.length - 1 ? "#ff9a6a" : "#e0c060",
                }}>
                  {hubName(s.hub)} <b>{s.price.toFixed(1)}×</b>
                </span>
              </span>
            ))}
          </div>
          <div style={{ color: "#506680", fontSize: 9, marginTop: 4 }}>
            Green = origin (1× base), orange = this hub. Price rises with distance,
            scarcity &amp; quality. The road is highlighted on the map.
          </div>
        </div>
      )}
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 12, right: 12, width: 300, maxHeight: "82%", overflowY: "auto",
  background: "rgba(12,18,26,0.96)", border: "1px solid #24364e", borderRadius: 8,
  padding: "10px 12px", zIndex: 110, boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
const sectionHdr: React.CSSProperties = {
  color: "#6a86a6", fontSize: 10, fontWeight: 700, textTransform: "uppercase",
  letterSpacing: 0.5, borderBottom: "1px solid #1e2e42", paddingBottom: 2, marginBottom: 3,
};
const row: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 5, fontSize: 11, padding: "2px 3px",
};
const emptyTxt: React.CSSProperties = { color: "#506680", fontSize: 10, fontStyle: "italic", padding: "2px 3px" };
