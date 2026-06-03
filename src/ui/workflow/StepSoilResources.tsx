import { useUIStore } from "../../state/uiStore";
import { useWorldStore } from "../../state/worldStore";
import { simSoilFertility } from "../../bridge/tauri";
import { genBtn } from "./WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

export function StepSoilResources({ invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const setLayer = useUIStore((s) => s.setLayer);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const rivers = useWorldStore((s) => s.rivers);

  const step4Done = stepCompleted[4] === true;
  const step5Done = stepCompleted[5] === true;

  const handleGenerate = async () => {
    if (simRunning) return;
    if (!step4Done) {
      setStatus("Step 4 required: Generate climate zones first (soil types depend on climate)");
      return;
    }
    if (!step5Done) {
      setStatus("Step 5 required: Generate rivers first (fertility depends on river proximity)");
      return;
    }
    setSimRunning(true);
    setStatus("Computing soil, fertility & fisheries...");
    try {
      await simSoilFertility(JSON.stringify(rivers));
      invalidateTiles();
      markStepCompleted(6);
      setStatus("Soil, fertility & fisheries computed");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step4Done && (
        <div style={{ color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3 }}>
          Complete Step 4 first (soil types depend on climate classification)
        </div>
      )}
      {step4Done && !step5Done && (
        <div style={{ color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3 }}>
          Complete Step 5 first (fertility depends on river proximity)
        </div>
      )}
      <button onClick={handleGenerate} disabled={simRunning || !step4Done || !step5Done} style={genBtn}>
        Generate Soil & Fertility
      </button>
      <div style={{ display: "flex", gap: 4, marginTop: 2 }}>
        <button onClick={() => setLayer("soil")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Soil</button>
        <button onClick={() => setLayer("fertility")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Fertility</button>
        <button onClick={() => setLayer("fisheries")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Fisheries</button>
      </div>
      <div style={{ color: "#405060", fontSize: 10 }}>
        Soil types derived from K{"\u00F6"}ppen climate. Fertility = soil (30%) + precipitation (20%)
        + temperature (15%) + river proximity (20%) + coast (10%) + volcanic (5%).
        Fisheries from upwelling zones + river mouths.
      </div>
    </div>
  );
}
