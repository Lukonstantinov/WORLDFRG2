import { useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { useViewportStore } from "@state/viewportStore";
import { simRunAll, simRunAllFromTerrain, finalizeWorld, saveWorldAs, persistOverlays, getWorldMeta,
  computePolitical, computeEconomy, computeSettlementDevelopment } from "@bridge";
import type { Settlement, RiverData } from "@types";
import { StepLandmass } from "@ui/workflow/StepLandmass";
import { StepElevation } from "@ui/workflow/StepElevation";
import { StepOceanAtmo } from "@ui/workflow/StepOceanAtmo";
import { StepClimate } from "@ui/workflow/StepClimate";
import { StepRivers } from "@ui/workflow/StepRivers";
import { StepSoilResources } from "@ui/workflow/StepSoilResources";
import { StepSettlements } from "@ui/workflow/StepSettlements";
import { StepBiological } from "@ui/workflow/StepBiological";
import { StepPolitical } from "@ui/workflow/StepPolitical";
import { StepEconomy } from "@ui/workflow/StepEconomy";
import { StepToponyms } from "@ui/workflow/StepToponyms";
import { StepWorldCharacteristics } from "@ui/workflow/StepWorldCharacteristics";

const STEP_INFO = [
  { step: 1, label: "Landmass", desc: "Paint your landmasses, load an image template, or generate from plates." },
  // World Characteristics sits AFTER Landmass on purpose. Every one of these
  // knobs is a decision you make ABOUT a map you can already see — where the
  // equator falls across your continents, how far the bands stretch, how hard
  // the seasons bite. Asking for them on an empty canvas meant guessing. They
  // are still upstream of everything that consumes them: elevation ignores them
  // entirely, and their first reader is Ocean & Atmosphere (3).
  // Settings-only — nothing to "generate", so it auto-completes (see
  // StepWorldCharacteristics) and never blocks Continue.
  { step: 0, label: "World Characteristics", desc: "Now that you can see your land: frame the latitudes (equator, band expansion, line proportion) and set the planet — rotation (incl. retrograde), axial tilt, sunlight, greenhouse, eccentricity, dryness. These decide where the wind belts, deserts and seasons land, so set them before Ocean & Atmosphere." },
  { step: 2, label: "Elevation", desc: "Generate terrain height. Mountains, coastlines, sea depth." },
  { step: 3, label: "Ocean & Atmosphere", desc: "Wind belts, ocean currents, temperature, and precipitation." },
  { step: 4, label: "Biomes & Climate", desc: "Classify K\u00F6ppen climate zones from temperature & precipitation." },
  { step: 5, label: "Rivers & Lakes", desc: "Trace rivers downhill and detect lake basins." },
  { step: 6, label: "Soil & Resources", desc: "Soil types, fertility scores, and fishery zones." },
  { step: 7, label: "Settlements", desc: "Find optimal locations for cities, towns, and villages." },
  // Toponyms sits right after Settlements: once cities exist the culture map is
  // active, so rivers/mountains/lakes/regions can be named in local style. It is
  // OPTIONAL and off the linear Continue chain (reached by clicking).
  { step: 12, label: "Toponyms (optional)", desc: "Name rivers, mountains, lakes and regions in the local culture's style. Editable — rename any feature. Needs Rivers (5) + Settlements (7)." },
  { step: 8, label: "Biological-Trade", desc: "Shark & shipworm waters, trade-good belts, trade routes, and the regional trade matrix." },
  { step: 9, label: "Political", desc: "Re-rank settlements by trade power (route centrality + good monopoly) and map their influence." },
  { step: 10, label: "Economy", desc: "Solve the market equilibrium: stock-based prices in grain-equivalent, barter ratios, currency goods, grain & trade wealth, supply chains and chokepoints." },
  // Step 11 (Living Trade / the campaign tick) is no longer part of the Forge
  // wizard — it lives in Chronicle mode (ChroniclePanel). Forge owns generation
  // only; finalizing after Economy hands off to Chronicle.
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
  const { setRivers, setLakes, setSettlements, setEconomy, setSettlementsDeveloped, setMeta } = useWorldStore();
  const meta = useWorldStore((s) => s.meta);
  const setAppMode = useUIStore((s) => s.setAppMode);
  const bioParams = useUIStore((s) => s.bioParams);
  const [seed, setSeed] = useState(() => Math.floor(Math.random() * 999999));
  const [plateCount, setPlateCount] = useState(16);

  const frozen = meta?.frozen === true;
  const canAdvance = (step: number) => stepCompleted[step] === true;

  // The steps present in the Forge wizard, in order (11 removed → 10 then optional
  // 12). Navigation walks this list so the 10→12 gap is skipped cleanly.
  // Linear Continue/Back chain: the mandatory generation steps in order. Toponyms
  // (12) is optional and OFF this chain — it's displayed after Settlements but
  // reached by clicking, so Continue at Settlements goes straight to Biological.
  const stepOrder: number[] = STEP_INFO.map((s) => s.step).filter((s) => s !== 12);

  // Chain the two query-only layers (Political → Economy) the same way their step
  // UIs do, so "Generate Full World" leaves a fully-generated, finalize-ready world.
  const runPoliticalAndEconomy = async (setts: Settlement[], rvs: RiverData[]) => {
    const hubs = setts.map((s) => ({ x: s.x, y: s.y, score: s.score, population: s.population }));
    const riverPts = rvs.map((r) => ({ points: r.points }));
    await computePolitical(hubs, riverPts, bioParams.tradeReach, bioParams.maxCrossing,
      bioParams.desertRoutes, bioParams.economicRegions, bioParams.piracyLevel);
    markStepCompleted(9);
    const econ = await computeEconomy(hubs, riverPts, bioParams.tradeReach, bioParams.maxCrossing,
      bioParams.desertRoutes, bioParams.economicRegions, bioParams.luxuryBias, bioParams.piracyLevel,
      bioParams.tradeSeason, bioParams.calendarMonths);
    setEconomy(econ);
    // Grow settlements by realized trade wealth (mirrors StepEconomy).
    try {
      const developed = await computeSettlementDevelopment(setts);
      if (developed.length > 0) setSettlementsDeveloped(developed);
    } catch { /* keep as-generated populations */ }
    markStepCompleted(10);
    setOverlayVisible("chokepoints", true);
    setOverlayVisible("politicalInfluence", true);
  };

  // The single Forge → Chronicle handoff: finalize (lock + save) the world, then
  // switch into Chronicle to play the campaign on it.
  const finalizeAndPlay = async () => {
    const ok = await lockAndSaveWorld();
    if (ok) setAppMode("chronicle");
  };

  const refreshMeta = async () => {
    const m = await getWorldMeta();
    if (m) setMeta(m);
  };

  // Permanently lock the generated map (terrain/climate/rivers/biomes become
  // read-only forever — there is no unfreeze) and save it to a .worldforge file,
  // then the campaign is played on top of it. Confirm-gated, so cancelling keeps
  // the world editable for further tuning. Returns true if the world was locked.
  const lockAndSaveWorld = async (): Promise<boolean> => {
    if (frozen) return true;
    if (!confirm(
      "Lock and save this map?\n\n" +
      "The map — terrain, climate, rivers and biomes — becomes PERMANENTLY read-only. " +
      "You will not be able to edit or regenerate its geography afterwards. Settlements, " +
      "trade and the campaign are then played on top of the finished map.\n\n" +
      "This cannot be undone. (Cancel to keep editing.)"
    )) return false;
    try {
      await finalizeWorld();
      await refreshMeta();
      // Persist the on-screen overlays into the world, then save it to a file.
      const w = useWorldStore.getState();
      try { await persistOverlays(w.settlements, w.rivers, w.lakes); } catch (e) { console.warn("persist skipped:", e); }
      let path: string | null = null;
      const def = (meta?.name || "world") + ".worldforge";
      try {
        const { save } = await import("@tauri-apps/plugin-dialog");
        const result = await save({ filters: [{ name: "WorldForge", extensions: ["worldforge"] }], defaultPath: def });
        if (result) path = result;
      } catch {
        const input = prompt("Save the locked world to path:", def);
        if (input) path = input;
      }
      if (path) {
        await saveWorldAs(path);
        setStatus("Map locked and saved to " + path + " — now play the campaign.");
      } else {
        setStatus("Map locked. Use Save World to write it to a file.");
      }
      return true;
    } catch (err) {
      setStatus(`Error: ${err}`);
      return false;
    }
  };

  const goNext = () => {
    const i = stepOrder.indexOf(workflowStep);
    if (i >= 0 && i < stepOrder.length - 1) setWorkflowStep(stepOrder[i + 1] as any);
  };

  const goBack = () => {
    // Toponyms (12) is off the linear chain; its Back returns to Settlements.
    if (workflowStep === 12) { setWorkflowStep(7 as any); return; }
    const i = stepOrder.indexOf(workflowStep);
    if (i > 0) setWorkflowStep(stepOrder[i - 1] as any);
  };

  const handleRunAll = async () => {
    if (simRunning) return;
    setSimRunning(true);
    setStatus("Running full world generation (from plates)...");
    try {
      const result = await simRunAll(
        seed, plateCount, terrainParams.mode,
        terrainParams.density, terrainParams.height, terrainParams.spread, terrainParams.roughness,
      );
      setRivers(result.rivers);
      setLakes(result.lakes);
      setSettlements(result.settlements);
      invalidateTiles();
      for (let i = 1; i <= 8; i++) markStepCompleted(i);
      setLandmassSource("plates");
      enableAllOverlays();
      setStatus("Ranking trade powers & solving the economy…");
      // Carry generation all the way through Political + Economy so the world is
      // fully generated and finalize-ready (no mid freeze — that moves to the end).
      await runPoliticalAndEconomy(result.settlements, result.rivers);
      setWorkflowStep(10);
      setStatus(`World complete! ${result.rivers.length} rivers, ${result.settlements.length} settlements — economy built. Finalize to play.`);
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
        seed, terrainParams.mode,
        terrainParams.density, terrainParams.height, terrainParams.spread, terrainParams.roughness,
      );
      setRivers(result.rivers);
      setLakes(result.lakes);
      setSettlements(result.settlements);
      invalidateTiles();
      for (let i = 2; i <= 8; i++) markStepCompleted(i);
      enableAllOverlays();
      setStatus("Ranking trade powers & solving the economy…");
      await runPoliticalAndEconomy(result.settlements, result.rivers);
      setWorkflowStep(10);
      setStatus(`World complete! ${result.rivers.length} rivers, ${result.settlements.length} settlements — economy built. Finalize to play.`);
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

      {/* Generate Full World (from plates) — geography work, blocked once frozen */}
      <button onClick={handleRunAll} disabled={simRunning || frozen}
        title={frozen ? "World is finalized — unfreeze to regenerate geography" : undefined}
        style={{ ...genBtn, background: "#1a5a2a", border: "1px solid #2a7040", color: "#a0e0b0", fontWeight: 600, textAlign: "center", opacity: frozen ? 0.5 : 1 }}>
        {simRunning ? "Generating..." : "Generate Full World"}
      </button>

      {/* Generate from existing template/painted landmass */}
      {(hasTemplate || stepCompleted[1]) && (
        <button onClick={handleRunFromTemplate} disabled={simRunning || frozen}
          title={frozen ? "World is finalized — unfreeze to regenerate geography" : undefined}
          style={{ ...genBtn, background: "#1a4a5a", border: "1px solid #2a6070", color: "#a0d0e0", fontWeight: 600, textAlign: "center", opacity: frozen ? 0.5 : 1 }}>
          {simRunning ? "Generating..." : "Complete from Landmass"}
        </button>
      )}

      <div style={{ borderTop: "1px solid #1a2a40", margin: "2px 0" }} />

      {/* Steps \u2014 Geography group (1, 0, 2-6) then Detail group (7-10):
          settlements, trade goods, political & economy. ALL are generation and run
          in Forge; finalizing after Economy hands the finished world to Chronicle.
          NOTE the display order: Landmass (1) comes first, then the settings-only
          World Characteristics (0), then Elevation (2). Step 0 renders a \u2699\ufe0f
          rather than a number precisely so it can sit out of numeric order without
          reading as a mistake \u2014 and so its id (and everyone's persisted
          stepCompleted map) never had to be renumbered. */}
      {STEP_INFO.map(({ step, label, desc }) => {
        const isActive = workflowStep === step;
        const isDone = stepCompleted[step] === true;
        // Only geography steps (1-6) lock once the world is finalized (frozen);
        // every generation step is otherwise available in Forge.
        const locked = step <= 6 && frozen;

        return (
          <div key={step}>
          {step === 7 && (
            <div style={{ borderTop: "1px solid #1a2a40", margin: "6px 0 4px", paddingTop: 6 }}>
              <div style={{ color: "#3a80c0", fontWeight: 700, fontSize: 12, marginBottom: 3 }}>
                Settlements, Trade &amp; Economy
              </div>
              <div style={{ color: "#506080", fontSize: 10, marginBottom: 4 }}>
                People, goods and the market \u2014 all generated here in Forge. Finalize
                after Economy to lock the map and play the campaign in Chronicle.
              </div>
            </div>
          )}
          <div style={{
            border: isActive ? "1px solid #1e3a58" : "1px solid transparent",
            borderRadius: 5, background: isActive ? "#0e1824" : "transparent",
            padding: isActive ? "6px" : "4px 6px",
            opacity: locked ? 0.45 : 1,
          }}>
            {/* Step header */}
            <div onClick={() => {
                if (simRunning) return;
                if (locked) { setStatus("World is finalized — unfreeze to edit geography (steps 1-6)."); return; }
                setWorkflowStep(step as any);
              }}
              style={{
                cursor: locked ? "not-allowed" : "pointer", display: "flex", alignItems: "center", gap: 4,
                color: isActive ? "#c0d8f0" : isDone ? "#60a060" : "#607090",
                fontWeight: isActive ? 600 : 400, fontSize: 12,
              }}>
              <span style={{ minWidth: 16 }}>{isDone ? "\u2713" : locked ? "\ud83d\udd12" : step === 0 ? "\u2699\ufe0f" : `${step}.`}</span>
              <span>{label}</span>
            </div>

            {/* Expanded content */}
            {isActive && (
              <div style={{ marginTop: 6 }}>
                <div style={{ color: "#506080", fontSize: 11, marginBottom: 6 }}>{desc}</div>

                {step === 0 && <StepWorldCharacteristics />}
                {step === 1 && <StepLandmass {...stepProps} />}
                {step === 2 && <StepElevation {...stepProps} />}
                {step === 3 && <StepOceanAtmo {...stepProps} />}
                {step === 4 && <StepClimate {...stepProps} />}
                {step === 5 && <StepRivers {...stepProps} />}
                {step === 6 && <StepSoilResources {...stepProps} />}
                {step === 7 && <StepSettlements {...stepProps} />}
                {step === 8 && <StepBiological {...stepProps} />}
                {step === 9 && <StepPolitical {...stepProps} />}
                {step === 12 && <StepToponyms />}
                {step === 10 && <StepEconomy {...stepProps} />}

                {/* Navigation. Economy (10) and the optional Toponyms (12) are the
                    terminal generation steps \u2014 there the CTA is Finalize & Play
                    (needs the economy built), which locks the world and enters
                    Chronicle. Every other step just continues. */}
                <div style={{ display: "flex", gap: 4, marginTop: 6 }}>
                  {step !== stepOrder[0] && (
                    <button onClick={goBack} disabled={simRunning} style={navBtn}>
                      \u2190 Back
                    </button>
                  )}
                  <div style={{ flex: 1 }} />
                  {step !== 10 && step !== 12 ? (
                    <button onClick={goNext} disabled={!canAdvance(step) || simRunning}
                      style={{ ...navBtn, background: canAdvance(step) ? "#2a5080" : "#1a2a40",
                        color: canAdvance(step) ? "#fff" : "#405060" }}>
                      Continue \u2192
                    </button>
                  ) : (
                    <button onClick={finalizeAndPlay} disabled={simRunning || !canAdvance(10)}
                      title={canAdvance(10)
                        ? "Lock & save the finished world, then play the campaign in Chronicle"
                        : "Build the Economy (step 10) first"}
                      style={{ ...navBtn, background: canAdvance(10) ? "#2a6a3a" : "#1a2a40",
                        color: canAdvance(10) ? "#a0e0b0" : "#405060", fontWeight: 600 }}>
                      \ud83d\udd12 Finalize &amp; Play \u2192
                    </button>
                  )}
                </div>
              </div>
            )}
          </div>
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
