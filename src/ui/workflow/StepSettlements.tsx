import { useUIStore } from "../../state/uiStore";
import { useWorldStore } from "../../state/worldStore";
import { useViewportStore } from "../../state/viewportStore";
import { simGenerateSettlements } from "../../bridge/tauri";
import { genBtn } from "./WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

export function StepSettlements({ seed, invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const setLayer = useUIStore((s) => s.setLayer);
  const focusOn = useViewportStore((s) => s.focusOn);
  const { rivers, settlements, setSettlements } = useWorldStore();

  const step6Done = stepCompleted[6] === true;

  const handleGenerate = async () => {
    if (simRunning) return;
    if (!step6Done) {
      setStatus("Step 6 required: Generate soil & fertility first (settlements need habitability data)");
      return;
    }
    setSimRunning(true);
    setStatus("Scoring habitability & placing cities...");
    try {
      const result = await simGenerateSettlements(seed, JSON.stringify(rivers));
      setSettlements(result.settlements);
      markStepCompleted(7);
      // Show the habitability heatmap + ranked city dots.
      setLayer("habitability");
      invalidateTiles();
      setOverlayVisible("settlements", true);

      const capitals = result.settlements.filter((s) => s.size === "capital").length;
      const cities = result.settlements.filter((s) => s.size === "city").length;
      const towns = result.settlements.filter((s) => s.size === "town").length;
      const villages = result.settlements.filter((s) => s.size === "village").length;
      setStatus(`${result.settlements.length} sites: ${capitals} capitals, ${cities} cities, ${towns} towns, ${villages} villages`);
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const dot = (size: string) =>
    size === "capital" ? "★" : size === "city" ? "●" : size === "town" ? "○" : "·";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step6Done && (
        <div style={{ color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3 }}>
          Complete Step 6 first (settlements need fertility & habitability data)
        </div>
      )}
      <button onClick={handleGenerate} disabled={simRunning || !step6Done} style={genBtn}>
        Find Settlements
      </button>

      {settlements.length > 0 && (
        <div style={{ maxHeight: 140, overflowY: "auto", display: "flex", flexDirection: "column", gap: 1 }}>
          {[...settlements].sort((a, b) => b.population - a.population).slice(0, 60).map((s, i) => (
            <div key={s.id}
              onClick={() => focusOn(s.x, s.y)}
              title="Click to locate on the map"
              style={{ display: "flex", justifyContent: "space-between", padding: "1px 3px",
                cursor: "pointer", borderRadius: 2 }}
              onMouseEnter={(e) => (e.currentTarget.style.background = "#16202e")}
              onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}>
              <span style={{ color: "#8090b0", fontSize: 10 }}>
                {dot(s.size)} #{i + 1} {s.size}
              </span>
              <span style={{ color: "#7a8aa0", fontSize: 10 }}>
                {s.population >= 1000 ? `${(s.population / 1000).toFixed(s.population >= 10000 ? 0 : 1)}k` : s.population}
              </span>
            </div>
          ))}
          {settlements.length > 60 && (
            <div style={{ color: "#405060", fontSize: 9 }}>... and {settlements.length - 60} more</div>
          )}
        </div>
      )}

      <div style={{ color: "#405060", fontSize: 10 }}>
        The Habitability heatmap shows where civilization is most likely
        (climate 40% + fertility 20% + water 20% + terrain 10% + trade 10%).
        Cities are placed highest-probability first; dot size = settlement rank.
      </div>
    </div>
  );
}
