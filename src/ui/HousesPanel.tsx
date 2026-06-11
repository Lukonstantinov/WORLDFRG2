import { useCampaignStore } from "../state/campaignStore";
import { useUIStore } from "../state/uiStore";
import { CoatOfArms } from "./CoatOfArms";

/** Merchant Houses browser — every trading family, its coat of arms, head of
 *  family, home city, wealth, the trades it controls (monopolies) and rivals.
 *  Active houses first; ruined ones greyed at the bottom. */
export function HousesPanel() {
  const open = useUIStore((s) => s.showHouses);
  const houses = useCampaignStore((s) => s.houses);
  const close = () => useUIStore.getState().setShowHouses(false);
  if (!open) return null;

  const active = houses.filter((h) => !h.defunct);
  const gone = houses.filter((h) => h.defunct);
  const maxWealth = Math.max(1, ...active.map((h) => h.wealth));

  return (
    <div style={panel}>
      <div style={header}>
        <span>⚜️ Merchant Houses ({active.length})</span>
        <span style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>
      <div style={{ overflowY: "auto", padding: "4px 8px 10px" }}>
        {houses.length === 0 && (
          <div style={empty}>Begin the campaign (Step 11) — trading families rise as goods start to move.</div>
        )}
        {active.map((h, i) => (
          <div key={h.name + i} style={card}>
            <CoatOfArms name={h.name} size={30} />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12 }}>{h.name}</span>
                <span style={{ color: "#6a86a6", fontSize: 9 }}>· {h.home_name}</span>
                <span style={{ flex: 1 }} />
                {h.political_power > 0.5 && <span title="A leading political power" style={{ fontSize: 10 }}>👑</span>}
              </div>
              <div style={{ color: "#9ab0c8", fontSize: 10 }}>
                {h.head_name} · gen. {h.generation} · led {h.head_age}y
              </div>
              {/* Trades the house controls */}
              <div style={{ color: "#cdbb88", fontSize: 10, marginTop: 1 }}>
                {h.specialties.length > 0 && <>Trades: {h.specialties.join(", ")}</>}
              </div>
              {h.monopolies.length > 0 && (
                <div style={{ color: "#e0b060", fontSize: 10 }}>
                  {h.monopolies.map(([g, s]) => `${g} ${Math.round(s * 100)}%`).join(" · ")} of the trade
                </div>
              )}
              {h.rivals.length > 0 && (
                <div style={{ color: "#c98", fontSize: 9 }}>⚔ rivals: {h.rivals.slice(0, 3).join(", ")}</div>
              )}
              {/* Wealth bar */}
              <div style={{ display: "flex", alignItems: "center", gap: 5, marginTop: 2 }}>
                <div style={{ flex: 1, height: 4, background: "#0a1018", borderRadius: 3, overflow: "hidden" }}>
                  <div style={{ width: `${(h.wealth / maxWealth) * 100}%`, height: "100%", background: "#c9a227" }} />
                </div>
                <span style={{ color: "#c9a227", fontSize: 9, minWidth: 40, textAlign: "right" }}>
                  {h.wealth >= 1000 ? `${(h.wealth / 1000).toFixed(1)}k` : h.wealth.toFixed(0)}
                </span>
              </div>
            </div>
          </div>
        ))}
        {gone.length > 0 && (
          <>
            <div style={{ color: "#5a6a7e", fontSize: 9, margin: "8px 0 2px", textTransform: "uppercase" }}>
              Fallen houses ({gone.length})
            </div>
            {gone.map((h, i) => (
              <div key={"d" + i} style={{ ...card, opacity: 0.45 }}>
                <CoatOfArms name={h.name} size={22} />
                <div style={{ flex: 1 }}>
                  <span style={{ color: "#9aa6b4", fontSize: 11, textDecoration: "line-through" }}>{h.name}</span>
                  <span style={{ color: "#566", fontSize: 9 }}> · once of {h.home_name}</span>
                </div>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 320, maxHeight: "78vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 40,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#cfe0f4", fontWeight: 700, fontSize: 12,
};
const card: React.CSSProperties = {
  display: "flex", gap: 8, alignItems: "flex-start", padding: "6px 4px",
  borderBottom: "1px solid #131e2a", cursor: "default",
};
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "10px 4px" };
