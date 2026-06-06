import { useState } from "react";
import { useUIStore } from "../../state/uiStore";
import { simGenerateShelves, simGenerateTerrainFromTemplate, simGenerateTerrainRidged, simScaleElevation } from "../../bridge/tauri";
import { genBtn } from "./WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

// Selectable terrain "templates" — each sets the four generation sliders.
const TERRAIN_PRESETS: {
  label: string; density: number; height: number; spread: number; roughness: number;
}[] = [
  { label: "Earthlike",   density: 0.5, height: 0.55, spread: 0.5,  roughness: 0.45 },
  { label: "Mountainous", density: 0.8, height: 0.85, spread: 0.35, roughness: 0.6 },
  { label: "Rolling",     density: 0.4, height: 0.3,  spread: 0.6,  roughness: 0.35 },
  { label: "Flat Plains", density: 0.2, height: 0.15, spread: 0.7,  roughness: 0.2 },
  { label: "Rugged",      density: 0.7, height: 0.7,  spread: 0.25, roughness: 0.85 },
];

export function StepElevation({ seed, invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const landmassSource = useUIStore((s) => s.landmassSource);
  const stepCompleted = useUIStore((s) => s.stepCompleted);

  // Mountain generation parameters — persisted in the store so they survive
  // switching workflow steps (previously local state that reset every time).
  const terrainParams = useUIStore((s) => s.terrainParams);
  const setTerrainParams = useUIStore((s) => s.setTerrainParams);
  const mountainDensity = terrainParams.density;
  const mountainHeight = terrainParams.height;
  const mountainSpread = terrainParams.spread;
  const noiseRoughness = terrainParams.roughness;
  const setMountainDensity = (v: number) => setTerrainParams({ density: v });
  const setMountainHeight = (v: number) => setTerrainParams({ height: v });
  const setMountainSpread = (v: number) => setTerrainParams({ spread: v });
  const setNoiseRoughness = (v: number) => setTerrainParams({ roughness: v });

  // Terrain randomiser: an independent seed so the user can roll new terrain
  // without changing the world seed. Stored so it persists across steps.
  const terrainSeed = terrainParams.seed ?? seed;
  const setTerrainSeed = (v: number) => setTerrainParams({ seed: v });

  // Elevation adjustment (scale + peak lock) — operates on already-generated
  // elevation, so the user can fine-tune relief without a full re-roll.
  const [showAdjust, setShowAdjust] = useState(false);
  const [elevPercent, setElevPercent] = useState(100); // % of current height
  const [lockPeaks, setLockPeaks] = useState(false);
  const [lockThreshold, setLockThreshold] = useState(0.6); // normalized 0-1

  // Shelf parameters
  const [showShelfDialog, setShowShelfDialog] = useState(false);
  const [shelfWidth, setShelfWidth] = useState(6);
  const [shelfNoise, setShelfNoise] = useState(0.4);
  const [shelfProfile, setShelfProfile] = useState(0.3);
  const [shelfDropoff, setShelfDropoff] = useState(8);

  // Shape-based elevation works on ANY land mask (template, painted, plate, or a
  // re-opened world), and is fully driven by the sliders below — so the user
  // always has complete control over mountains/relief regardless of how the
  // landmass was made.
  const step1Done = stepCompleted[1] === true;
  void landmassSource;

  const runElevation = async (useSeed: number, ridged = false) => {
    if (simRunning) return;
    if (!step1Done) {
      setStatus("Step 1 required: Create landmass first");
      return;
    }
    setSimRunning(true);
    setStatus(ridged
      ? "Generating world-scale ridged cordillera..."
      : "Generating elevation from terrain shape...");
    try {
      if (ridged) {
        await simGenerateTerrainRidged(useSeed, mountainDensity, mountainHeight, mountainSpread, noiseRoughness);
      } else {
        await simGenerateTerrainFromTemplate(useSeed, mountainDensity, mountainHeight, mountainSpread, noiseRoughness);
      }
      invalidateTiles();
      markStepCompleted(2);
      setStatus(`Elevation & sea depth generated (seed ${useSeed})`);
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const handleGenerateElevation = () => runElevation(terrainSeed);
  const handleGenerateRidged = () => runElevation(terrainSeed, true);

  const handleRandomizeTerrain = () => {
    const newSeed = Math.floor(Math.random() * 0xffffffff);
    setTerrainSeed(newSeed);
    runElevation(newSeed);
  };

  const handleScaleElevation = async () => {
    if (simRunning) return;
    if (!step1Done) { setStatus("Generate elevation first"); return; }
    setSimRunning(true);
    setStatus("Adjusting elevation...");
    try {
      await simScaleElevation(elevPercent / 100, lockPeaks ? lockThreshold : 2.0);
      invalidateTiles();
      const lockMsg = lockPeaks ? ` (peaks ≥ ${Math.round(lockThreshold * 8848)} m locked)` : "";
      setStatus(`Elevation scaled to ${elevPercent}%${lockMsg}`);
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const handleGenerateShelves = async () => {
    if (simRunning) return;
    setSimRunning(true);
    setStatus("Generating continental shelves...");
    try {
      await simGenerateShelves(seed, shelfWidth, shelfNoise, shelfProfile, shelfDropoff);
      invalidateTiles();
      markStepCompleted(2);
      setStatus("Shelves generated");
      setShowShelfDialog(false);
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const slider = (label: string, value: number, min: number, max: number, step: number,
    onChange: (v: number) => void, hint?: string) => (
    <div style={{ marginBottom: 4 }}>
      <div style={{ display: "flex", justifyContent: "space-between" }}>
        <span style={{ color: "#607090", fontSize: 10 }}>{label}</span>
        <span style={{ color: "#8090b0", fontSize: 10 }}>{value}</span>
      </div>
      <input type="range" min={min} max={max} step={step} value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        style={{ width: "100%", height: 12 }} />
      {hint && <div style={{ color: "#405060", fontSize: 9 }}>{hint}</div>}
    </div>
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step1Done && (
        <div style={{ color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3 }}>
          Complete Step 1 first (create landmass)
        </div>
      )}

      <div style={{ background: "#0a1018", border: "1px solid #2a3a50", borderRadius: 4, padding: 6, marginBottom: 2 }}>
        <div style={{ color: "#6090b0", fontSize: 10, fontWeight: 600, marginBottom: 4 }}>
          Terrain Generation Settings
        </div>
        <div style={{ color: "#506880", fontSize: 9, marginBottom: 3 }}>Presets (click, then Generate):</div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 3, marginBottom: 6 }}>
          {TERRAIN_PRESETS.map((p) => {
            const active = mountainDensity === p.density && mountainHeight === p.height
              && mountainSpread === p.spread && noiseRoughness === p.roughness;
            return (
              <button key={p.label} title={p.label}
                onClick={() => setTerrainParams({
                  density: p.density, height: p.height, spread: p.spread, roughness: p.roughness,
                })}
                style={{
                  padding: "2px 6px", borderRadius: 3, cursor: "pointer", fontSize: 9,
                  border: active ? "1px solid #3a7ac0" : "1px solid #1e2e42",
                  background: active ? "#1a3a5a" : "#0d1219",
                  color: active ? "#c0ddf0" : "#6a8aaa",
                }}>
                {p.label}
              </button>
            );
          })}
        </div>
        {slider("Mountain Density", mountainDensity, 0.1, 1.0, 0.05, setMountainDensity, "Few ridges \u2194 Many ridges")}
        {slider("Mountain Height", mountainHeight, 0.1, 1.0, 0.05, setMountainHeight,
          `Tallest peaks \u2248 ${Math.round((0.35 + mountainHeight * 0.6) * 8848).toLocaleString()} m  (gentle hills \u2194 Himalaya)`)}
        {slider("Mountain Spread", mountainSpread, 0.0, 1.0, 0.05, setMountainSpread, "Narrow peaks \u2194 Wide ranges")}
        {slider("Noise Roughness", noiseRoughness, 0.0, 1.0, 0.05, setNoiseRoughness, "Smooth terrain \u2194 Rough terrain")}
      </div>

      <button onClick={handleGenerateElevation} disabled={simRunning || !step1Done} style={genBtn}>
        Generate Elevation (from terrain)
      </button>
      <button onClick={handleGenerateRidged} disabled={simRunning || !step1Done}
        style={{ ...genBtn, background: "#221a2e", color: "#c8a8e0" }}
        title="Plate-free, world-size-aware: many ridged ranges that scale with the map (best for large worlds)">
        ⛰️ Generate Ridged Cordillera (world-scale)
      </button>
      <button onClick={handleRandomizeTerrain} disabled={simRunning || !step1Done}
        style={{ ...genBtn, background: "#1a2e1a", color: "#9cd09c" }}>
        🎲 Randomize Terrain (new seed)
      </button>
      <button onClick={() => setShowAdjust(!showAdjust)} disabled={simRunning || !step1Done} style={genBtn}>
        {showAdjust ? "\u25B2 Adjust Elevation" : "\u25BC Adjust Elevation"}
      </button>

      {showAdjust && (
        <div style={{ background: "#0a1018", border: "1px solid #2a3a50", borderRadius: 4, padding: 6 }}>
          {slider("Elevation Scale", elevPercent, 25, 200, 5, setElevPercent, `${elevPercent}% of current height`)}
          <label style={{ display: "flex", alignItems: "center", gap: 6, color: "#607090", fontSize: 10, margin: "4px 0" }}>
            <input type="checkbox" checked={lockPeaks} onChange={(e) => setLockPeaks(e.target.checked)} />
            Lock highest peaks (keep above threshold fixed)
          </label>
          {lockPeaks && slider("Lock Threshold", lockThreshold, 0.2, 0.95, 0.05, setLockThreshold,
            `Peaks \u2265 ${Math.round(lockThreshold * 8848)} m stay fixed`)}
          <button onClick={handleScaleElevation} disabled={simRunning || !step1Done}
            style={{ ...genBtn, background: "#2a2440", color: "#c0b0e0", textAlign: "center" }}>
            Apply Scale
          </button>
          <div style={{ color: "#405060", fontSize: 9, marginTop: 2 }}>
            Scales relief on the existing elevation. Lock peaks to lower/raise only the
            lowlands. Re-run later phases (climate, rivers) to reflect changes.
          </div>
        </div>
      )}

      <button onClick={() => setShowShelfDialog(!showShelfDialog)} disabled={simRunning} style={genBtn}>
        {showShelfDialog ? "\u25B2 Shelf Settings" : "\u25BC Generate Shelves"}
      </button>

      {showShelfDialog && (
        <div style={{ background: "#0a1018", border: "1px solid #2a3a50", borderRadius: 4, padding: 6 }}>
          {slider("Shelf Width", shelfWidth, 1, 20, 1, setShelfWidth, "Narrow \u2194 Wide")}
          {slider("Noise Variation", shelfNoise, 0, 1, 0.05, setShelfNoise, "Uniform \u2194 Natural")}
          {slider("Depth Profile", shelfProfile, 0, 1, 0.05, setShelfProfile, "Linear \u2194 Exponential")}
          {slider("Drop-off Width", shelfDropoff, 1, 20, 1, setShelfDropoff, "Abrupt \u2194 Gradual")}
          <button onClick={handleGenerateShelves} disabled={simRunning}
            style={{ ...genBtn, background: "#1a3a50", color: "#80b0d0", textAlign: "center" }}>
            Generate Shelves
          </button>
        </div>
      )}

      <div style={{ color: "#405060", fontSize: 10, marginTop: 2 }}>
        Elevation is generated from the land shape (coast distance + noise ridges),
        fully controlled by the sliders above. Use 🎲 Randomize for a new seed with
        the same settings, or the Elevation paint tool for manual mountains
        (Shift+drag resets to 0 m).
      </div>
    </div>
  );
}
