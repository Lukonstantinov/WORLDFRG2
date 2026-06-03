import { useUIStore } from "../../state/uiStore";
import { useWorldStore } from "../../state/worldStore";
import { simRiversHydrology } from "../../bridge/tauri";
import { genBtn } from "./WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

export function StepRivers({ invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const riverParams = useUIStore((s) => s.riverParams);
  const setRiverParams = useUIStore((s) => s.setRiverParams);
  const { rivers, lakes, setRivers, setLakes } = useWorldStore();

  const step2Done = stepCompleted[2] === true;

  const handleGenerate = async () => {
    if (simRunning) return;
    if (!step2Done) {
      setStatus("Step 2 required: Generate elevation first (rivers need slopes to flow)");
      return;
    }
    setSimRunning(true);
    setStatus("Extracting rivers & hydrology...");
    try {
      const result = await simRiversHydrology(
        riverParams.density, riverParams.width,
        riverParams.lakeFillDepth, riverParams.lakeMaxFraction,
      );
      setRivers(result.rivers);
      setLakes(result.lakes);
      invalidateTiles();
      markStepCompleted(5);
      setOverlayVisible("rivers", true);
      setOverlayVisible("lakes", true);
      setStatus(`Rivers: ${result.rivers.length} rivers, ${result.lakes.length} lakes`);
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const slider = (label: string, value: number, min: number, max: number, step: number,
    onChange: (v: number) => void, fmt: (v: number) => string, hint?: string) => (
    <div style={{ marginBottom: 4 }}>
      <div style={{ display: "flex", justifyContent: "space-between" }}>
        <span style={{ color: "#607090", fontSize: 10 }}>{label}</span>
        <span style={{ color: "#8090b0", fontSize: 10 }}>{fmt(value)}</span>
      </div>
      <input type="range" min={min} max={max} step={step} value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        style={{ width: "100%", height: 12 }} />
      {hint && <div style={{ color: "#405060", fontSize: 9 }}>{hint}</div>}
    </div>
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step2Done && (
        <div style={{ color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3 }}>
          Complete Step 2 first (rivers need elevation data to flow downhill)
        </div>
      )}

      <div style={{ background: "#0a1018", border: "1px solid #2a3a50", borderRadius: 4, padding: 6, marginBottom: 2 }}>
        <div style={{ color: "#6090b0", fontSize: 10, fontWeight: 600, marginBottom: 4 }}>
          River & Lake Settings
        </div>
        {slider("River Density", riverParams.density, 0.1, 1.5, 0.05,
          (v) => setRiverParams({ density: v }), (v) => v.toFixed(2),
          "Few trunk rivers ↔ Very many tributaries")}
        {slider("River Width", riverParams.width, 0.2, 2.0, 0.1,
          (v) => setRiverParams({ width: v }), (v) => `${v.toFixed(1)}×`,
          "Thin ↔ Wide")}
        {slider("Lake Depth Threshold", riverParams.lakeFillDepth, 0.001, 0.02, 0.001,
          (v) => setRiverParams({ lakeFillDepth: v }), (v) => `${Math.round(v * 8848)}m`,
          "More lakes ↔ Only deep basins")}
        {slider("Max Lake Size", riverParams.lakeMaxFraction, 0.00002, 0.005, 0.00002,
          (v) => setRiverParams({ lakeMaxFraction: v }), (v) => `${(v * 100).toFixed(3)}%`,
          "Tiny lakes ↔ Allow large")}
      </div>

      <button onClick={handleGenerate} disabled={simRunning || !step2Done} style={genBtn}>
        Generate Rivers & Lakes
      </button>
      {rivers.length > 0 && (
        <div style={{ color: "#608060", fontSize: 10 }}>
          {rivers.length} rivers, {lakes.length} lakes extracted
        </div>
      )}
      <div style={{ color: "#405060", fontSize: 10 }}>
        Uses D8 steepest-descent flow routing over a depression-filled surface.
        Adjust density/width for the river network and the lake thresholds to
        control how many and how large lakes form.
      </div>
    </div>
  );
}
