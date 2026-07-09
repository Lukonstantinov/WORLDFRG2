import { useUIStore } from "../../state/uiStore";
import { useWorldStore } from "../../state/worldStore";
import { simRiversHydrology, simRefreshHydrologyBiology } from "../../bridge/tauri";
import { genBtn } from "./WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

export function StepRivers({ seed, invalidateTiles }: Props) {
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
        riverParams.density, 1.0, // width is now physical (precip × drainage × climate)
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

  // One-click refresh: re-run hydrology → soil/fertility → biology on the existing
  // world, so an older world gains meanders, oxbow backwaters, salt lakes, delta
  // abundance and the salt/goods economy without re-rolling terrain or moving cities.
  const handleRefresh = async () => {
    if (simRunning) return;
    setSimRunning(true);
    setStatus("Refreshing rivers, lakes, salt & goods…");
    try {
      const result = await simRefreshHydrologyBiology(
        seed, riverParams.density, 1.0,
        riverParams.lakeFillDepth, riverParams.lakeMaxFraction, 6, 0.5,
      );
      setRivers(result.rivers);
      setLakes(result.lakes);
      invalidateTiles();
      setOverlayVisible("rivers", true);
      setOverlayVisible("lakes", true);
      setStatus(`Refreshed: ${result.rivers.length} rivers, ${result.lakes.length} lakes, salt & goods updated`);
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
        {slider("Lake Depth Threshold", riverParams.lakeFillDepth, 0.001, 0.02, 0.001,
          (v) => setRiverParams({ lakeFillDepth: v }), (v) => `${Math.round(v * 8848)}m`,
          "More lakes ↔ Only deep basins")}
        <div style={{ color: "#405060", fontSize: 9, marginTop: 2 }}>
          River width is derived from discharge (precipitation × drainage area ×
          climate) — wider, deeper-blue downstream; arid rivers stay thin.
        </div>
      </div>

      <button onClick={handleGenerate} disabled={simRunning || !step2Done} style={genBtn}>
        Generate Rivers & Lakes
      </button>
      {rivers.length > 0 && (
        <div style={{ color: "#608060", fontSize: 10 }}>
          {rivers.length} rivers, {lakes.length} lakes extracted
        </div>
      )}

      <button onClick={handleRefresh} disabled={simRunning || !step2Done}
        title="Re-run hydrology → soil/fertility → biology on the existing world: adds meanders, oxbow backwaters, salt lakes, delta abundance and salt/goods — without re-rolling terrain or moving settlements."
        style={{ ...genBtn, background: "#12222e", color: "#8fc0d8", border: "1px solid #244a60" }}>
        🔄 Refresh meanders, salt &amp; goods
      </button>
      <div style={{ color: "#405060", fontSize: 9 }}>
        Refresh updates rivers, lakes, salt &amp; the goods economy on an existing
        world (keeps your terrain &amp; cities). Full regen: use “Complete from Landmass”.
      </div>
      <div style={{ color: "#405060", fontSize: 10 }}>
        Uses D8 steepest-descent flow routing over a depression-filled surface.
        Adjust density for the river network and the lake threshold for how
        readily basins fill.
      </div>
    </div>
  );
}
