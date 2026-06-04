import { useState } from "react";
import { useUIStore } from "../../state/uiStore";
import { useWorldStore } from "../../state/worldStore";
import { useViewportStore } from "../../state/viewportStore";
import { simRunAll, simRunAllFromTerrain } from "../../bridge/tauri";
import { StepLandmass } from "./StepLandmass";
import { StepElevation } from "./StepElevation";
import { StepOceanAtmo } from "./StepOceanAtmo";
import { StepClimate } from "./StepClimate";
import { StepRivers } from "./StepRivers";
import { StepSoilResources } from "./StepSoilResources";
import { StepSettlements } from "./StepSettlements";
import { StepBiological } from "./StepBiological";
import { StepPolitical } from "./StepPolitical";
import { StepEconomy } from "./StepEconomy";

const STEP_INFO = [
  { step: 1, label: "Landmass", desc: "Paint your landmasses, load an image template, or generate from plates." },
  { step: 2, label: "Elevation", desc: "Generate terrain height. Mountains, coastlines, sea depth." },
  { step: 3, label: "Ocean & Atmosphere", desc: "Wind belts, ocean currents, temperature, and precipitation." },
  { step: 4, label: "Biomes & Climate", desc: "Classify K\u00F6ppen climate zones from temperature & precipitation." },
  { step: 5, label: "Rivers & Lakes", desc: "Trace rivers downhill and detect lake basins." },
  { step: 6, label: "Soil & Resources", desc: "Soil types, fertility scores, and fishery zones." },
  { step: 7, label: "Settlements", desc: "Find optimal locations for cities, towns, and villages." },
  { step: 8, label: "Biological-Trade", desc: "Shark & shipworm waters, trade-good belts, trade routes, and the regional trade matrix." },
  { step: 9, label: "Political", desc: "Re-rank settlements by trade power (route centrality + good monopoly) and map their influence." },
  { step: 10, label: "Economy", desc: "Build the trade economy: quality-graded production, per-hop prices, hub wealth, supply chains and strategic chokepoints." },
] as const;

export function WorkflowPanel() {
  const workflowStep = useUIStore((s) => s.workflowStep);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const setWorkflowStep = useUIStore((s) => s.setWorkflowStep);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const landmassSource = useUIStore((s) => s.landmassSource);
  const setLandmassSource = useUIStore((s) => s.setLandmassSource);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const terrainParams = useUIStore((s) => s.terrainParams);
  const invalidateTiles = useViewportStore((s) => s.invalidateTiles);
  const { setRivers, setLakes, setSettlements } = useWorldStore();
  const [seed, setSeed] = useState(() => Math.floor(Math.random() * 999999));
  const [plateCount, setPlateCount] = useState(16);

  const canAdvance = (step: number) => stepCompleted[step] === true;

  const goNext = () => {
    if (workflowStep < 10) {
      setWorkflowStep((workflowStep + 1) as any);
    }
  };

  const goBack = () => {
    if (workflowStep > 1) {
      setWorkflowStep((workflowStep - 1) as any);
    }
  };

  const handleRunAll = async () => {
    if (simRunning) return;
    setSimRunning(true);
    setStatus("Running full world generation (from plates)...");
    try {
      const result = await simRunAll(seed, plateCount);
      setRivers(result.rivers);
      setLakes(result.lakes);
      setSettlements(result.settlements);
      invalidateTiles();
      for (let i = 1; i <= 8; i++) markStepCompleted(i);
      setLandmassSource("plates");
      setWorkflowStep(8);
      enableAllOverlays();
      setStatus(`World complete! ${result.rivers.length} rivers, ${result.settlements.length} settlements`);
    } catch (err) {
      setStatus(`Error: ${err}`);
    }
    setSimRunning(false);
  };

  const handleRunFromTemplate = async () => {
    if (simRunning) return;
    if (!stepCompleted[1]) {
      setStatus("Load a template or paint landmass first (Step 1)");
      return;
    }
    setSimRunning(true);
    setStatus("Running full generation (keeping your landmass)...");
    try {
      const result = await simRunAllFromTerrain(
        seed, terrainParams.density, terrainParams.height, terrainParams.spread, terrainParams.roughness,
      );
      setRivers(result.rivers);
      setLakes(result.lakes);
      setSettlements(result.settlements);
      invalidateTiles();
      for (let i = 2; i <= 8; i++) markStepCompleted(i);
      setWorkflowStep(8);
      enableAllOverlays();
      setStatus(`World complete! ${result.rivers.length} rivers, ${result.settlements.length} settlements`);
    } catch (err) {
      setStatus(`Error: ${err}`);
    }
    setSimRunning(false);
  };

  const enableAllOverlays = () => {
    setOverlayVisible("rivers", true);
    setOverlayVisible("lakes", true);
    setOverlayVisible("settlements", true);
    setOverlayVisible("latLines", true);
    setOverlayVisible("sharkZones", true);
    setOverlayVisible("tradeRoutes", true);
  };

  const stepProps = { seed, plateCount, invalidateTiles };

  const hasTemplate = landmassSource === "template" || landmassSource === "painted";

  return (
    <div style={{
      width: 220, background: "#0d1219", borderRight: "1px solid #1e2a38",
      padding: "8px", overflowY: "auto", display: "flex", flexDirection: "column",
      gap: 4, fontSize: 12,
    }}>
      <div style={{ color: "#3a80c0", fontWeight: 700, fontSize: 13, marginBottom: 2 }}>
        World Generation
      </div>

      {/* Seed & Plates */}
      <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
        <label style={{ color: "#607090", fontSize: 11, minWidth: 30 }}>Seed</label>
        <input type="number" value={seed} onChange={(e) => setSeed(Number(e.target.value))}
          style={inputStyle} />
        <button onClick={() => setSeed(Math.floor(Math.random() * 999999))}
          style={{ ...smallBtn, width: "auto", padding: "2px 6px" }}>New</button>
      </div>
      <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
        <label style={{ color: "#607090", fontSize: 11, minWidth: 30 }}>Plates</label>
        <input type="number" value={plateCount} min={4} max={40}
          onChange={(e) => setPlateCount(Number(e.target.value))} style={inputStyle} />
      </div>

      {/* Generate Full World (from plates) */}
      <button onClick={handleRunAll} disabled={simRunning}
        style={{ ...genBtn, background: "#1a5a2a", border: "1px solid #2a7040", color: "#a0e0b0", fontWeight: 600, textAlign: "center" }}>
        {simRunning ? "Generating..." : "Generate Full World"}
      </button>

      {/* Generate from existing template/painted landmass */}
      {(hasTemplate || stepCompleted[1]) && (
        <button onClick={handleRunFromTemplate} disabled={simRunning}
          style={{ ...genBtn, background: "#1a4a5a", border: "1px solid #2a6070", color: "#a0d0e0", fontWeight: 600, textAlign: "center" }}>
          {simRunning ? "Generating..." : "Complete from Landmass"}
        </button>
      )}

      <div style={{ borderTop: "1px solid #1a2a40", margin: "2px 0" }} />

      {/* Steps */}
      {STEP_INFO.map(({ step, label, desc }) => {
        const isActive = workflowStep === step;
        const isDone = stepCompleted[step] === true;

        return (
          <div key={step} style={{
            border: isActive ? "1px solid #1e3a58" : "1px solid transparent",
            borderRadius: 5, background: isActive ? "#0e1824" : "transparent",
            padding: isActive ? "6px" : "4px 6px",
          }}>
            {/* Step header */}
            <div onClick={() => !simRunning && setWorkflowStep(step as any)}
              style={{
                cursor: "pointer", display: "flex", alignItems: "center", gap: 4,
                color: isActive ? "#c0d8f0" : isDone ? "#60a060" : "#607090",
                fontWeight: isActive ? 600 : 400, fontSize: 12,
              }}>
              <span style={{ minWidth: 16 }}>{isDone ? "\u2713" : `${step}.`}</span>
              <span>{label}</span>
            </div>

            {/* Expanded content */}
            {isActive && (
              <div style={{ marginTop: 6 }}>
                <div style={{ color: "#506080", fontSize: 11, marginBottom: 6 }}>{desc}</div>

                {step === 1 && <StepLandmass {...stepProps} />}
                {step === 2 && <StepElevation {...stepProps} />}
                {step === 3 && <StepOceanAtmo {...stepProps} />}
                {step === 4 && <StepClimate {...stepProps} />}
                {step === 5 && <StepRivers {...stepProps} />}
                {step === 6 && <StepSoilResources {...stepProps} />}
                {step === 7 && <StepSettlements {...stepProps} />}
                {step === 8 && <StepBiological {...stepProps} />}
                {step === 9 && <StepPolitical {...stepProps} />}
                {step === 10 && <StepEconomy {...stepProps} />}

                {/* Navigation */}
                <div style={{ display: "flex", gap: 4, marginTop: 6 }}>
                  {step > 1 && (
                    <button onClick={goBack} disabled={simRunning} style={navBtn}>
                      \u2190 Back
                    </button>
                  )}
                  <div style={{ flex: 1 }} />
                  {step < 10 ? (
                    <button onClick={goNext} disabled={!canAdvance(step) || simRunning}
                      style={{ ...navBtn, background: canAdvance(step) ? "#2a5080" : "#1a2a40",
                        color: canAdvance(step) ? "#fff" : "#405060" }}>
                      Continue \u2192
                    </button>
                  ) : (
                    <button onClick={() => {}} disabled={!isDone}
                      style={{ ...navBtn, background: isDone ? "#2a6a3a" : "#1a2a40",
                        color: isDone ? "#a0e0b0" : "#405060" }}>
                      Complete \u2713
                    </button>
                  )}
                </div>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

// Shared styles
const inputStyle: React.CSSProperties = {
  flex: 1, padding: "3px 6px", background: "#080c12", border: "1px solid #1e2e42",
  borderRadius: 4, color: "#b0c0d0", fontSize: 11, outline: "none",
};

const smallBtn: React.CSSProperties = {
  padding: "4px 8px", borderRadius: 4, border: "1px solid #1a2a40",
  background: "#151d28", color: "#6a8aaa", cursor: "pointer", fontSize: 11,
};

const navBtn: React.CSSProperties = {
  padding: "4px 10px", borderRadius: 4, border: "1px solid #1a2a40",
  background: "#151d28", color: "#6a8aaa", cursor: "pointer", fontSize: 11,
};

export const genBtn: React.CSSProperties = {
  width: "100%", padding: "6px 8px", borderRadius: 4, border: "1px solid #1a2a40",
  background: "#151d28", color: "#7a98b8", cursor: "pointer", fontSize: 11, textAlign: "left" as const,
};
