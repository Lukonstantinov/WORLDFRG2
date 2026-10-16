import { useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore, decodeProvinceRaster } from "@state/worldStore";
import { useViewportStore } from "@state/viewportStore";
import { simRunAll, simRunAllFromTerrain, finalizeWorld, saveWorldAs, persistOverlays, getWorldMeta, getProvinceLayer,
  computePolitical, computeEconomy, computeSettlementDevelopment,
  getPlanetConfig, setPlanetConfig, setLatitudeConfig, setLandmassAxes } from "@bridge";
import type { Settlement, RiverData } from "@types";
import { genBtn } from "@ui/workflow/workflowStyles";
import { WORLD_PRESETS, worldPresetPlanetKnobs, type WorldPreset } from "@ui/workflow/worldPresets";
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

/** GENERATOR_AND_GOODS_REDESIGN.md §8 — six stages + a report, grouped over the
 *  OLD step ids (kept so saved progress, staleness and every step's own checks
 *  are untouched). A stage lists its parts; `step` is the id the part completes
 *  and selects (layer/tool defaults); `need` marks parts the stage needs before
 *  Continue. Step 0 (planet settings) is split: the map frame belongs with the
 *  land you judge it against, the physics with the climate it drives. Goods (8)
 *  now come BEFORE Settlements (7), so cities can be founded where resources are.
 *  Step 11 (the campaign tick) lives in Chronicle mode, not here. */
type StagePart = { key: string; step: number; label: string; need: boolean; desc: string };
const STAGES: { id: string; label: string; uses: string; makes: string; parts: StagePart[] }[] = [
  { id: "land", label: "World & Land",
    uses: "your painting, an image, or tectonic plates",
    makes: "land and sea, plates, where the equator sits",
    parts: [
      { key: "land", step: 1, label: "Land", need: true, desc: "Paint your landmasses, load an image template, or generate them from plates." },
      { key: "frame", step: 0, label: "Map frame", need: false, desc: "Place the equator and poles across your land." },
    ] },
  { id: "relief", label: "Relief",
    uses: "land and plates",
    makes: "heights, mountain ranges, coasts, sea depth, shelves",
    parts: [{ key: "elev", step: 2, label: "Elevation", need: true, desc: "Generate terrain height: mountains, coastlines, sea depth." }] },
  { id: "climate", label: "Climate",
    uses: "relief and the planet settings",
    makes: "winds, currents, temperature, rain, seasons, climate zones",
    parts: [
      { key: "planet", step: 0, label: "Planet", need: false, desc: "Tilt, rotation, sunlight, greenhouse, dryness. All default to Earth." },
      { key: "air", step: 3, label: "Winds, sea & rain", need: true, desc: "Wind belts, ocean currents, temperature and rainfall." },
      { key: "zones", step: 4, label: "Climate zones", need: true, desc: "Sort every place into a Köppen climate zone from its temperature and rain." },
    ] },
  { id: "water", label: "Water & Life",
    uses: "relief and climate",
    makes: "rivers, lakes, soils, fertility, fisheries, biomes",
    parts: [
      { key: "rivers", step: 5, label: "Rivers & lakes", need: true, desc: "Trace rivers downhill and find lake basins." },
      { key: "soil", step: 6, label: "Soils & biomes", need: true, desc: "Soil types, farmland quality, fisheries and the living biomes." },
    ] },
  { id: "resources", label: "Resources",
    uses: "climate, soils, biomes and plates",
    makes: "where each good grows, ore deposits, sea hazards",
    parts: [{ key: "goods", step: 8, label: "Goods", need: true, desc: "Place trade goods and ore deposits. This runs before the cities, so towns can grow where the resources are." }] },
  { id: "people", label: "People",
    uses: "everything above",
    makes: "cultures, cities, provinces, place names, trade powers",
    parts: [
      { key: "towns", step: 7, label: "Settlements", need: true, desc: "Found cities, towns and villages where people can live, and divide the land into provinces." },
      { key: "names", step: 12, label: "Place names", need: false, desc: "Name rivers, mountains, lakes and regions in the local culture's style. Optional, and every name can be edited." },
      { key: "powers", step: 9, label: "Trade powers", need: true, desc: "Set trade reach and piracy, then rank cities by their trade position." },
    ] },
  { id: "report", label: "Report",
    uses: "cities, goods and routes",
    makes: "prices, trade flows and chokepoints; the world is ready to play",
    parts: [{ key: "economy", step: 10, label: "Trade report", need: true, desc: "Solve the market: local prices, who trades what with whom, and the chokepoints." }] },
];
/** The stage that holds a step (step 0 resolves to its first home, World & Land). */
const stageOfStep = (step: number) => Math.max(0, STAGES.findIndex((s) => s.parts.some((p) => p.step === step)));

export function WorkflowPanel() {
  const workflowStep = useUIStore((s) => s.workflowStep);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const staleSteps = useUIStore((s) => s.staleSteps);
  const setWorkflowStep = useUIStore((s) => s.setWorkflowStep);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const landmassSource = useUIStore((s) => s.landmassSource);
  const setLandmassSource = useUIStore((s) => s.setLandmassSource);
  const wizardMode = useUIStore((s) => s.wizardMode);
  const setWizardMode = useUIStore((s) => s.setWizardMode);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const terrainParams = useUIStore((s) => s.terrainParams);
  const setTerrainParams = useUIStore((s) => s.setTerrainParams);
  const invalidateTiles = useViewportStore((s) => s.invalidateTiles);
  const { setRivers, setLakes, setSettlements, setEconomy, setSettlementsDeveloped, setMeta, setProvinces } = useWorldStore();
  const meta = useWorldStore((s) => s.meta);
  const setAppMode = useUIStore((s) => s.setAppMode);
  const bioParams = useUIStore((s) => s.bioParams);
  const [seed, setSeed] = useState(() => Math.floor(Math.random() * 999999));
  const [plateCount, setPlateCount] = useState(16);
  // GENERATION_UX_REDESIGN_PLAN.md Slice 5 — world presets. `selectedPreset`
  // is display-only (which chip reads highlighted); applying a preset writes
  // straight into the SAME state Generate already reads (terrainParams,
  // metadata via setLandmassAxes/setPlanetConfig), so it composes with every
  // control below rather than being a separate code path.
  const [selectedPreset, setSelectedPreset] = useState<string | null>(null);
  const [presetBusy, setPresetBusy] = useState(false);

  const applyWorldPreset = async (p: WorldPreset) => {
    setPresetBusy(true);
    try {
      setSelectedPreset(p.id);
      // Landmass axes + elevation model/sliders — only the fields the preset
      // actually names, per rule: a preset only sets the axes it is about.
      await setLandmassAxes(p.oceanFraction, p.continentGoal);
      // A preset's ocean fraction/continent goal only take effect on a
      // from-plates generation — force the "regenerate the landmass too"
      // checkbox on so the preset the user just picked actually applies,
      // rather than silently being ignored by "Complete from Landmass".
      if ((p.oceanFraction !== undefined || p.continentGoal !== undefined) && landmassExists) {
        setRegenerateLandmassToo(true);
      }
      const terrainPatch: Partial<typeof terrainParams> = {};
      if (p.elevMode !== undefined) terrainPatch.mode = p.elevMode;
      if (p.elevDensity !== undefined) terrainPatch.density = p.elevDensity;
      if (p.elevHeight !== undefined) terrainPatch.height = p.elevHeight;
      if (p.elevSpread !== undefined) terrainPatch.spread = p.elevSpread;
      if (p.elevRoughness !== undefined) terrainPatch.roughness = p.elevRoughness;
      if (Object.keys(terrainPatch).length > 0) setTerrainParams(terrainPatch);

      // Planetary knobs — reuses the exact ARCHETYPES span + archetypeAt
      // interpolation StepWorldCharacteristics already applies by hand.
      const knobs = worldPresetPlanetKnobs(p);
      if (knobs) {
        const current = await getPlanetConfig();
        const next = {
          ...current,
          rotationRate: knobs.rotationRate ?? current.rotationRate,
          solarLum: knobs.solarLum ?? current.solarLum,
          greenhouse: knobs.greenhouse ?? current.greenhouse,
          eccentricity: knobs.eccentricity ?? current.eccentricity,
          dryness: knobs.dryness ?? current.dryness,
        };
        await setPlanetConfig(next);
        if (knobs.obliquity !== undefined || knobs.equatorOffset !== undefined || knobs.latScale !== undefined) {
          const lc = useWorldStore.getState().latConfig;
          const eq = knobs.equatorOffset ?? lc.equatorOffset;
          const sc = knobs.latScale ?? lc.latScale;
          const tilt = knobs.obliquity ?? (meta?.obliquity ?? 23.44);
          useWorldStore.getState().setLatConfig(eq, sc, lc.lineRatio);
          const m = await setLatitudeConfig(eq, sc, lc.lineRatio, tilt);
          setMeta(m);
        }
      }
      setStatus(`Preset "${p.label}" applied — press Generate World.`);
    } catch (err) {
      setStatus(`Error applying preset: ${err}`);
    }
    setPresetBusy(false);
  };

  const frozen = meta?.frozen === true;
  const canAdvance = (step: number) => stepCompleted[step] === true;


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

  // ── Stage navigation (GENERATOR_AND_GOODS_REDESIGN.md §8) ──
  // Which part of the open stage is showing. Step 0 lives in TWO stages (map frame
  // and planet), so the part key, not the step id, says which stage is open; it is
  // ignored once something else (a run-all, a click elsewhere) moves the step.
  const [partKey, setPartKey] = useState<string | null>(null);
  const keyStage = partKey
    ? STAGES.findIndex((s) => s.parts.some((p) => p.key === partKey && p.step === workflowStep))
    : -1;
  const activeStage = keyStage >= 0 ? keyStage : stageOfStep(workflowStep);
  const stageDone = (i: number) => STAGES[i].parts.every((p) => !p.need || stepCompleted[p.step] === true);
  const stageStale = (i: number) => STAGES[i].parts.some((p) => stepCompleted[p.step] && staleSteps[p.step]);
  // Geography (stages 1-4 = steps 0-6) locks once the world is finalized.
  const stageLocked = (i: number) => frozen && STAGES[i].parts.every((p) => p.step <= 6);
  const openPart = (p: StagePart) => { setPartKey(p.key); setWorkflowStep(p.step as any); };
  /** Open a stage on the first part it still needs, else its first part. */
  const openStage = (i: number) => {
    const st = STAGES[i];
    if (!st) return;
    openPart(st.parts.find((p) => p.need && stepCompleted[p.step] !== true) ?? st.parts.find((p) => p.need) ?? st.parts[0]);
  };
  const goNext = () => openStage(activeStage + 1);
  const goBack = () => openStage(activeStage - 1);

  /// Both run-alls generate provinces on the Rust side
  /// (`generate_and_persist_provinces`, called from `sim_run_all` and
  /// `sim_run_all_from_terrain`) — but neither handler ever loaded the result
  /// into the store, so a fully generated world reported "No provinces yet" in
  /// the Toolbar and the Provinces panel while 900-odd provinces sat persisted
  /// in metadata. The data was there the whole time; only the frontend did not
  /// know. Read it back exactly the way `App.tsx` does on world open.
  const loadProvincesIntoStore = async () => {
    try {
      const pl = await getProvinceLayer();
      if (pl.provinces?.length) setProvinces(pl.provinces, decodeProvinceRaster(pl));
    } catch { /* province layer absent — leave the store as it was */ }
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
      await loadProvincesIntoStore();
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
      await loadProvincesIntoStore();
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

  // A finished world opens on a clean map: water and cities only. Everything else
  // (trade routes, chokepoints, lat lines…) is one click away in the Toolbar.
  const enableAllOverlays = () => {
    setOverlayVisible("rivers", true);
    setOverlayVisible("lakes", true);
    setOverlayVisible("settlements", true);
  };

  const stepProps = { seed, plateCount, invalidateTiles };

  const hasTemplate = landmassSource === "template" || landmassSource === "painted";
  // GENERATION_UX_REDESIGN_PLAN.md Slice 4 — "Generate Full World" and
  // "Complete from Landmass" merge into ONE primary button whose behaviour is
  // decided by `landmassSource`, stated in a line of text rather than by
  // making the user pick between two buttons that differed only by colour.
  // The one genuine choice — regenerate a painted/templated landmass too —
  // is a small checkbox, shown only when there is a landmass to keep.
  const landmassExists = hasTemplate || stepCompleted[1];
  const [regenerateLandmassToo, setRegenerateLandmassToo] = useState(false);
  const willUsePlates = !landmassExists || landmassSource === "plates" || regenerateLandmassToo;
  const handleGenerate = () => (willUsePlates ? handleRunAll() : handleRunFromTemplate());

  return (
    <div style={{
      width: 220, background: "#18140e", borderRight: "1px solid #352e21",
      padding: "8px", overflowY: "auto", display: "flex", flexDirection: "column",
      gap: 4, fontSize: 12,
    }}>
      <div style={{ color: "#a58855", fontWeight: 700, fontSize: 13, marginBottom: 2 }}>
        World Generation
      </div>

      {/* GENERATION_UX_REDESIGN_PLAN.md Slice 9 (F9) — the two-mode head.
          Purely a display choice: both modes read/write the same state, so
          switching never loses anything a preset or a step already set. */}
      <div style={{ display: "flex", gap: 3, marginBottom: 2 }}>
        {(["quick", "detailed"] as const).map((m) => (
          <button key={m} onClick={() => setWizardMode(m)}
            style={{
              flex: 1, fontSize: 10.5, padding: "3px 0", borderRadius: 4, cursor: "pointer",
              border: `1px solid ${wizardMode === m ? "#c09b5a" : "#483d2b"}`,
              background: wizardMode === m ? "#3f3421" : "#221c12",
              color: wizardMode === m ? "#cfe2f6" : "#a28f6e", fontWeight: wizardMode === m ? 600 : 400,
            }}>
            {m === "quick" ? "Quick" : "Detailed"}
          </button>
        ))}
      </div>

      {/* GENERATION_UX_REDESIGN_PLAN.md Slice 5 — world presets: pick one,
          press Generate World below, get a whole world. */}
      <div style={{ opacity: frozen ? 0.5 : 1, pointerEvents: frozen ? "none" : undefined }}>
        <div style={{ color: "#ac9d84", fontSize: 10.5, marginBottom: 3 }}>Presets</div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 3 }}>
          {WORLD_PRESETS.map((p) => (
            <button key={p.id} onClick={() => applyWorldPreset(p)} disabled={presetBusy}
              title={p.blurb}
              style={{
                fontSize: 9.5, padding: "3px 2px", borderRadius: 4, cursor: presetBusy ? "wait" : "pointer",
                textAlign: "center", lineHeight: 1.25,
                border: `1px solid ${selectedPreset === p.id ? "#c09b5a" : "#483d2b"}`,
                background: selectedPreset === p.id ? "#3f3421" : "#252016",
                color: selectedPreset === p.id ? "#cfe2f6" : "#ac9d84",
              }}>
              {p.icon} {p.label}
            </button>
          ))}
        </div>
        {selectedPreset && (
          <div style={{ fontSize: 9, color: "#897a61", marginTop: 4, lineHeight: 1.35 }}>
            {WORLD_PRESETS.find((p) => p.id === selectedPreset)?.expect}
          </div>
        )}
      </div>

      <div style={{ borderTop: "1px solid #3b311f", margin: "2px 0" }} />

      {/* Seed & Plates */}
      <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
        <label style={{ color: "#8a7d66", fontSize: 11, minWidth: 30 }}>Seed</label>
        <input type="number" value={seed} onChange={(e) => setSeed(Number(e.target.value))}
          style={inputStyle} />
        <button onClick={() => setSeed(Math.floor(Math.random() * 999999))}
          style={{ ...smallBtn, width: "auto", padding: "2px 6px" }}>New</button>
      </div>
      <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
        <label style={{ color: "#8a7d66", fontSize: 11, minWidth: 30 }}>Plates</label>
        <input type="number" value={plateCount} min={4} max={40}
          onChange={(e) => setPlateCount(Number(e.target.value))} style={inputStyle} />
      </div>

      {/* GENERATION_UX_REDESIGN_PLAN.md Slice 4 — one primary Generate button. */}
      <button onClick={handleGenerate} disabled={simRunning || frozen}
        title={frozen ? "World is finalized — unfreeze to regenerate geography" : undefined}
        style={{
          ...genBtn,
          background: willUsePlates ? "#1a5a2a" : "#4d3f27",
          border: `1px solid ${willUsePlates ? "#2a7040" : "#2a6070"}`,
          color: willUsePlates ? "#a0e0b0" : "#dbc7a5",
          fontWeight: 600, textAlign: "center", opacity: frozen ? 0.5 : 1,
        }}>
        {simRunning ? "Generating..." : "Generate World"}
      </button>
      <div style={{ fontSize: 9.5, color: "#897a61", lineHeight: 1.4, margin: "-1px 0 2px" }}>
        {willUsePlates
          ? "Generating from plates — your landmass will be regenerated."
          : "Generating from your painted landmass — your coastlines are kept."}
      </div>
      {landmassExists && landmassSource !== "plates" && (
        <label style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 10.5, color: "#ac9d84", margin: "0 0 2px" }}>
          <input type="checkbox" checked={regenerateLandmassToo}
            onChange={(e) => setRegenerateLandmassToo(e.target.checked)}
            style={{ accentColor: "#c09b5a", width: 12, height: 12 }} />
          Regenerate the landmass too (from plates)
        </label>
      )}

      <div style={{ borderTop: "1px solid #3b311f", margin: "2px 0" }} />

      {/* GENERATION_UX_REDESIGN_PLAN.md Slice 9 \u2014 the Quick-mode summary:
          no 13-step list, just where the world stands and the one button
          that matters next. Switching to Detailed shows the full wizard on
          the SAME state, so nothing is lost by starting here. */}
      {wizardMode === "quick" && (
        <div>
          {canAdvance(10) ? (
            <>
              <div style={{ fontSize: 10.5, color: "#60a060", marginBottom: 4 }}>
                \u2713 World generated \u2014 settlements, trade and economy are built.
              </div>
              <button onClick={finalizeAndPlay} disabled={simRunning}
                style={{ ...navBtn, width: "100%", background: "#2a6a3a", color: "#a0e0b0", fontWeight: 600, padding: "6px 0" }}>
                \ud83d\udd12 Finalize &amp; Play \u2192
              </button>
            </>
          ) : (
            <div style={{ fontSize: 10.5, color: "#897a61", lineHeight: 1.4 }}>
              Pick a preset above (or just press Generate World) for a complete,
              playable world \u2014 settlements, trade routes and economy included.
            </div>
          )}
          <button onClick={() => setWizardMode("detailed")}
            style={{ ...navBtn, width: "100%", marginTop: 6, background: "transparent", color: "#a28f6e" }}>
            Switch to Detailed for step-by-step control \u2192
          </button>
        </div>
      )}

      {/* The stage wizard: six stages and a report, each showing what it USES
          and what it MAKES, its parts as tabs, and a re-run chip when an earlier
          stage changed. Old step ids underneath, so nothing saved is renumbered. */}
      {wizardMode === "detailed" && STAGES.map((stage, i) => {
        const isActive = activeStage === i;
        const isDone = stageDone(i);
        const locked = stageLocked(i);
        const current = stage.parts.find((p) => p.key === partKey && p.step === workflowStep)
          ?? stage.parts.find((p) => p.step === workflowStep) ?? stage.parts[0];
        const isReport = stage.id === "report";
        return (
          <div key={stage.id}>
          <div style={{
            border: isActive ? "1px solid #4e4028" : "1px solid transparent",
            borderRadius: 5, background: isActive ? "#211b11" : "transparent",
            padding: isActive ? "6px" : "4px 6px",
            opacity: locked ? 0.45 : 1,
          }}>
            <div onClick={() => {
                if (simRunning) return;
                if (locked) { setStatus("World is finalized: its geography can no longer be changed."); return; }
                openStage(i);
              }}
              role="button" tabIndex={0}
              onKeyDown={(e) => { if ((e.key === "Enter" || e.key === " ") && !simRunning && !locked) openStage(i); }}
              style={{
                cursor: locked ? "not-allowed" : "pointer", display: "flex", alignItems: "center", gap: 5,
                color: isActive ? "#c0d8f0" : isDone ? "#60a060" : "#8a7d66",
                fontWeight: isActive ? 600 : 400, fontSize: 12,
              }}>
              <span style={{ minWidth: 16 }}>{isDone ? "✓" : locked ? "🔒" : isReport ? "R" : `${i + 1}.`}</span>
              <span>{stage.label}</span>
              {isDone && stageStale(i) && (
                <span title="An earlier stage changed since this ran. Re-run it to update (you can still continue)."
                  style={{ marginLeft: "auto", fontSize: 9, padding: "0 5px", borderRadius: 3,
                    color: "#1a1206", background: "#d9a441", fontWeight: 600 }}>{"↻"} re-run</span>
              )}
            </div>

            {isActive && (
              <div style={{ marginTop: 6 }}>
                <div style={{ fontSize: 10, color: "#8a7d66", lineHeight: 1.45, marginBottom: 6 }}>
                  <div><b style={{ color: "#b5a27c" }}>Uses:</b> {stage.uses}</div>
                  <div><b style={{ color: "#b5a27c" }}>Makes:</b> {stage.makes}</div>
                </div>
                {stage.parts.length > 1 && (
                  <div role="tablist" style={{ display: "flex", flexWrap: "wrap", gap: 3, marginBottom: 6 }}>
                    {stage.parts.map((p) => {
                      const on = p.key === current.key;
                      const done = stepCompleted[p.step] === true && p.step !== 0;
                      return (
                        <button key={p.key} role="tab" aria-selected={on} disabled={simRunning}
                          onClick={() => openPart(p)}
                          style={{
                            fontSize: 10, padding: "2px 7px", borderRadius: 999, cursor: "pointer",
                            border: `1px solid ${on ? "#c09b5a" : "#483d2b"}`,
                            background: on ? "#3f3421" : "transparent",
                            color: on ? "#f0dcaa" : done ? "#7fb07f" : "#a28f6e",
                          }}>
                          {done ? "✓ " : ""}{p.label}{p.need ? "" : " (optional)"}
                        </button>
                      );
                    })}
                  </div>
                )}
                <div style={{ color: "#7a6d56", fontSize: 11, marginBottom: 6 }}>{current.desc}</div>

                {current.key === "land" && <StepLandmass {...stepProps} />}
                {current.key === "frame" && <StepWorldCharacteristics part="frame" />}
                {current.key === "elev" && <StepElevation {...stepProps} />}
                {current.key === "planet" && <StepWorldCharacteristics part="planet" />}
                {current.key === "air" && <StepOceanAtmo {...stepProps} />}
                {current.key === "zones" && <StepClimate {...stepProps} />}
                {current.key === "rivers" && <StepRivers {...stepProps} />}
                {current.key === "soil" && <StepSoilResources {...stepProps} />}
                {current.key === "goods" && <StepBiological {...stepProps} />}
                {current.key === "towns" && <StepSettlements {...stepProps} />}
                {current.key === "names" && <StepToponyms />}
                {current.key === "powers" && <StepPolitical {...stepProps} />}
                {current.key === "economy" && <StepEconomy {...stepProps} />}

                {/* Continue moves to the next stage once this one has everything it
                    needs; the Report ends in Finalize & Play. */}
                <div style={{ display: "flex", gap: 4, marginTop: 6 }}>
                  {i > 0 && (
                    <button onClick={goBack} disabled={simRunning} style={navBtn}>
                      {"←"} Back
                    </button>
                  )}
                  <div style={{ flex: 1 }} />
                  {!isReport ? (
                    <button onClick={goNext} disabled={!isDone || simRunning}
                      title={isDone ? undefined : "Finish the required parts of this stage first"}
                      style={{ ...navBtn, background: isDone ? "#705c3a" : "#3b311f",
                        color: isDone ? "#fff" : "#5c5344" }}>
                      Continue {"→"}
                    </button>
                  ) : (
                    <button onClick={finalizeAndPlay} disabled={simRunning || !canAdvance(10)}
                      title={canAdvance(10)
                        ? "Lock & save the finished world, then play the campaign in Chronicle"
                        : "Build the trade report first"}
                      style={{ ...navBtn, background: canAdvance(10) ? "#2a6a3a" : "#3b311f",
                        color: canAdvance(10) ? "#a0e0b0" : "#5c5344", fontWeight: 600 }}>
                      {"🔒"} Finalize &amp; Play {"→"}
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
  flex: 1, padding: "3px 6px", background: "#110e09", border: "1px solid #3e3422",
  borderRadius: 4, color: "#cec4b2", fontSize: 11, outline: "none",
};

const smallBtn: React.CSSProperties = {
  padding: "4px 8px", borderRadius: 4, border: "1px solid #3b311f",
  background: "#262017", color: "#a5916f", cursor: "pointer", fontSize: 11,
};

const navBtn: React.CSSProperties = {
  padding: "4px 10px", borderRadius: 4, border: "1px solid #3b311f",
  background: "#262017", color: "#a5916f", cursor: "pointer", fontSize: 11,
};

// genBtn now lives in a leaf module (workflowStyles) to break the WorkflowPanel↔Step*
// import cycle; imported above for local use and re-exported here so existing
// `import { genBtn } from "@ui/workflow/WorkflowPanel"` callers keep working.
export { genBtn };
