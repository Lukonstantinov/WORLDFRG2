import { useState } from "react";
import { MapCanvas } from "./ui/MapCanvas";
import { Toolbar } from "./ui/Toolbar";
import { StatusBar } from "./ui/StatusBar";
import { WorkflowPanel } from "./ui/workflow/WorkflowPanel";
import { InfoPanel } from "./ui/InfoPanel";
import { TradeMatrixPanel } from "./ui/TradeMatrixPanel";
import { HubPanel } from "./ui/HubPanel";
import { MerchantRoutePanel } from "./ui/MerchantRoutePanel";
import { FuturesLanePanel } from "./ui/FuturesLanePanel";
import { FuturesPanel } from "./ui/FuturesPanel";
import { WarehousesPanel } from "./ui/WarehousesPanel";
import { CityRankingPanel } from "./ui/CityRankingPanel";
import { GoodFlowPanel } from "./ui/GoodFlowPanel";
import { GoodsBrowserPanel } from "./ui/GoodsBrowserPanel";
import { HousesPanel } from "./ui/HousesPanel";
import { SpeculationPanel } from "./ui/SpeculationPanel";
import { SettlementSearch } from "./ui/SettlementSearch";
import { ElevationLegend } from "./ui/ElevationLegend";
import { ElevationHistogram } from "./ui/ElevationHistogram";
import { GoodsEditor } from "./ui/GoodsEditor";
import { GoodsChainReview } from "./ui/GoodsChainReview";
import { GoodDetailPanel } from "./ui/GoodDetailPanel";
import { ImportWorldDialog } from "./ui/ImportWorldDialog";
import { useWorldStore } from "./state/worldStore";
import { useUIStore } from "./state/uiStore";
import { useViewportStore } from "./state/viewportStore";
import { useGoodsStore } from "./state/goodsStore";
import { newWorld, saveWorldAs, openWorld, exportHeightmap, exportLayers, persistOverlays, getOverlays, saveCampaignAs, openCampaign, newCampaign, finalizeWorld } from "./bridge/tauri";

const EXPORTABLE_LAYERS: { id: string; label: string }[] = [
  { id: "land", label: "Land / Sea" },
  { id: "elevation", label: "Elevation" },
  { id: "terrain", label: "Hillshade" },
  { id: "climate", label: "Climate" },
  { id: "biomes", label: "Biomes" },
  { id: "temperature", label: "Temperature" },
  { id: "precipitation", label: "Precipitation" },
  { id: "soil", label: "Soil" },
  { id: "fertility", label: "Fertility" },
  { id: "fisheries", label: "Fisheries" },
  { id: "habitability", label: "Habitability" },
  { id: "currents", label: "Currents" },
  { id: "wind", label: "Wind" },
  { id: "shelf", label: "Shelf" },
  { id: "plates", label: "Plates" },
];

function ExportDialog({ name, onClose }: { name: string; onClose: () => void }) {
  const setStatus = useUIStore((s) => s.setStatus);
  const [selected, setSelected] = useState<Set<string>>(
    new Set(["elevation", "climate", "biomes"])
  );
  const [heightmap, setHeightmap] = useState(true);
  const [busy, setBusy] = useState(false);
  const base = name || "world";

  const toggle = (id: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });

  const handleExport = async () => {
    setBusy(true);
    try {
      let dir: string | null = null;
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const result = await open({ directory: true, title: "Choose export folder" });
        if (result) dir = result as string;
      } catch {
        const input = prompt("Enter export folder path:");
        if (input) dir = input;
      }
      if (!dir) { setBusy(false); return; }

      const layers = Array.from(selected);
      const written: string[] = [];
      if (layers.length > 0) {
        const paths = await exportLayers(dir, base, layers);
        written.push(...paths);
      }
      if (heightmap) {
        const hp = `${dir}/${base}_heightmap.png`;
        await exportHeightmap(hp);
        written.push(hp);
      }
      setStatus(`Exported ${written.length} file(s) to ${dir}`);
      onClose();
    } catch (err) {
      console.error("Export failed:", err);
      alert("Export failed: " + err);
    }
    setBusy(false);
  };

  return (
    <div style={{
      position: "absolute", inset: 0, display: "flex",
      alignItems: "center", justifyContent: "center",
      background: "rgba(0,0,0,0.75)", zIndex: 100,
    }}>
      <div style={{
        background: "#111820", border: "1px solid #1e2e42", borderRadius: 10,
        padding: "24px 28px", minWidth: 360, maxHeight: "80%", overflowY: "auto",
        boxShadow: "0 12px 40px rgba(0,0,0,0.5)",
      }}>
        <h2 style={{ margin: "0 0 16px", color: "#c0d8f0", fontSize: 17, fontWeight: 600 }}>
          Export Layers
        </h2>
        <div style={{ color: "#5a7898", fontSize: 11, marginBottom: 10 }}>
          Each selected layer is saved as <code>{base}_&lt;layer&gt;.png</code> at full resolution.
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "2px 12px", marginBottom: 14 }}>
          {EXPORTABLE_LAYERS.map((l) => (
            <label key={l.id} style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: 12, color: "#a0b8d0", padding: "2px 0" }}>
              <input type="checkbox" checked={selected.has(l.id)} onChange={() => toggle(l.id)}
                style={{ accentColor: "#4a90d0" }} />
              {l.label}
            </label>
          ))}
        </div>

        <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: 12, color: "#a0b8d0", marginBottom: 16, paddingTop: 8, borderTop: "1px solid #1a2535" }}>
          <input type="checkbox" checked={heightmap} onChange={() => setHeightmap((v) => !v)}
            style={{ accentColor: "#4a90d0" }} />
          16-bit grayscale heightmap (for game engines)
        </label>

        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button onClick={onClose} disabled={busy}
            style={{ padding: "8px 16px", borderRadius: 6, border: "1px solid #1e2e42", background: "#0d1219", color: "#7090b0", cursor: "pointer", fontSize: 13 }}>
            Cancel
          </button>
          <button onClick={handleExport} disabled={busy || (selected.size === 0 && !heightmap)}
            style={{ padding: "8px 18px", borderRadius: 6, border: "none", background: busy ? "#1a3050" : "#2060a0", color: "#fff", cursor: busy ? "wait" : "pointer", fontSize: 13, fontWeight: 600 }}>
            {busy ? "Exporting..." : "Choose Folder & Export"}
          </button>
        </div>
      </div>
    </div>
  );
}

function NewWorldDialog({ onCreated }: { onCreated: () => void }) {
  const [name, setName] = useState("My World");
  const [width, setWidth] = useState(3600);
  const [height, setHeight] = useState(1800);
  const [creating, setCreating] = useState(false);
  const setMeta = useWorldStore((s) => s.setMeta);

  const handleCreate = async () => {
    setCreating(true);
    try {
      const meta = await newWorld(name, width, height);
      setMeta(meta);
      onCreated();
    } catch (err) {
      console.error("Failed to create world:", err);
      alert("Failed to create world: " + err);
    }
    setCreating(false);
  };

  const presets = [
    { label: "Small (360\u00D7180)", w: 360, h: 180 },
    { label: "Medium (1800\u00D7900)", w: 1800, h: 900 },
    { label: "Standard (3600\u00D71800)", w: 3600, h: 1800 },
    { label: "Large (7200\u00D73600)", w: 7200, h: 3600 },
  ];

  return (
    <div style={{
      position: "absolute", inset: 0, display: "flex",
      alignItems: "center", justifyContent: "center",
      background: "rgba(0,0,0,0.75)", zIndex: 100,
    }}>
      <div style={{
        background: "#111820", border: "1px solid #1e2e42",
        borderRadius: 10, padding: "28px 32px", minWidth: 380,
        boxShadow: "0 12px 40px rgba(0,0,0,0.5)",
      }}>
        <h2 style={{ margin: "0 0 20px", color: "#c0d8f0", fontSize: 18, fontWeight: 600 }}>
          New World
        </h2>

        <div style={{ marginBottom: 14 }}>
          <label style={dialogLabel}>World Name</label>
          <input
            value={name} onChange={(e) => setName(e.target.value)}
            style={dialogInput}
          />
        </div>

        <div style={{ marginBottom: 14 }}>
          <label style={dialogLabel}>Grid Size</label>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>
            {presets.map((p) => (
              <button
                key={p.label}
                onClick={() => { setWidth(p.w); setHeight(p.h); }}
                style={{
                  padding: "5px 12px", borderRadius: 5, cursor: "pointer", fontSize: 11,
                  border: width === p.w && height === p.h ? "1px solid #3a7ac0" : "1px solid #1e2e42",
                  background: width === p.w && height === p.h ? "#1a3a5a" : "#0d1219",
                  color: width === p.w && height === p.h ? "#c0ddf0" : "#5a7090",
                  fontWeight: width === p.w && height === p.h ? 600 : 400,
                }}
              >
                {p.label}
              </button>
            ))}
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <input
              type="number" value={width} onChange={(e) => setWidth(Number(e.target.value))}
              style={{ ...dialogInput, width: 90 }}
            />
            <span style={{ color: "#3a5070" }}>\u00D7</span>
            <input
              type="number" value={height} onChange={(e) => setHeight(Number(e.target.value))}
              style={{ ...dialogInput, width: 90 }}
            />
          </div>
        </div>

        <div style={{ color: "#3a5070", fontSize: 11, marginBottom: 18 }}>
          {Math.ceil(width / 128)} \u00D7 {Math.ceil(height / 128)} = {Math.ceil(width / 128) * Math.ceil(height / 128)} tiles
        </div>

        <button
          onClick={handleCreate}
          disabled={creating}
          style={{
            width: "100%", padding: "10px 0", borderRadius: 6,
            border: "none", cursor: creating ? "wait" : "pointer",
            background: creating ? "#1a3050" : "#2060a0", color: "#fff",
            fontSize: 14, fontWeight: 600, letterSpacing: 0.3,
            transition: "background 0.15s",
          }}
        >
          {creating ? "Creating..." : "Create World"}
        </button>
      </div>
    </div>
  );
}

export default function App() {
  const isLoaded = useWorldStore((s) => s.isLoaded);
  const meta = useWorldStore((s) => s.meta);
  const setMeta = useWorldStore((s) => s.setMeta);
  const clear = useWorldStore((s) => s.clear);
  const setRivers = useWorldStore((s) => s.setRivers);
  const setLakes = useWorldStore((s) => s.setLakes);
  const setSettlements = useWorldStore((s) => s.setSettlements);
  const setEconomy = useWorldStore((s) => s.setEconomy);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const setStepsCompleted = useUIStore((s) => s.setStepsCompleted);
  const setWorkflowStep = useUIStore((s) => s.setWorkflowStep);
  const loadGoodsFromWorld = useGoodsStore((s) => s.loadFromWorld);
  const invalidateTiles = useViewportStore((s) => s.invalidateTiles);
  const setStatus = useUIStore((s) => s.setStatus);
  const [showDialog, setShowDialog] = useState(!isLoaded);
  const [showExport, setShowExport] = useState(false);
  const [showImport, setShowImport] = useState(false);

  const handleOpen = async () => {
    try {
      let path: string | null = null;
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const result = await open({
          filters: [{ name: "WorldForge", extensions: ["worldforge", "db"] }],
        });
        if (result) path = result as string;
      } catch {
        const input = prompt("Enter path to .worldforge file:");
        if (input) path = input;
      }
      if (!path) return;

      setStatus("Opening world...");
      const res = await openWorld(path);
      clear();
      setMeta(res.meta);
      invalidateTiles();
      // Re-hydrate the overlay layers persisted in the DB (settlements, rivers,
      // lakes, the trade economy) + the world's goods, so an uploaded world comes
      // back complete — not just the map tiles.
      try {
        await loadGoodsFromWorld();
        const ov = await getOverlays();
        if (ov.rivers?.length) setRivers(ov.rivers);
        if (ov.lakes?.length) setLakes(ov.lakes);
        if (ov.settlements?.length) setSettlements(ov.settlements);
        if (ov.economy && ov.economy.hubs.length) setEconomy(ov.economy);
        // Restore the persisted wizard progress; older saves never wrote it, so
        // fall back to inferring completion from which data is present.
        const restored = [
          ...parseProgress(res.world_progress),
          ...parseProgress(res.campaign_progress),
        ];
        if (restored.length > 0) {
          setStepsCompleted(restored);
        } else {
          if (ov.rivers?.length) [1, 2, 3, 4, 5, 6].forEach(markStepCompleted);
          if (ov.settlements?.length) markStepCompleted(7);
          if (ov.economy && ov.economy.hubs.length) [8, 9, 10].forEach(markStepCompleted);
        }
        // A finalized (locked) world is read-only — jump straight to the campaign
        // side. If its economy is already built, land on the campaign step;
        // otherwise the economy step so it can be (re)built on the locked map.
        if (res.meta.frozen) {
          setWorkflowStep((ov.economy && ov.economy.hubs.length ? 11 : 10) as never);
        }
      } catch (e) {
        console.warn("Overlay re-hydration skipped:", e);
      }
      setShowDialog(false);
      setStatus("World loaded: " + res.meta.name);

      // Pre-split single-file save: offer to split it into a frozen world file
      // + a campaign file (the in-memory migration already happened on open).
      if (res.legacy && confirm(
        "This is a legacy single-file save. Split it into a frozen .worldforge world " +
        "file and a .campaign file?\n\nThe original file is left untouched."
      )) {
        await finalizeWorld();
        setMeta({ ...res.meta, frozen: true });
        await handleSaveAs();
        await handleSaveCampaign();
      }
    } catch (err) {
      console.error("Failed to open world:", err);
      alert("Failed to open world: " + err);
    }
  };

  const handleOpenCampaign = async () => {
    if (!isLoaded) return;
    try {
      let path: string | null = null;
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const result = await open({
          filters: [{ name: "WorldForge Campaign", extensions: ["campaign"] }],
        });
        if (result) path = result as string;
      } catch {
        const input = prompt("Enter path to .campaign file:");
        if (input) path = input;
      }
      if (!path) return;

      const info = await openCampaign(path);
      if (!info.world_match) {
        alert(
          "This campaign was saved against a different (or since-modified) world. " +
          "Its settlements and economy may not line up with the current map."
        );
      }
      const ov = await getOverlays();
      setSettlements(ov.settlements ?? []);
      setEconomy(ov.economy && ov.economy.hubs.length ? ov.economy : null);
      const restored = parseProgress(info.campaign_progress);
      const ui = useUIStore.getState();
      const worldSteps = Object.entries(ui.stepCompleted)
        .filter(([k, v]) => v && Number(k) <= 6)
        .map(([k]) => Number(k));
      setStepsCompleted([...worldSteps, ...restored]);
      setStatus(`Campaign loaded: ${info.name}`);
    } catch (err) {
      console.error("Failed to open campaign:", err);
      alert("Failed to open campaign: " + err);
    }
  };

  const handleSaveCampaign = async () => {
    try {
      let path: string | null = null;
      const def = (meta?.name || "world") + ".campaign";
      try {
        const { save } = await import("@tauri-apps/plugin-dialog");
        const result = await save({
          filters: [{ name: "WorldForge Campaign", extensions: ["campaign"] }],
          defaultPath: def,
        });
        if (result) path = result;
      } catch {
        const input = prompt("Enter campaign save path:", def);
        if (input) path = input;
      }
      if (!path) return;

      setStatus("Saving campaign...");
      // Persist the on-screen settlements into the campaign table first.
      try {
        const w = useWorldStore.getState();
        await persistOverlays(w.settlements, w.rivers, w.lakes);
      } catch (e) {
        console.warn("Overlay persist skipped:", e);
      }
      await saveCampaignAs(path);
      setStatus("Campaign saved to " + path);
    } catch (err) {
      console.error("Campaign save failed:", err);
      alert("Campaign save failed: " + err);
    }
  };

  const handleNewCampaign = async () => {
    if (!meta?.frozen) {
      alert("Finalize the world first (World wizard, after step 6).");
      return;
    }
    const name = prompt("Campaign name:", "New Campaign");
    if (!name) return;
    try {
      await newCampaign(name);
      // Fresh campaign: human data resets, world geography (steps 1-6) stays.
      setSettlements([]);
      setEconomy(null);
      const ui = useUIStore.getState();
      setStepsCompleted(
        Object.entries(ui.stepCompleted)
          .filter(([k, v]) => v && Number(k) <= 6)
          .map(([k]) => Number(k)),
      );
      setStatus(`Campaign started: ${name}`);
    } catch (err) {
      alert("Failed to start campaign: " + err);
    }
  };

  const handleSaveAs = async () => {
    try {
      let path: string | null = null;
      try {
        const { save } = await import("@tauri-apps/plugin-dialog");
        const result = await save({
          filters: [{ name: "WorldForge", extensions: ["worldforge"] }],
          defaultPath: (meta?.name || "world") + ".worldforge",
        });
        if (result) path = result;
      } catch {
        const input = prompt("Enter save path:", (meta?.name || "world") + ".worldforge");
        if (input) path = input;
      }
      if (!path) return;

      setStatus("Saving...");
      // Persist the on-screen overlay layers into the DB first so the saved file is
      // a COMPLETE world (the economy is already persisted by the Economy step).
      try {
        const w = useWorldStore.getState();
        await persistOverlays(w.settlements, w.rivers, w.lakes);
      } catch (e) {
        console.warn("Overlay persist skipped:", e);
      }
      await saveWorldAs(path);
      setStatus("Saved to " + path);
    } catch (err) {
      console.error("Save failed:", err);
      alert("Save failed: " + err);
    }
  };

  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", background: "#080c12" }}>
      {/* Header bar */}
      <div style={{
        height: 38, display: "flex", alignItems: "center", padding: "0 12px",
        background: "#0a0f18", borderBottom: "1px solid #1a2535",
        gap: 0, flexShrink: 0,
      }}>
        <span style={{ fontWeight: 700, fontSize: 14, color: "#3a80c0", letterSpacing: 0.5, marginRight: 12 }}>
          WorldForge 2
        </span>
        {meta && (
          <span style={{ color: "#4a6080", fontSize: 12, marginRight: 16 }}>
            {meta.name}
          </span>
        )}

        <div style={{ display: "flex", gap: 2, marginLeft: 4 }}>
          <button onClick={() => setShowDialog(true)} style={headerBtn}>New</button>
          <button onClick={handleOpen} style={headerBtn}>Open</button>
          {isLoaded && (
            <>
              <button onClick={handleSaveAs} style={headerBtn}>Save World</button>
              <button onClick={handleNewCampaign} style={headerBtn} title="Start a fresh campaign on this finalized world">New Campaign</button>
              <button onClick={handleOpenCampaign} style={headerBtn}>Open Campaign</button>
              <button onClick={handleSaveCampaign} style={headerBtn}>Save Campaign</button>
              <button onClick={() => setShowImport(true)} style={headerBtn} title="Copy layers from another world file">Import Layers</button>
              <button onClick={() => setShowExport(true)} style={headerBtn}>Export</button>
            </>
          )}
        </div>

        <div style={{ flex: 1 }} />
        {isLoaded && <SettlementSearch />}
      </div>

      {/* Main layout: workflow | map | toolbar */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        {/* Left: Workflow panel */}
        {isLoaded && <WorkflowPanel />}

        {/* Center: Map */}
        <div style={{ flex: 1, position: "relative", minWidth: 0, minHeight: 0, overflow: "hidden" }}>
          <MapCanvas />
          <InfoPanel />
          <HubPanel />
          <GoodFlowPanel />
          <GoodsBrowserPanel />
          <HousesPanel />
          <SpeculationPanel />
          <CityRankingPanel />
          <MerchantRoutePanel />
          <FuturesLanePanel />
          <FuturesPanel />
          <WarehousesPanel />
          <TradeMatrixPanel />
          <ElevationLegend />
          <ElevationHistogram />
        </div>

        {/* Right: Toolbar */}
        <Toolbar />
      </div>

      <StatusBar />

      <GoodsEditor />
      <GoodsChainReview />
      <GoodDetailPanel />
      {showDialog && <NewWorldDialog onCreated={() => setShowDialog(false)} />}
      {showExport && <ExportDialog name={meta?.name || "world"} onClose={() => setShowExport(false)} />}
      {showImport && <ImportWorldDialog onClose={() => setShowImport(false)} />}
    </div>
  );
}

/** Parse a persisted step-completion map ({"3":true,…}) into step numbers. */
function parseProgress(json: string | null): number[] {
  if (!json) return [];
  try {
    return Object.entries(JSON.parse(json) as Record<string, boolean>)
      .filter(([, v]) => v)
      .map(([k]) => Number(k))
      .filter((n) => Number.isFinite(n));
  } catch {
    return [];
  }
}

const dialogLabel: React.CSSProperties = {
  color: "#5a7898", fontSize: 11, display: "block", marginBottom: 4, fontWeight: 500,
};

const dialogInput: React.CSSProperties = {
  width: "100%", padding: "7px 10px", background: "#080c12",
  border: "1px solid #1e2e42", borderRadius: 5, color: "#c0d0e0", fontSize: 13,
  outline: "none",
};

const headerBtn: React.CSSProperties = {
  padding: "4px 12px", borderRadius: 4, border: "1px solid #1a2a40",
  background: "transparent", color: "#6a8aaa", cursor: "pointer", fontSize: 11,
  fontWeight: 500, transition: "background 0.1s",
};
