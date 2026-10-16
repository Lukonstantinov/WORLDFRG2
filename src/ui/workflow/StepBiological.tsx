import { useUIStore } from "@state/uiStore";
import { useWorldStore, decodeProvinceRaster } from "@state/worldStore";
import { useGoodsStore } from "@state/goodsStore";
import { simBiological, simGenerateProvinces } from "@bridge";
import { GOOD_DEFS, goodOverlayKey } from "@goods";
import { genBtn } from "@ui/workflow/WorkflowPanel";
import { GoodsReportPanel } from "@ui/goods/GoodsReportPanel";
import { useState } from "react";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

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
  const settlements = useWorldStore((s) => s.settlements);
  const setSettlements = useWorldStore((s) => s.setSettlements);
  const setProvinces = useWorldStore((s) => s.setProvinces);
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
    setStatus("Computing sea hazards, trade goods, routes & matrix...");
    try {
      // Snapshot any edited good specs into the world so generation uses them.
      if (goodsSpecs.length > 0) await applyGoodsToWorld();
      await simBiological(seed, JSON.stringify(rivers), bioParams.gemDeposits, bioParams.climateStrictness);
      // A province freezes its goods when it is built, so provinces made in step 7
      // (before any goods existed) carried none. Rebuild them now, at the saved size —
      // the same order Run-All uses.
      if (settlements.length > 0) {
        setStatus("Rebuilding provinces with their goods...");
        const res = await simGenerateProvinces(settlements, rivers);
        setProvinces(res.provinces, decodeProvinceRaster(res));
        const founded = res.founded_settlements ?? [];
        if (founded.length > 0) setSettlements([...settlements, ...founded]);
      }
      markStepCompleted(8); // gates the trade-route / flow computation in MapCanvas
      invalidateTiles();    // bumps tileVersion → refetches goods/routes/flows
      // Surface the new overlays.
      useUIStore.getState().borrowOverlay("stormZones");
      useUIStore.getState().borrowOverlay("reefZones");
      useUIStore.getState().borrowOverlay("tradeRoutes");
      useUIStore.getState().borrowOverlay("tradeFlows");
      setStatus("Goods & trade computed: hazards, goods, routes & trade matrix");
      setReportOpen(true);
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  // Goods generation ALWAYS routes through the Goods & Chains review window first
  // (planted vs manufactured + the recipe schematic), then runs on confirm.
  const handleGenerate = async () => {
    if (simRunning) return;
    if (!step6Done) {
      setStatus("Step 6 required: compute Soil & Fertility first (fisheries drive the fish goods)");
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
        <div style={warn}>Complete Step 6 first (fisheries feed the fish-good belts)</div>
      )}
      {step6Done && !step7Done && (
        <div style={warn}>Complete Step 7 first (the trade matrix needs settlements)</div>
      )}

      {/* Generation parameters */}
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#897a61" }}>
        <span title="Ore and gem districts scale with this world's land area automatically; this sets how rich the crust is (6 = Earth-like).">Ore richness</span>
        <span style={{ color: "#bcab8e" }}>×{(bioParams.gemDeposits / 6).toFixed(2)}</span>
      </div>
      <input type="range" min={0} max={16} value={bioParams.gemDeposits}
        onChange={(e) => setBioParams({ gemDeposits: Number(e.target.value) })}
        style={{ width: "100%" }} />

      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#897a61", marginTop: 2 }}>
        <span>Climate strictness</span>
        <span style={{ color: "#bcab8e" }}>
          {bioParams.climateStrictness < 0.4 ? "diffuse" : bioParams.climateStrictness > 0.6 ? "tight" : "neutral"}
        </span>
      </div>
      <input type="range" min={0} max={100} value={Math.round(bioParams.climateStrictness * 100)}
        onChange={(e) => setBioParams({ climateStrictness: Number(e.target.value) / 100 })}
        style={{ width: "100%" }} />
      <div style={{ fontSize: 9, color: "#897a61" }}>
        How tightly each good hugs its ideal climate (tight = smaller, more clustered belts).
      </div>

      <button onClick={handleGenerate} disabled={simRunning || !step6Done || !step7Done} style={{ ...genBtn, marginTop: 2 }}>
        Generate Goods &amp; Hazards
      </button>

      <div style={{ display: "flex", gap: 4, marginTop: 2, flexWrap: "wrap" }}>
        <button onClick={() => useUIStore.getState().toggleOverlay("stormZones")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Hurricanes</button>
        <button onClick={() => useUIStore.getState().toggleOverlay("reefZones")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Reefs</button>
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

      <div style={{ color: "#5c5344", fontSize: 10, marginTop: 2 }}>
        Trade goods: {GOOD_DEFS.length} climate/terrain belts, each a toggle in the
        Toolbar &rsaquo; Trade Goods. Trade-route settings live in the Political step (9);
        the Trade Matrix &amp; economy in the Economy step (10).
      </div>
    </div>
  );
}

const warn: React.CSSProperties = {
  color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3,
};
