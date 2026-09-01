import { useState } from "react";
import { useUIStore } from "@state/uiStore";
import {
  simGeneratePlates, simInvertTerrain, loadImageTemplate,
  landOpSmoothRoughen, landOpFjords, landOpIslands, landOpFill,
  renderWorldThumbnail, undoAction,
  type IslandKind,
} from "@bridge";
import { genBtn } from "@ui/workflow/workflowStyles";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

const panelStyle: React.CSSProperties = {
  border: "1px solid #22384f", borderRadius: 4, padding: 6, marginTop: 6,
  display: "flex", flexDirection: "column", gap: 5,
};
const rowStyle: React.CSSProperties = { display: "flex", flexWrap: "wrap", alignItems: "center", gap: 6, fontSize: 11 };
const labelStyle: React.CSSProperties = { color: "#8ba3bd", minWidth: 60 };
const smallBtn: React.CSSProperties = {
  ...genBtn, padding: "3px 8px", fontSize: 11, marginTop: 0,
};

function randomSeed(): number {
  return Math.floor(Math.random() * 0xffffffff);
}

export function StepLandmass({ seed, plateCount, invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const setLandmassSource = useUIStore((s) => s.setLandmassSource);
  const lassoPolygon = useUIStore((s) => s.lassoPolygon);
  const clearLasso = useUIStore((s) => s.clearLasso);
  const activeTool = useUIStore((s) => s.activeTool);
  const setTool = useUIStore((s) => s.setTool);

  const [areaOpen, setAreaOpen] = useState(true);
  const [smoothAmount, setSmoothAmount] = useState(-0.5);
  const [fjordCount, setFjordCount] = useState(3);
  const [fjordLength, setFjordLength] = useState(300);
  const [fjordWidth, setFjordWidth] = useState(3);
  const [islandCount, setIslandCount] = useState(5);
  const [islandKind, setIslandKind] = useState<IslandKind>("arc");
  const [islandSize, setIslandSize] = useState(3);
  const [lastOpSeed, setLastOpSeed] = useState<number>(randomSeed());
  // Rolling a fresh seed on every Generate press is the right default for
  // brainstorming (ITCZ_AND_LAND_TOOLS_PLAN.md Commit 2) — pressing "Generate
  // from Plates" twice used to give the IDENTICAL world. Lock it to iterate
  // sliders against one fixed landmass instead.
  const [lockSeed, setLockSeed] = useState(false);

  const [variants, setVariants] = useState<
    { a: { seed: number; thumb: string; w: number; h: number }; b: { seed: number; thumb: string; w: number; h: number } } | null
  >(null);

  const hasLasso = lassoPolygon.length >= 3;

  const handleGeneratePlates = async (useSeed?: number) => {
    if (simRunning) return;
    setSimRunning(true);
    setStatus("Generating plates & landmass...");
    try {
      await simGeneratePlates(useSeed ?? seed, plateCount);
      invalidateTiles();
      markStepCompleted(1);
      setLandmassSource("plates");
      setStatus("Plates & landmass generated");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const handleRandomise = async () => {
    await handleGeneratePlates(randomSeed());
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
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "bmp", "webp"] }],
      });
      if (!result) return;
      const path = result as string;
      setSimRunning(true);
      setStatus(`Loading template...`);
      const modified = await loadImageTemplate(path);
      invalidateTiles();
      markStepCompleted(1);
      setLandmassSource("template");
      setStatus(`Template loaded (${modified.length} tiles) — image auto-resized to world grid`);
    } catch (err) {
      setStatus(`Template error: ${err}`);
      console.error("Template load failed:", err);
    }
    setSimRunning(false);
  };

  // ── Area tools: each op consumes the drawn lasso, so it clears after use. A
  // Re-roll button re-runs the same op with a fresh seed WITHOUT drawing again —
  // undo the previous result first, then re-apply against the same polygon.
  const runOp = async (fn: () => Promise<[number, number][]>, label: string, keepLasso = false) => {
    if (simRunning || !hasLasso) return;
    setSimRunning(true);
    setStatus(`${label}...`);
    try {
      const modified = await fn();
      invalidateTiles();
      setStatus(`${label}: ${modified.length} tiles changed`);
      if (!keepLasso) clearLasso();
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const applySmoothRoughen = () => {
    const s = randomSeed();
    setLastOpSeed(s);
    return runOp(
      () => landOpSmoothRoughen(lassoPolygon, smoothAmount, s),
      smoothAmount < 0 ? "Smoothing coastline" : "Roughening coastline",
      true,
    );
  };
  const applyFjords = () => {
    const s = randomSeed();
    setLastOpSeed(s);
    return runOp(
      () => landOpFjords(lassoPolygon, fjordCount, fjordLength, fjordWidth, s),
      "Carving fjords",
      true,
    );
  };
  const applyIslands = () => {
    const s = randomSeed();
    setLastOpSeed(s);
    return runOp(
      () => landOpIslands(lassoPolygon, islandCount, islandKind, islandSize, s),
      "Placing islands",
      true,
    );
  };
  const applyFill = (land: boolean) =>
    runOp(() => landOpFill(lassoPolygon, land), land ? "Filling land" : "Filling sea", true);

  // Re-roll: undo the last op's result, then re-apply the same op with a new
  // seed against the SAME lasso (still held since ops keep it while area tools
  // are open).
  const reroll = async (again: () => Promise<void>) => {
    if (simRunning || !hasLasso) return;
    setSimRunning(true);
    try {
      await undoAction();
      invalidateTiles();
    } finally {
      setSimRunning(false);
    }
    await again();
  };

  // ── 2-variant compare: generate A → thumbnail → undo → generate B →
  // thumbnail → show both. Keeping either just re-generates from its own seed
  // (deterministic) or, for the one already on the map (B), does nothing.
  const handleCompareVariants = async () => {
    if (simRunning) return;
    setSimRunning(true);
    setStatus("Generating variant A...");
    try {
      const seedA = randomSeed();
      await simGeneratePlates(seedA, plateCount);
      const thumbA = await renderWorldThumbnail(220);
      await undoAction();
      setStatus("Generating variant B...");
      const seedB = randomSeed();
      await simGeneratePlates(seedB, plateCount);
      const thumbB = await renderWorldThumbnail(220);
      invalidateTiles();
      markStepCompleted(1);
      setLandmassSource("plates");
      setVariants({
        a: { seed: seedA, thumb: thumbA.rgba, w: thumbA.width, h: thumbA.height },
        b: { seed: seedB, thumb: thumbB.rgba, w: thumbB.width, h: thumbB.height },
      });
      setStatus("Two variants generated — B is on the map now. Pick one.");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const keepVariant = async (which: "a" | "b") => {
    if (!variants || simRunning) return;
    if (which === "b") {
      // B is already the world on disk.
      setVariants(null);
      return;
    }
    setSimRunning(true);
    setStatus("Restoring variant A...");
    try {
      await simGeneratePlates(variants.a.seed, plateCount);
      invalidateTiles();
      setVariants(null);
      setStatus("Variant A kept");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <button onClick={handleImageTemplate} disabled={simRunning} style={genBtn}>
        Load Image Template
      </button>
      <button onClick={() => handleGeneratePlates(lockSeed ? seed : randomSeed())} disabled={simRunning} style={genBtn}>
        Generate from Plates
      </button>
      <label style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 10, color: "#6a8aaa" }}>
        <input type="checkbox" checked={lockSeed} onChange={(e) => setLockSeed(e.target.checked)} />
        Lock seed (iterate settings on this same landmass)
      </label>
      <button onClick={handleRandomise} disabled={simRunning} style={genBtn}>
        🎲 Randomise landmass (new seed, ignores lock)
      </button>
      <button onClick={handleInvert} disabled={simRunning} style={genBtn}>
        Invert Land / Sea
      </button>
      <button onClick={handleCompareVariants} disabled={simRunning} style={genBtn}>
        ⚖️ Compare 2 variants
      </button>

      {variants && (
        <div style={panelStyle}>
          <div style={{ fontSize: 11, color: "#8ba3bd" }}>Two generated variants — pick one:</div>
          <div style={{ display: "flex", gap: 8 }}>
            <VariantThumb label="A" data={variants.a} onKeep={() => keepVariant("a")} disabled={simRunning} />
            <VariantThumb label="B (on map)" data={variants.b} onKeep={() => keepVariant("b")} disabled={simRunning} highlight />
          </div>
        </div>
      )}

      <button
        onClick={() => setAreaOpen((v) => !v)}
        style={{ ...genBtn, background: "transparent", border: "1px solid #22384f", textAlign: "left" }}
      >
        {areaOpen ? "▾" : "▸"} Area tools {hasLasso ? "(lasso drawn)" : ""}
      </button>
      {areaOpen && (
        <div style={panelStyle}>
          <div style={rowStyle}>
            <button
              onClick={() => setTool(activeTool === "lasso" ? "pan" : "lasso")}
              style={{ ...smallBtn, background: activeTool === "lasso" ? "#2a5a7a" : undefined }}
            >
              {activeTool === "lasso" ? "Drawing lasso… click to stop" : "✏️ Draw lasso"}
            </button>
            {hasLasso && (
              <button onClick={clearLasso} style={smallBtn}>Clear</button>
            )}
          </div>
          <div style={{ fontSize: 10, color: "#5a7591" }}>
            Freehand-drag a selection on the map, then apply an op below. Every
            op fades out at the lasso edge instead of cutting a hard line.
          </div>

          <div style={{ borderTop: "1px solid #1a2c3f", paddingTop: 5 }}>
            <div style={rowStyle}>
              <span style={labelStyle}>Smooth↔Rough</span>
              <input
                type="range" min={-1} max={1} step={0.05} value={smoothAmount}
                onChange={(e) => setSmoothAmount(parseFloat(e.target.value))}
                style={{ flex: 1 }}
              />
              <span style={{ width: 30, textAlign: "right" }}>{smoothAmount.toFixed(2)}</span>
            </div>
            <div style={rowStyle}>
              <button onClick={applySmoothRoughen} disabled={simRunning || !hasLasso} style={smallBtn}>Apply</button>
              <button onClick={() => reroll(applySmoothRoughen)} disabled={simRunning || !hasLasso} style={smallBtn}>Re-roll</button>
              <span style={{ fontSize: 9, color: "#4a6580" }}>seed {lastOpSeed}</span>
            </div>
          </div>

          <div style={{ borderTop: "1px solid #1a2c3f", paddingTop: 5 }}>
            <div style={rowStyle}>
              <span style={labelStyle}>Fjords</span>
              <span>count</span>
              <input type="number" min={1} max={20} value={fjordCount}
                onChange={(e) => setFjordCount(parseInt(e.target.value) || 1)} style={{ width: 40 }} />
              <span>len km</span>
              <input type="number" min={50} max={2000} step={50} value={fjordLength}
                onChange={(e) => setFjordLength(parseFloat(e.target.value) || 100)} style={{ width: 55 }} />
              <span>width</span>
              <input type="number" min={1} max={12} value={fjordWidth}
                onChange={(e) => setFjordWidth(parseFloat(e.target.value) || 1)} style={{ width: 35 }} />
            </div>
            <div style={rowStyle}>
              <button onClick={applyFjords} disabled={simRunning || !hasLasso} style={smallBtn}>Apply</button>
              <button onClick={() => reroll(applyFjords)} disabled={simRunning || !hasLasso} style={smallBtn}>Re-roll</button>
            </div>
          </div>

          <div style={{ borderTop: "1px solid #1a2c3f", paddingTop: 5 }}>
            <div style={rowStyle}>
              <span style={labelStyle}>Islands</span>
              <select value={islandKind} onChange={(e) => setIslandKind(e.target.value as IslandKind)}>
                <option value="arc">Arc (volcanic)</option>
                <option value="scatter">Scatter</option>
                <option value="single">Single</option>
              </select>
              <span>count</span>
              <input type="number" min={1} max={30} value={islandCount}
                onChange={(e) => setIslandCount(parseInt(e.target.value) || 1)} style={{ width: 40 }}
                disabled={islandKind === "single"} />
              <span>size</span>
              <input type="number" min={1} max={12} value={islandSize}
                onChange={(e) => setIslandSize(parseFloat(e.target.value) || 1)} style={{ width: 35 }} />
            </div>
            <div style={rowStyle}>
              <button onClick={applyIslands} disabled={simRunning || !hasLasso} style={smallBtn}>Apply</button>
              <button onClick={() => reroll(applyIslands)} disabled={simRunning || !hasLasso} style={smallBtn}>Re-roll</button>
            </div>
          </div>

          <div style={{ borderTop: "1px solid #1a2c3f", paddingTop: 5 }}>
            <div style={rowStyle}>
              <span style={labelStyle}>Fill</span>
              <button onClick={() => applyFill(true)} disabled={simRunning || !hasLasso} style={smallBtn}>Land</button>
              <button onClick={() => applyFill(false)} disabled={simRunning || !hasLasso} style={smallBtn}>Sea</button>
            </div>
          </div>
        </div>
      )}

      <div style={{ color: "#3a5068", fontSize: 10, marginTop: 2 }}>
        Template images are auto-resized to fit the world grid.
        Use the Paint tool to draw landmasses manually. Shift+drag to erase.
      </div>
    </div>
  );
}

function VariantThumb({
  label, data, onKeep, disabled, highlight,
}: {
  label: string;
  data: { seed: number; thumb: string; w: number; h: number };
  onKeep: () => void;
  disabled?: boolean;
  highlight?: boolean;
}) {
  // Decode the base64 RGBA into a data URL via an offscreen canvas.
  const [src, setSrc] = useState<string | null>(null);
  if (src === null && typeof document !== "undefined") {
    try {
      const bin = atob(data.thumb);
      const bytes = new Uint8ClampedArray(bin.length);
      for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      const canvas = document.createElement("canvas");
      canvas.width = data.w;
      canvas.height = data.h;
      const ctx = canvas.getContext("2d");
      if (ctx) {
        ctx.putImageData(new ImageData(bytes, data.w, data.h), 0, 0);
        setSrc(canvas.toDataURL());
      }
    } catch {
      // leave src null; show a placeholder
    }
  }
  return (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 3, alignItems: "center" }}>
      <div style={{
        border: highlight ? "1px solid #4ad0e0" : "1px solid #22384f",
        width: "100%", aspectRatio: `${data.w} / ${data.h}`, overflow: "hidden", borderRadius: 3,
      }}>
        {src && <img src={src} alt={label} style={{ width: "100%", height: "100%", imageRendering: "pixelated" }} />}
      </div>
      <div style={{ fontSize: 10, color: "#8ba3bd" }}>{label}</div>
      <button onClick={onKeep} disabled={disabled} style={smallBtn}>Keep {label[0]}</button>
    </div>
  );
}
