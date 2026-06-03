import { useUIStore } from "../../state/uiStore";
import { simGeneratePlates, simInvertTerrain, loadImageTemplate } from "../../bridge/tauri";
import { genBtn } from "./WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

export function StepLandmass({ seed, plateCount, invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const setLandmassSource = useUIStore((s) => s.setLandmassSource);

  const handleGeneratePlates = async () => {
    if (simRunning) return;
    setSimRunning(true);
    setStatus("Generating plates & landmass...");
    try {
      await simGeneratePlates(seed, plateCount);
      invalidateTiles();
      markStepCompleted(1);
      setLandmassSource("plates");
      setStatus("Plates & landmass generated");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const handleInvert = async () => {
    if (simRunning) return;
    setSimRunning(true);
    setStatus("Inverting terrain...");
    try {
      await simInvertTerrain();
      invalidateTiles();
      markStepCompleted(1);
      setStatus("Terrain inverted");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const handleImageTemplate = async () => {
    if (simRunning) return;
    try {
      console.log("[template] opening dialog...");
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "bmp", "webp"] }],
      });
      console.log("[template] dialog result:", result);
      if (!result) return;
      const path = result as string;

      setSimRunning(true);
      setStatus(`Loading template...`);

      console.log("[template] invoking load_image_template with path:", path);
      const modified = await loadImageTemplate(path);
      console.log("[template] success, modified tiles:", modified.length);
      invalidateTiles();
      markStepCompleted(1);
      setLandmassSource("template");
      setStatus(`Template loaded (${modified.length} tiles) \u2014 image auto-resized to world grid`);
    } catch (err) {
      setStatus(`Template error: ${err}`);
      console.error("Template load failed:", err);
    }
    setSimRunning(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <button onClick={handleImageTemplate} disabled={simRunning} style={genBtn}>
        Load Image Template
      </button>
      <button onClick={handleGeneratePlates} disabled={simRunning} style={genBtn}>
        Generate from Plates
      </button>
      <button onClick={handleInvert} disabled={simRunning} style={genBtn}>
        Invert Land / Sea
      </button>
      <div style={{ color: "#3a5068", fontSize: 10, marginTop: 2 }}>
        Template images are auto-resized to fit the world grid.
        Use the Paint tool to draw landmasses manually. Shift+drag to erase.
      </div>
    </div>
  );
}
