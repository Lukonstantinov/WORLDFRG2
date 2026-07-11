import { useUIStore } from "../state/uiStore";

/** A floating top bar of toggle chips for EVERY window + the workflow panel and
 *  toolbar — so the user can pop windows on/off and strip the UI down to a clean
 *  map with just this bar ("Hide all"). Each chip lights up when its window is open. */
export function WindowBar() {
  const s = useUIStore();
  const chips: { label: string; on: boolean; set: (v: boolean) => void }[] = [
    { label: "⚜️ Houses", on: s.showHouses, set: s.setShowHouses },
    { label: "💰 Money", on: s.showMoneyFinance, set: s.setShowMoneyFinance },
    { label: "🏦 Banks", on: s.showBank, set: s.setShowBank },
    { label: "🏛 Colonies", on: s.showColonial, set: s.setShowColonial },
    { label: "📦 Goods", on: s.showGoodsWindow, set: s.setShowGoodsWindow },
    { label: "🏆 Cities", on: s.showCityRanking, set: s.setShowCityRanking },
    { label: "🗞 News", on: s.showNews, set: s.setShowNews },
    { label: "🏬 Depots", on: s.showWarehouses, set: s.setShowWarehouses },
    { label: "📜 Futures", on: s.showFutures, set: s.setShowFutures },
    { label: "🔀 Trade", on: s.showTradeMatrix, set: s.setShowTradeMatrix },
    { label: "🧭 Itinerary", on: s.showItinerary, set: s.setShowItinerary },
    { label: "🌊 Hydrology", on: s.showHydrology, set: s.setShowHydrology },
    { label: "📊 Economy", on: s.showEconomyDashboard, set: s.setShowEconomyDashboard },
    { label: "📖 Codex", on: s.showGoodsCodex, set: s.setShowGoodsCodex },
  ];
  const chrome: { label: string; on: boolean; set: (v: boolean) => void }[] = [
    { label: "◧ Workflow", on: s.showWorkflow, set: s.setShowWorkflow },
    { label: "Toolbar ◨", on: s.showToolbar, set: s.setShowToolbar },
  ];
  const hideAll = () => {
    chips.forEach((c) => c.set(false));
    s.setShowWorkflow(false);
    s.setShowToolbar(false);
  };

  const chip = (c: { label: string; on: boolean; set: (v: boolean) => void }) => (
    <button key={c.label} onClick={() => c.set(!c.on)} title={`Toggle ${c.label}`}
      style={{
        padding: "3px 9px", borderRadius: 5, fontSize: 11, cursor: "pointer", whiteSpace: "nowrap",
        border: `1px solid ${c.on ? "#3a80c0" : "#1e2e42"}`,
        background: c.on ? "#19324a" : "rgba(12,18,26,0.6)",
        color: c.on ? "#cfe2f6" : "#6a86a6", fontWeight: c.on ? 600 : 400,
      }}>{c.label}</button>
  );

  return (
    <div style={{
      position: "absolute", top: 8, left: "50%", transform: "translateX(-50%)", zIndex: 130,
      display: "flex", alignItems: "center", gap: 4, flexWrap: "wrap", justifyContent: "center",
      maxWidth: "78%", padding: "5px 8px", borderRadius: 8,
      background: "rgba(9,14,20,0.92)", border: "1px solid #1e2e42",
      boxShadow: "0 6px 22px rgba(0,0,0,0.5)",
    }}>
      {chips.map(chip)}
      <span style={{ width: 1, height: 18, background: "#24364e", margin: "0 2px" }} />
      {chrome.map(chip)}
      <button onClick={hideAll} title="Hide every window, the workflow panel and the toolbar"
        style={{
          padding: "3px 9px", borderRadius: 5, fontSize: 11, cursor: "pointer", whiteSpace: "nowrap",
          border: "1px solid #5a2f3a", background: "#3a1820", color: "#e0a0a0", fontWeight: 600,
        }}>✕ Hide all</button>
    </div>
  );
}
