import { useUIStore } from "../../state/uiStore";
import { useWorldStore } from "../../state/worldStore";
import { computeEconomy } from "../../bridge/tauri";
import { genBtn } from "./WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

export function StepEconomy(_props: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const setShowTradeMatrix = useUIStore((s) => s.setShowTradeMatrix);
  const overlayVisibility = useUIStore((s) => s.overlayVisibility);
  const reachGood = useUIStore((s) => s.reachGood);
  const setReachGood = useUIStore((s) => s.setReachGood);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const bioParams = useUIStore((s) => s.bioParams);
  const settlements = useWorldStore((s) => s.settlements);
  const rivers = useWorldStore((s) => s.rivers);
  const economy = useWorldStore((s) => s.economy);
  const setEconomy = useWorldStore((s) => s.setEconomy);

  const step9Done = stepCompleted[9] === true;

  const handleCompute = async () => {
    if (simRunning) return;
    if (!step9Done) {
      setStatus("Step 9 required: compute the Political layer first (the economy is anchored on hubs)");
      return;
    }
    setSimRunning(true);
    setStatus("Building economy: production, quality grades, prices, wealth & chokepoints...");
    try {
      const result = await computeEconomy(
        settlements.map((s) => ({ x: s.x, y: s.y, score: s.score, population: s.population })),
        rivers.map((r) => ({ points: r.points })),
        bioParams.tradeReach,
        bioParams.maxCrossing,
        bioParams.desertRoutes,
        bioParams.economicRegions,
        bioParams.luxuryBias,
        bioParams.piracyLevel,
        bioParams.tradeSeason,
        bioParams.calendarMonths,
      );
      setEconomy(result);
      markStepCompleted(10);
      setOverlayVisible("chokepoints", true);
      setOverlayVisible("politicalInfluence", true); // show trade-hub circles
      setStatus(`Economy built: ${result.hubs.length} hubs, ${result.chains.length} supply chains, ${result.regions.length} regions, ${result.chokepoints.length} chokepoints`);
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step9Done && (
        <div style={warn}>Complete Step 9 first (the economy is built on the political hubs)</div>
      )}

      <button onClick={handleCompute} disabled={simRunning || !step9Done} style={genBtn}>
        Build Economy
      </button>

      {economy && economy.hubs.length > 0 && (
        <>
          <button onClick={() => setShowTradeMatrix(true)} style={{ ...genBtn, marginTop: 4 }}>
            📊 Open Trade Matrix
          </button>

          {/* Per-good reach: highlight which hubs a good reaches on the map. */}
          <div style={{ marginTop: 5 }}>
            <div style={{ fontSize: 10, color: "#8aa0c0", marginBottom: 2 }}>Good reach (highlight on map)</div>
            <select
              value={reachGood ?? ""}
              onChange={(e) => setReachGood(e.target.value || null)}
              style={{ width: "100%", fontSize: 10, padding: "2px 4px", background: "#0d1219", color: "#c0d8f0", border: "1px solid #1e2a38", borderRadius: 4 }}
            >
              <option value="">— none —</option>
              {economy.goods.map((g) => (
                <option key={g} value={g}>{g}</option>
              ))}
            </select>
            {reachGood && (() => {
              const chains = economy.chains.filter((c) => c.good_name === reachGood);
              const hubs = new Set(chains.map((c) => { const p = c.points[c.points.length - 1]; return p ? `${p[0]},${p[1]}` : ""; }));
              return (
                <div style={{ fontSize: 10, color: "#c8b070", marginTop: 2 }}>
                  {reachGood}: reaches {hubs.size} hub{hubs.size === 1 ? "" : "s"} via {chains.length} route{chains.length === 1 ? "" : "s"}
                </div>
              );
            })()}
          </div>

          {([
            { key: "tradeRegions", label: "Trade regions (territories)" },
            { key: "chokepoints", label: "Strategic chokepoints" },
            { key: "hubNames", label: "Hub names" },
            { key: "settlementNames", label: "Settlement names" },
          ] as const).map((o) => (
            <label key={o.key} style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 10, color: "#8aa0c0", marginTop: 3, cursor: "pointer" }}>
              <input type="checkbox" checked={!!overlayVisibility[o.key]}
                onChange={(e) => setOverlayVisible(o.key, e.target.checked)} />
              {o.label}
            </label>
          ))}
          <div style={{ marginTop: 4, maxHeight: 180, overflowY: "auto" }}>
            {economy.chokepoints.slice(0, 10).map((cp, i) => (
              <div key={i} style={{
                display: "flex", justifyContent: "space-between",
                fontSize: 10, padding: "2px 4px", borderBottom: "1px solid #14202e", color: "#d0b0a0",
              }}>
                <span>{cp.name}</span>
                <span style={{ color: "#a08070" }}>{Math.round(cp.share * 100)}%</span>
              </div>
            ))}
          </div>
        </>
      )}

      <div style={{ color: "#405060", fontSize: 10, marginTop: 2 }}>
        Builds the <b>trade economy</b> on the political hubs: quality-graded
        production, cost-aware flows with a price at every hop, per-hub wealth,
        and the world&rsquo;s strategic chokepoints (the busiest straits &amp;
        passes). Click a hub to inspect its goods &amp; supply chains.
      </div>
    </div>
  );
}

const warn: React.CSSProperties = {
  color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3,
};
