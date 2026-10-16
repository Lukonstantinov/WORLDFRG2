import { useUIStore } from "@state/uiStore";
import { simClassifyClimate } from "@bridge";
import { genBtn } from "@ui/workflow/WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}


export function StepClimate({ invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const setLayer = useUIStore((s) => s.setLayer);

  const step3Done = stepCompleted[3] === true;

  const handleGenerate = async () => {
    if (simRunning) return;
    if (!step3Done) {
      setStatus("Step 3 required: Generate ocean & atmosphere first (wind, currents, temperature, precipitation)");
      return;
    }
    setSimRunning(true);
    setStatus("Classifying climate zones...");
    try {
      await simClassifyClimate();
      invalidateTiles();
      markStepCompleted(4);
      setLayer("climate");
      setStatus("K\u00F6ppen climate classification complete");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step3Done && (
        <div style={{ color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3 }}>
          Generate the winds, sea and rain first (temperature and rain decide the zones)
        </div>
      )}
      <button onClick={handleGenerate} disabled={simRunning || !step3Done} style={genBtn}>
        Generate Climate Zones
      </button>
      <div style={{ display: "flex", gap: 4, marginTop: 2 }}>
        <button onClick={() => setLayer("climate")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>
          View Climate
        </button>
        <button onClick={() => setLayer("biomes")}
          title="Biomes are classified in Water & Life; before then this shows an approximation from the climate zones."
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>
          View Biomes
        </button>
      </div>

      <div style={{ color: "#7a6d56", fontSize: 10, lineHeight: 1.4, marginTop: 4 }}>
        The key under the map lists every climate zone in the map's own colours.
        These are CLIMATE zones; the living biomes they feed (rainforest, taiga,
        mangrove…) are classified in Water &amp; Life, once rivers and soil exist.
      </div>
    </div>
  );
}
