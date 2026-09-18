import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { useGoodsStore } from "@state/goodsStore";
import { simBiological } from "@bridge";
import { GOOD_DEFS, goodOverlayKey } from "@goods";
import { genBtn } from "@ui/workflow/WorkflowPanel";
import { GoodsReportPanel } from "@ui/goods/GoodsReportPanel";
import { useState } from "react";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

/*  Trade reach. "Global (cross any ocean)" is gone: an unbroken open-water run is
 *  capped by a WORLD RULE now (`MAX_OPEN_SEA_CROSSING_KM`, query_commands/mod.rs),
 *  which binds at every reach except 2, so no setting can put a trans-oceanic lane
 *  back. Reach 1 may only ask for something SHORTER than the rule.
 *
 *  `SEA_CAP_KM` mirrors that Rust constant for the SLIDER BOUND only — the backend
 *  clamps regardless (gated by `no_caller_can_ask_its_way_across_an_ocean`), so if
 *  the two ever drift the rule still holds and the slider merely stops short. */
const REACH_LABELS = ["Sea trade — short crossings only", "Coastal — custom crossing limit", "Continental only — no sea legs"];
const KM_EQUATOR = 40075;
const SEA_CAP_KM = 800;

export function StepBiological({ seed, invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const setLayer = useUIStore((s) => s.setLayer);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const bioParams = useUIStore((s) => s.bioParams);
  const setBioParams = useUIStore((s) => s.setBioParams);
  const rivers = useWorldStore((s) => s.rivers);
  const openGoodsEditor = useGoodsStore((s) => s.setOpen);
  const loadGoodsFromWorld = useGoodsStore((s) => s.loadFromWorld);
  const goodsSpecs = useGoodsStore((s) => s.specs);
  const applyGoodsToWorld = useGoodsStore((s) => s.applyToWorld);

  const openChainReview = useUIStore((s) => s.openChainReview);
  // The placement report (§8.20). Opened automatically once generation finishes —
  // an absent or fallback-seeded good is exactly the thing the user needs to see at
  // that moment, and it is persisted, so it can be reopened any time afterwards.
  const [reportOpen, setReportOpen] = useState(false);

  const step6Done = stepCompleted[6] === true;
  const step7Done = stepCompleted[7] === true;
  const step8Done = stepCompleted[8] === true;

  // The actual generation, run only after the user confirms in the Chain Review.
  const runGeneration = async () => {
    if (simRunning) return;
    setSimRunning(true);
    setStatus("Computing shark/shipworm waters, trade goods, routes & matrix...");
    try {
      // Snapshot any edited good specs into the world so generation uses them.
      if (goodsSpecs.length > 0) await applyGoodsToWorld();
      await simBiological(seed, JSON.stringify(rivers), bioParams.gemDeposits, bioParams.climateStrictness);
      markStepCompleted(8); // gates the trade-route / flow computation in MapCanvas
      invalidateTiles();    // bumps tileVersion → refetches shark/goods/routes/flows
      // Surface the new overlays.
      setOverlayVisible("sharkZones", true);
      setOverlayVisible("shipwormZones", true);
      setOverlayVisible("stormZones", true);
      setOverlayVisible("reefZones", true);
      setOverlayVisible("tradeRoutes", true);
      setOverlayVisible("tradeFlows", true);
      setStatus("Biological-Trade computed: sharks, shipworms, goods, routes & trade matrix");
      setReportOpen(true);
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  // Goods generation ALWAYS routes through the Goods & Chains review window first
  // (planted vs manufactured + the recipe schematic), then runs on confirm.
  const handleGenerate = async () => {
    if (simRunning) return;
    if (!step6Done) {
      setStatus("Step 6 required: compute Soil & Fertility first (fisheries drive shark/fish goods)");
      return;
    }
    if (!step7Done) {
      setStatus("Step 7 required: place Settlements first (the trade matrix groups them into regions)");
      return;
    }
    if (goodsSpecs.length === 0) await loadGoodsFromWorld();
    openChainReview(() => { void runGeneration(); });
  };

  const goodIds = goodsSpecs.length > 0 ? goodsSpecs.filter((g) => g.enabled).map((g) => g.id) : GOOD_DEFS.map((g) => g.name);
  const enableAllGoods = () => { for (const id of goodIds) setOverlayVisible(goodOverlayKey(id), true); };
  const disableAllGoods = () => { for (const id of goodIds) setOverlayVisible(goodOverlayKey(id), false); };
  const openEditor = () => { void loadGoodsFromWorld(); openGoodsEditor(true); };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step6Done && (
        <div style={warn}>Complete Step 6 first (fisheries feed shark & fish-good belts)</div>
      )}
      {step6Done && !step7Done && (
        <div style={warn}>Complete Step 7 first (the trade matrix needs settlements)</div>
      )}

      {/* Generation parameters */}
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#5a7090" }}>
        <span>Gemstone deposits</span><span style={{ color: "#8aa0c0" }}>{bioParams.gemDeposits}</span>
      </div>
      <input type="range" min={0} max={16} value={bioParams.gemDeposits}
        onChange={(e) => setBioParams({ gemDeposits: Number(e.target.value) })}
        style={{ width: "100%" }} />

      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#5a7090", marginTop: 2 }}>
        <span>Climate strictness</span>
        <span style={{ color: "#8aa0c0" }}>
          {bioParams.climateStrictness < 0.4 ? "diffuse" : bioParams.climateStrictness > 0.6 ? "tight" : "neutral"}
        </span>
      </div>
      <input type="range" min={0} max={100} value={Math.round(bioParams.climateStrictness * 100)}
        onChange={(e) => setBioParams({ climateStrictness: Number(e.target.value) / 100 })}
        style={{ width: "100%" }} />
      <div style={{ fontSize: 9, color: "#5a7090" }}>
        How tightly each good hugs its ideal climate (tight = smaller, more clustered belts).
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#5a7090", marginTop: 2 }}>
        <span>Economic regions</span><span style={{ color: "#8aa0c0" }}>{bioParams.economicRegions}</span>
      </div>
      <input type="range" min={4} max={40} value={bioParams.economicRegions}
        onChange={(e) => setBioParams({ economicRegions: Number(e.target.value) })}
        style={{ width: "100%" }} />
      <div style={{ fontSize: 9, color: "#5a7090" }}>
        Granularity of trade regions &amp; political hubs (also scales route hubs).
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#5a7090", marginTop: 2 }}>
        <span>Demand bias</span>
        <span style={{ color: "#8aa0c0" }}>
          {bioParams.luxuryBias < 0.4 ? "subsistence" : bioParams.luxuryBias > 0.6 ? "mercantile" : "balanced"}
        </span>
      </div>
      <input type="range" min={0} max={100} value={Math.round(bioParams.luxuryBias * 100)}
        onChange={(e) => setBioParams({ luxuryBias: Number(e.target.value) / 100 })}
        style={{ width: "100%" }} />
      <div style={{ fontSize: 9, color: "#5a7090" }}>
        Subsistence worlds trade staples; mercantile worlds prize distant luxuries (silk, spices).
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#5a7090", marginTop: 2 }}>
        <span>Piracy</span>
        <span style={{ color: "#8aa0c0" }}>
          {bioParams.piracyLevel < 0.1 ? "safe seas" : bioParams.piracyLevel > 0.6 ? "infested" : "some raiders"}
        </span>
      </div>
      <input type="range" min={0} max={100} value={Math.round(bioParams.piracyLevel * 100)}
        onChange={(e) => setBioParams({ piracyLevel: Number(e.target.value) / 100 })}
        style={{ width: "100%" }} />
      <div style={{ fontSize: 9, color: "#5a7090" }}>
        Raiders make coastal narrows &amp; straits costlier — trade hugs safe coasts, detours or goes overland.
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#5a7090", marginTop: 2 }}>
        <span>Trade season</span>
        <span style={{ color: "#8aa0c0" }}>
          {bioParams.tradeSeason === 0 ? "all year" : `moon ${bioParams.tradeSeason}/${bioParams.calendarMonths}`}
        </span>
      </div>
      <input type="range" min={0} max={bioParams.calendarMonths} value={bioParams.tradeSeason}
        onChange={(e) => setBioParams({ tradeSeason: Number(e.target.value) })}
        style={{ width: "100%" }} />
      <div style={{ fontSize: 9, color: "#5a7090" }}>
        Seasonal closures: winter snows shut high mountain passes; monsoon/cyclone seas close their sailing windows, so routes detour. Routes redraw live as you slide.
      </div>

      <div style={{ fontSize: 10, color: "#5a7090", marginTop: 2 }}>Trade reach</div>
      <select value={bioParams.tradeReach}
        onChange={(e) => setBioParams({ tradeReach: Number(e.target.value) })}
        style={{ width: "100%", background: "#080c12", color: "#b0c0d0", border: "1px solid #1e2e42", borderRadius: 4, fontSize: 10, padding: "2px 4px" }}>
        {REACH_LABELS.map((l, i) => <option key={i} value={i}>{l}</option>)}
      </select>

      {bioParams.tradeReach === 1 && (
        <>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#5a7090", marginTop: 2 }}>
            <span>Max open-water crossing</span>
            <span style={{ color: "#8aa0c0" }}>{Math.round(bioParams.maxCrossing * KM_EQUATOR)} km</span>
          </div>
          {/* Stated in KM, not "% of map width" — a fraction of the map is not a
              distance anyone can reason about, and reading it as one is how the
              shipped 12% came to mean a 4,809 km ocean traversal. */}
          <input type="range" min={50} max={SEA_CAP_KM} step={25}
            value={Math.round(bioParams.maxCrossing * KM_EQUATOR)}
            onChange={(e) => setBioParams({ maxCrossing: Number(e.target.value) / KM_EQUATOR })}
            style={{ width: "100%" }} />
          <div style={{ fontSize: 9, color: "#5a7090" }}>
            The longest stretch of OPEN water a route may cross without sight of land.
            Coast-hugging and island-hopping are unlimited — shelf and coastal water
            reset the run — so a network still reaches a long way, one hop at a time.
          </div>
        </>
      )}

      <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 10, color: "#8aa0c0", marginTop: 4, cursor: "pointer" }}>
        <input type="checkbox" checked={bioParams.desertRoutes}
          onChange={(e) => setBioParams({ desertRoutes: e.target.checked })} />
        Silk Road caravans (overland steppe &amp; desert)
      </label>
      <div style={{ fontSize: 9, color: "#5a7090", marginTop: 1 }}>
        Trade threads overland steppe corridors and deserts (Silk-Road style) when storms/reefs make the seas perilous.
      </div>

      <button onClick={handleGenerate} disabled={simRunning || !step6Done || !step7Done} style={{ ...genBtn, marginTop: 2 }}>
        Generate Biological Layer
      </button>

      <div style={{ display: "flex", gap: 4, marginTop: 2, flexWrap: "wrap" }}>
        <button onClick={() => setLayer("shark")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Shark</button>
        <button onClick={() => setLayer("shipworm")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Shipworm</button>
        <button onClick={() => setLayer("storm")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Storm</button>
        <button onClick={() => setLayer("reef")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Reef</button>
        <button onClick={() => setLayer("salinity")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Salinity</button>
      </div>

      <div style={{ display: "flex", gap: 4 }}>
        <button onClick={enableAllGoods}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Show all goods</button>
        <button onClick={disableAllGoods}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Hide goods</button>
      </div>

      <button onClick={openEditor}
        style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>{"\u{1F4DD}"} Edit Goods Library…</button>

      {step8Done && !reportOpen && (
        <button onClick={() => setReportOpen(true)}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>
          {"\u{1F33F}"} Goods placement report…
        </button>
      )}
      {reportOpen && <GoodsReportPanel onClose={() => setReportOpen(false)} />}

      <div style={{ color: "#405060", fontSize: 10, marginTop: 2 }}>
        Shark waters: warm, shallow, frequented coasts (bull/tiger-shark habitat).
        Trade goods: {GOOD_DEFS.length} climate/terrain belts, each a toggle in the
        Toolbar &rsaquo; Trade Goods. The Trade Matrix &amp; economy now live in the
        Economy step (10).
      </div>
    </div>
  );
}

const warn: React.CSSProperties = {
  color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3,
};
