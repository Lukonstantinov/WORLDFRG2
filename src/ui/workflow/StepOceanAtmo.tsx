import { useUIStore } from "../../state/uiStore";
import { simOceanAtmosphere } from "../../bridge/tauri";
import { genBtn } from "./WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

export function StepOceanAtmo({ invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const setLayer = useUIStore((s) => s.setLayer);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);

  const step2Done = stepCompleted[2] === true;

  const handleGenerate = async () => {
    if (simRunning) return;
    if (!step2Done) {
      setStatus("Step 2 required: Generate elevation first");
      return;
    }
    setSimRunning(true);
    setStatus("Generating wind belts, ocean currents, temperature & precipitation...");
    try {
      await simOceanAtmosphere();
      invalidateTiles();
      markStepCompleted(3);
      // Auto-enable wind and current overlays so user can see results
      setOverlayVisible("wind", true);
      setOverlayVisible("currents", true);
      setOverlayVisible("latLines", true);
      setStatus("Wind belts, ocean currents, temperature & precipitation generated");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step2Done && (
        <div style={{ color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3 }}>
          Complete Step 2 first (generate elevation)
        </div>
      )}
      <button onClick={handleGenerate} disabled={simRunning || !step2Done} style={genBtn}>
        Generate Wind & Currents
      </button>
      <div style={{ display: "flex", gap: 4, marginTop: 2 }}>
        <button onClick={() => setLayer("temperature")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>
          View Temperature
        </button>
        <button onClick={() => setLayer("precipitation")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>
          View Precipitation
        </button>
      </div>
      <div style={{ display: "flex", gap: 4 }}>
        <button onClick={() => setLayer("wind")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>
          View Wind
        </button>
        <button onClick={() => setLayer("currents")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>
          View Currents
        </button>
      </div>
      <div style={{ color: "#405060", fontSize: 10 }}>
        This step computes: wind belts (trade winds, westerlies, polar easterlies),
        ocean current gyres, temperature (latitude + altitude + currents),
        and precipitation (moisture advection, ITCZ, orographic lift).
      </div>
    </div>
  );
}
