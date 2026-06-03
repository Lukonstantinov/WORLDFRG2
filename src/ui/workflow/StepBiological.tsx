import { useUIStore } from "../../state/uiStore";
import { useWorldStore } from "../../state/worldStore";
import { simBiological } from "../../bridge/tauri";
import { GOOD_DEFS, goodOverlayKey } from "../../goods";
import { genBtn } from "./WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

export function StepBiological({ seed, invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const setLayer = useUIStore((s) => s.setLayer);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const setShowTradeMatrix = useUIStore((s) => s.setShowTradeMatrix);
  const rivers = useWorldStore((s) => s.rivers);

  const step6Done = stepCompleted[6] === true;
  const step7Done = stepCompleted[7] === true;

  const handleGenerate = async () => {
    if (simRunning) return;
    if (!step6Done) {
      setStatus("Step 6 required: compute Soil & Fertility first (fisheries drive shark/fish goods)");
      return;
    }
    if (!step7Done) {
      setStatus("Step 7 required: place Settlements first (the trade matrix groups them into regions)");
      return;
    }
    setSimRunning(true);
    setStatus("Computing shark waters, trade goods, routes & matrix...");
    try {
      await simBiological(seed, JSON.stringify(rivers));
      markStepCompleted(8); // gates the trade-route / flow computation in MapCanvas
      invalidateTiles();    // bumps tileVersion → refetches shark/goods/routes/flows
      // Surface the new overlays.
      setOverlayVisible("sharkZones", true);
      setOverlayVisible("tradeRoutes", true);
      setOverlayVisible("tradeFlows", true);
      setStatus("Biological-Trade computed: sharks, goods, routes & trade matrix");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const enableAllGoods = () => {
    for (const g of GOOD_DEFS) setOverlayVisible(goodOverlayKey(g.name), true);
  };
  const disableAllGoods = () => {
    for (const g of GOOD_DEFS) setOverlayVisible(goodOverlayKey(g.name), false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step6Done && (
        <div style={warn}>Complete Step 6 first (fisheries feed shark & fish-good belts)</div>
      )}
      {step6Done && !step7Done && (
        <div style={warn}>Complete Step 7 first (the trade matrix needs settlements)</div>
      )}

      <button onClick={handleGenerate} disabled={simRunning || !step6Done || !step7Done} style={genBtn}>
        Generate Biological Layer
      </button>

      <div style={{ display: "flex", gap: 4, marginTop: 2 }}>
        <button onClick={() => setLayer("shark")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Shark Waters</button>
        <button onClick={() => setLayer("salinity")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Salinity</button>
      </div>

      <div style={{ display: "flex", gap: 4 }}>
        <button onClick={enableAllGoods}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Show all goods</button>
        <button onClick={disableAllGoods}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Hide goods</button>
      </div>

      <button onClick={() => setShowTradeMatrix(true)} disabled={!stepCompleted[8]}
        style={{ ...genBtn, marginTop: 2 }}>
        Open Trade Matrix
      </button>

      <div style={{ color: "#405060", fontSize: 10, marginTop: 2 }}>
        Shark waters: warm, shallow, frequented coasts (bull/tiger-shark habitat).
        Trade goods: {GOOD_DEFS.length} climate/terrain belts, each a toggle in the
        Toolbar &rsaquo; Trade Goods. Trade matrix groups settlements into regions
        and matches each good&rsquo;s surplus to demand as flows.
      </div>
    </div>
  );
}

const warn: React.CSSProperties = {
  color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3,
};
