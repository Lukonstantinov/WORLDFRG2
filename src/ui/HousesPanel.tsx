import { useState } from "react";
import { useCampaignStore } from "../state/campaignStore";
import { useUIStore } from "../state/uiStore";
import { CoatOfArms } from "./CoatOfArms";
import { GOOD_DEFS } from "../goods";
import { campaignGetHouseHistory } from "../bridge/tauri";
import type { HouseHistory, CampaignDiagnostics } from "../types";

const GOOD_ICON = new Map(GOOD_DEFS.map((g) => [g.name, g.emoji]));
const goodIcon = (name: string) => GOOD_ICON.get(name) ?? "\u{1F4E6}"; // 📦 fallback

/** "Is trade actually moving?" — a compact health strip above the houses list.
 *  Answers the core merchant-house question: are shipments flowing, how many are
 *  financed by houses vs. local guilds, are voyages being lost, and how much of
 *  the world do houses actually control. */
function TradeDiagnostics({ diag }: { diag: CampaignDiagnostics }) {
  const fleet = diag.fleet_sea + diag.fleet_river + diag.fleet_caravan;
  const moving = diag.shipments_last > 0;
  const housePct = diag.shipments_last > 0
    ? Math.round((diag.by_house / diag.shipments_last) * 100) : 0;
  const stat = (label: string, value: string, color = "#cfe0f4", title?: string) => (
    <div style={diagCell} title={title}>
      <div style={{ color, fontWeight: 700, fontSize: 12 }}>{value}</div>
      <div style={{ color: "#6a86a6", fontSize: 9 }}>{label}</div>
    </div>
  );
  return (
    <div style={diagBar}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
        <span style={{ width: 7, height: 7, borderRadius: "50%", background: moving ? "#5fd08a" : "#d06a5f" }} />
        <span style={{ color: moving ? "#9fe0b8" : "#e0a09a", fontSize: 10, fontWeight: 600 }}>
          {moving ? "Trade is flowing" : "No shipments last advance"}
        </span>
        <span style={{ flex: 1 }} />
        <span style={{ color: "#6a86a6", fontSize: 9 }}>year {diag.year}</span>
      </div>
      <div style={{ display: "flex", gap: 4 }}>
        {stat("shipped", String(diag.shipments_last), "#cfe0f4", "Shipments dispatched last advance")}
        {stat("by houses", `${housePct}%`, housePct > 0 ? "#e0c060" : "#7a90a8", `${diag.by_house} financed by a house, ${diag.by_guild} by local guilds`)}
        {stat("lost", String(diag.lost_last), diag.lost_last > 0 ? "#e0a09a" : "#7a90a8", "Voyages lost to storm/ambush last advance")}
        {stat("in transit", String(diag.in_transit), "#9ab0c8", "Shipments currently in flight")}
      </div>
      <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
        {stat("controls", String(diag.controlled_settlements), diag.controlled_settlements > 0 ? "#9fe0b8" : "#d06a5f", "Settlements a house controls (>=50% of trade throughput)")}
        {stat("ships", String(fleet), "#9ab0c8", `${diag.fleet_sea} sea · ${diag.fleet_river} river · ${diag.fleet_caravan} caravan`)}
        {stat("houses", `${diag.houses_active}`, "#cfe0f4", `${diag.houses_active} active · ${diag.houses_defunct} ruined`)}
        {stat("wealth", diag.total_house_wealth >= 100 ? `${Math.round(diag.total_house_wealth)}` : diag.total_house_wealth.toFixed(1), "#e0c060", "Combined wealth of all active houses")}
      </div>
    </div>
  );
}

/** Merchant Houses browser — every trading family, its coat of arms, head of
 *  family, home city, wealth, the trades it controls (monopolies) and rivals.
 *  Active houses first; ruined ones greyed at the bottom. */
export function HousesPanel() {
  const open = useUIStore((s) => s.showHouses);
  const houses = useCampaignStore((s) => s.houses);
  const diag = useCampaignStore((s) => s.diagnostics);
  const [history, setHistory] = useState<HouseHistory | null>(null);
  const close = () => useUIStore.getState().setShowHouses(false);
  const openTimeline = (name: string) => {
    campaignGetHouseHistory(name).then((h) => setHistory(h)).catch(() => setHistory(null));
  };
  if (!open) return null;

  const active = houses.filter((h) => !h.defunct);
  const gone = houses.filter((h) => h.defunct);
  const maxWealth = Math.max(1, ...active.map((h) => h.wealth));

  return (
    <div style={panel}>
      {history && <HouseTimeline history={history} onClose={() => setHistory(null)} />}
      <div style={header}>
        <span>⚜️ Merchant Houses ({active.length})</span>
        <span style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>
      {diag && <TradeDiagnostics diag={diag} />}
      <div style={{ overflowY: "auto", padding: "4px 8px 10px" }}>
        {houses.length === 0 && (
          <div style={empty}>Begin the campaign (Step 11) — trading families rise as goods start to move.</div>
        )}
        {active.map((h, i) => (
          <div key={h.name + i} style={{ ...card, cursor: "pointer" }} onClick={() => openTimeline(h.name)} title="View this family's timeline">
            <CoatOfArms name={h.name} size={30} />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                {/* Colour chip = this house's unique map colour */}
                <span style={{ width: 9, height: 9, borderRadius: 2, background: h.color ?? "#888", flex: "0 0 auto", alignSelf: "center" }} />
                <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12 }}>{h.name}</span>
                <span style={{ color: "#6a86a6", fontSize: 9 }}>· {h.home_name}</span>
                <span style={{ flex: 1 }} />
                {h.dominant && <span title="Controls its seat city (>=50% of its trade)" style={{ fontSize: 10 }}>⚖</span>}
                {h.political_power > 0.5 && <span title="A leading political power" style={{ fontSize: 10 }}>👑</span>}
              </div>
              <div style={{ color: "#9ab0c8", fontSize: 10 }}>
                {h.head_name} · gen. {h.generation} · led {h.head_age}y
              </div>
              {/* Trades the house controls — with good icons */}
              {h.specialties.length > 0 && (
                <div style={{ color: "#cdbb88", fontSize: 10, marginTop: 1, display: "flex", flexWrap: "wrap", gap: 4, alignItems: "center" }}>
                  {h.specialties.map((g) => (
                    <span key={g} title={g} style={{ display: "inline-flex", alignItems: "center", gap: 2 }}>
                      <span style={{ fontSize: 11 }}>{goodIcon(g)}</span>{g}
                    </span>
                  ))}
                </div>
              )}
              {h.monopolies.length > 0 && (
                <div style={{ color: "#e0b060", fontSize: 10 }}>
                  {h.monopolies.map(([g, s]) => `${goodIcon(g)} ${g} ${Math.round(s * 100)}%`).join(" · ")} of the trade
                </div>
              )}
              {/* Cities this house trades with (seat first) */}
              {h.cities && h.cities.length > 0 && (
                <div style={{ color: "#88a8c8", fontSize: 9, marginTop: 1 }}>
                  🏙 {h.cities.slice(0, 6).join(", ")}{h.cities.length > 6 ? ` +${h.cities.length - 6}` : ""}
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
              <div key={"d" + i} style={{ ...card, opacity: 0.55, cursor: "pointer" }} onClick={() => openTimeline(h.name)} title="View this family's timeline">
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
const diagBar: React.CSSProperties = {
  padding: "6px 10px", borderBottom: "1px solid #1a2a3e", background: "#0a1119",
};
const diagCell: React.CSSProperties = {
  flex: 1, textAlign: "center", padding: "3px 2px", borderRadius: 4,
  background: "#101c28",
};

const EVENT_ICON: Record<string, string> = {
  founded: "🏛", succession: "👤", monopoly: "💰", monopoly_lost: "💸",
  control_gained: "⚖", control_lost: "💔", branch: "🌿", loss: "⚠️", dissolved: "🪦",
};
const EVENT_COLOR: Record<string, string> = {
  founded: "#cfe0f4", succession: "#9ab0c8", monopoly: "#e0b060", monopoly_lost: "#b08a5a",
  control_gained: "#7fd0a0", control_lost: "#d88", loss: "#e08a5a",
  branch: "#9fe07a", dissolved: "#8a93a0",
};

/** A house's chronicle as a vertical timeline: founding, successions, monopolies,
 *  cities controlled (gained/lost + year), the worst loss — plus its most
 *  profitable trade resources. */
function HouseTimeline({ history, onClose }: { history: HouseHistory; onClose: () => void }) {
  const ev = history.events;
  const maxProfit = Math.max(1e-6, ...history.top_goods.map(([, p]) => p));
  return (
    <div style={timelinePanel}>
      <div style={{ ...header, borderBottom: "1px solid #1a2a3e" }}>
        <span style={{ display: "flex", alignItems: "center", gap: 7 }}>
          <span style={{ width: 11, height: 11, borderRadius: 2, background: history.color }} />
          <CoatOfArms name={history.name} size={22} />
          <span>{history.name}</span>
        </span>
        <span style={{ cursor: "pointer", color: "#7a90a8" }} onClick={onClose}>✕</span>
      </div>
      <div style={{ overflowY: "auto", padding: "8px 12px 12px" }}>
        <div style={{ color: "#9ab0c8", fontSize: 10, marginBottom: 8 }}>
          {history.founder || `Founded in year ${history.founded_year}`}
          {history.defunct && <span style={{ color: "#d88" }}> · fallen</span>}
        </div>

        {/* Most profitable resources */}
        {history.top_goods.length > 0 && (
          <>
            <div style={timelineHdr}>Most profitable trade resources</div>
            {history.top_goods.map(([g, p]) => (
              <div key={g} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
                <span style={{ fontSize: 12, width: 16 }}>{goodIcon(g)}</span>
                <span style={{ color: "#cdbb88", fontSize: 10, width: 78, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{g}</span>
                <div style={{ flex: 1, height: 4, background: "#0a1018", borderRadius: 2, overflow: "hidden" }}>
                  <div style={{ width: `${(p / maxProfit) * 100}%`, height: "100%", background: "#c9a227" }} />
                </div>
              </div>
            ))}
          </>
        )}

        {/* Timeline */}
        <div style={{ ...timelineHdr, marginTop: 10 }}>Chronicle</div>
        {ev.length === 0 && <div style={empty}>No recorded events yet.</div>}
        <div style={{ position: "relative", paddingLeft: 14 }}>
          {/* vertical rail */}
          <div style={{ position: "absolute", left: 4, top: 4, bottom: 4, width: 2, background: "#1c2c40" }} />
          {ev.map((e, i) => (
            <div key={i} style={{ position: "relative", marginBottom: 8 }}>
              <span style={{ position: "absolute", left: -14, top: 0, fontSize: 11 }}>{EVENT_ICON[e.kind] ?? "•"}</span>
              <div style={{ color: "#6a86a6", fontSize: 9 }}>Year {e.year}</div>
              <div style={{ color: EVENT_COLOR[e.kind] ?? "#c0d0e0", fontSize: 11 }}>{e.text}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

const timelinePanel: React.CSSProperties = {
  position: "absolute", top: 0, right: 326, width: 300, maxHeight: "78vh",
  display: "flex", flexDirection: "column",
  background: "#0a121c", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.6)", zIndex: 41,
};
const timelineHdr: React.CSSProperties = {
  color: "#7a90a8", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4,
  margin: "4px 0 3px", borderBottom: "1px solid #16222e", paddingBottom: 2,
};
