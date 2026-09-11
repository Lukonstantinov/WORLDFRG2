import { useState } from "react";
import { MapCanvas } from "@ui/world/MapCanvas";
import { Toolbar } from "@ui/world/Toolbar";
import { StatusBar } from "@ui/world/StatusBar";
import { WorkflowPanel } from "@ui/workflow/WorkflowPanel";
import { ChroniclePanel } from "@ui/campaign/ChroniclePanel";
import { InfoPanel } from "@ui/world/InfoPanel";
import { TradeMatrixPanel } from "@ui/goods/TradeMatrixPanel";
import { HubPanel } from "@ui/campaign/HubPanel";
import { SatelliteConstructionPanel } from "@ui/campaign/SatelliteConstructionPanel";
import { MerchantRoutePanel } from "@ui/goods/MerchantRoutePanel";
import { FuturesLanePanel } from "@ui/campaign/FuturesLanePanel";
import { FuturesPanel } from "@ui/campaign/FuturesPanel";
import { WarehousesPanel } from "@ui/campaign/WarehousesPanel";
import { CityRankingPanel } from "@ui/campaign/CityRankingPanel";
import { GoodFlowPanel } from "@ui/goods/GoodFlowPanel";
import { GoodsBrowserPanel } from "@ui/goods/GoodsBrowserPanel";
import { HousesPanel } from "@ui/campaign/HousesPanel";
import { MoneyFinancePanel } from "@ui/campaign/MoneyFinancePanel";
import { WarPanel } from "@ui/campaign/WarPanel";
import { StatesPanel } from "@ui/campaign/StatesPanel";
import { ItineraryPanel } from "@ui/goods/ItineraryPanel";
import { GoodsCodexPanel } from "@ui/goods/GoodsCodexPanel";
import { EconomyDashboardPanel } from "@ui/campaign/EconomyDashboardPanel";
import { HydrologyPanel } from "@ui/world/HydrologyPanel";
import { ProvincePanel } from "@ui/world/ProvincePanel";
import { ProvinceInspector } from "@ui/world/ProvinceInspector";
import { AtlasPanel } from "@ui/campaign/AtlasPanel";
import { PeoplesPanel } from "@ui/campaign/PeoplesPanel";
import { ColonialPanel } from "@ui/campaign/ColonialPanel";
import { BankPanel } from "@ui/campaign/BankPanel";
import { NewsFeedPanel } from "@ui/campaign/NewsFeedPanel";
import { PlaguePanel } from "@ui/campaign/PlaguePanel";
import { ImmigrationPanel } from "@ui/campaign/ImmigrationPanel";
import { GuildsPanel } from "@ui/campaign/GuildsPanel";
import { FiguresPanel } from "@ui/campaign/FiguresPanel";
import { LandmarksPanel } from "@ui/campaign/LandmarksPanel";
import { DynastiesPanel } from "@ui/campaign/DynastiesPanel";
import { GoodsMarketPanel } from "@ui/goods/GoodsMarketPanel";
import { MarketsPanel } from "@ui/campaign/MarketsPanel";
import { SettlementSearch } from "@ui/world/SettlementSearch";
import { LayerLegend, GoodQualityLegend } from "@ui/world/LayerLegend";
import { GoodsEditor } from "@ui/goods/GoodsEditor";
import { GoodsChainReview } from "@ui/goods/GoodsChainReview";
import { GoodDetailPanel } from "@ui/goods/GoodDetailPanel";
import { ImportWorldDialog } from "@ui/world/ImportWorldDialog";
import { SettingsPanel } from "@ui/SettingsPanel";
import { CampaignLibraryPanel } from "@ui/campaign/CampaignLibraryPanel";
import { useCampaignStore } from "@state/campaignStore";
import { WindowBar } from "@ui/world/WindowBar";
import { CampaignTopBar } from "@ui/campaign/CampaignTopBar";
import { ErrorBoundary } from "@ui/world/ErrorBoundary";
import { useWorldStore, decodeProvinceRaster } from "@state/worldStore";
import { useUIStore } from "@state/uiStore";
import { useViewportStore } from "@state/viewportStore";
import { useGoodsStore } from "@state/goodsStore";
import { newWorld, saveWorldAs, openWorld, exportHeightmap, exportLayers, writeExportImage, persistOverlays, getOverlays, saveCampaignAs, openCampaign, newCampaign, finalizeWorld, getAppearance, getToponyms, getProvinceLayer, worldHumanLayerStatus } from "@bridge";
import { useSettingsStore } from "@state/settingsStore";
import { getApp } from "@canvas/PixiApp";
import { exportMapSnapshot, exportMapSnapshotRaw } from "@canvas/mapExport";
import { PdfDocument, rgbaToRgb, type TextLine } from "@canvas/pdfWriter";
import { MAP_THEMES, applyMapTheme } from "@ui/world/mapThemes";
import { layerGroups } from "@ui/world/Toolbar";

// GENERATION_UX_REDESIGN_PLAN.md Slice 8 (F6) — DERIVED from `layerGroups`,
// the canonical list `Toolbar.tsx` renders from, instead of a second
// hand-copied table (the old list drifted to 15 of the real 26 layers).
const EXPORTABLE_LAYERS: { id: string; label: string }[] =
  layerGroups.flatMap((g) => g.layers);

// GENERATION_UX_REDESIGN_PLAN.md Slice 8 (F5/F6/F7/F10) — the export is now a
// COMPOSITION, not a layer dump. "Map (PNG)" captures the live on-screen
// canvas — base layer through the same tile path the screen uses (fixing F5's
// seams by construction, since there is no second render path to drift from
// it) PLUS every visible overlay (rivers/cities/borders/names — F7, which the
// old Rust-side tile stitcher could never see) at whatever opacity is
// currently set (Slice 10, inherited for free). Available in BOTH modes
// (F10) since it just reads whatever `getApp()` is showing, Forge or
// Chronicle alike. The MAP PLATE picker lets you apply one of the named
// compositions (§8.17) before capturing, rather than assembling one by hand.
// "Layers (raw)" below is the OLD path, kept for raw single-layer data
// export (a heightmap-adjacent use case a composited screenshot can't serve).
function ExportDialog({ name, onClose }: { name: string; onClose: () => void }) {
  const setStatus = useUIStore((s) => s.setStatus);
  const [tab, setTab] = useState<"map" | "layers" | "atlas">("map");
  const [selected, setSelected] = useState<Set<string>>(
    new Set(["elevation", "climate", "biomes"])
  );
  const [heightmap, setHeightmap] = useState(true);
  const [busy, setBusy] = useState(false);
  const base = name || "world";
  const activeMapTheme = useUIStore((s) => s.activeMapTheme);
  // GENERATION_UX_REDESIGN_PLAN.md Slice 8 — the resolution multiplier: real
  // additional pixel density over the current view (`exportMapSnapshot`
  // re-runs the exact on-screen draw code at a boosted device-pixel ratio),
  // not a re-scale of an already-rasterized screenshot.
  const [resMultiplier, setResMultiplier] = useState(1);
  // GENERATION_UX_REDESIGN_PLAN.md Slice 8 (the PDF atlas) — a page per
  // selected plate, a gazetteer page, real vector text via the PDF standard
  // fonts (no embedding needed — see pdfWriter.ts's own doc comment).
  const [atlasPlates, setAtlasPlates] = useState<Set<string>>(new Set(["physical"]));
  const [atlasGazetteer, setAtlasGazetteer] = useState(true);
  const [atlasProgress, setAtlasProgress] = useState("");
  const settlements = useWorldStore((s) => s.settlements);
  const rivers = useWorldStore((s) => s.rivers);
  const lakes = useWorldStore((s) => s.lakes);
  const provinces = useWorldStore((s) => s.provinces);

  const toggleAtlasPlate = (id: string) =>
    setAtlasPlates((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });

  const toggle = (id: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });

  const handleExportMap = async () => {
    if (!getApp()) { alert("Map canvas not ready."); return; }
    setBusy(true);
    try {
      const dataUrl = exportMapSnapshot(resMultiplier);
      if (!dataUrl) { alert("Could not render the map for export."); setBusy(false); return; }
      let path: string | null = null;
      const def = `${base}_map.png`;
      try {
        const { save } = await import("@tauri-apps/plugin-dialog");
        const result = await save({ filters: [{ name: "PNG Image", extensions: ["png"] }], defaultPath: def });
        if (result) path = result;
      } catch {
        const input = prompt("Save the map PNG to path:", def);
        if (input) path = input;
      }
      if (!path) { setBusy(false); return; }
      await writeExportImage(path, dataUrl);
      setStatus(`Map exported to ${path}`);
      onClose();
    } catch (err) {
      console.error("Map export failed:", err);
      alert("Map export failed: " + err);
    }
    setBusy(false);
  };

  const handleExportLayers = async () => {
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

  // GENERATION_UX_REDESIGN_PLAN.md Slice 8 (the PDF atlas). Page size is A4
  // landscape in PDF points (842 x 595) — every page shares this size
  // (`PdfDocument`'s own constraint), so a map plate's raster is scaled to
  // fill the full bleed; the gazetteer's text pages share it too.
  const ATLAS_PAGE_W = 842;
  const ATLAS_PAGE_H = 595;

  /** Chunked base64 encode — `String.fromCharCode(...bytes)` overflows the
   *  call stack on a multi-MB uncompressed page (this writer's own honest
   *  trade-off, see pdfWriter.ts's doc comment), so this walks the buffer in
   *  small slices instead of spreading the whole array at once. */
  const bytesToBase64 = (bytes: Uint8Array): string => {
    let binary = "";
    const chunk = 0x8000;
    for (let i = 0; i < bytes.length; i += chunk) {
      binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
    }
    return btoa(binary);
  };

  /** Paginates a flat list of gazetteer lines into `TextLine[]` pages sized
   *  to fit the atlas page, a section heading (bold, larger) starting each
   *  section on its own fresh page rather than running headings together
   *  with the entries below them. */
  const buildGazetteerPages = (
    sections: { title: string; rows: string[] }[],
  ): TextLine[][] => {
    const marginX = 44;
    const topY = ATLAS_PAGE_H - 50;
    const lineH = 13;
    const bottomY = 40;
    const pages: TextLine[][] = [];
    let page: TextLine[] = [];
    let y = topY;
    const newPage = () => { if (page.length) pages.push(page); page = []; y = topY; };
    for (const section of sections) {
      if (!section.rows.length) continue;
      if (y < topY) newPage(); // start each section on a fresh page
      page.push({ text: section.title, x: marginX, y, size: 15, font: "F2" });
      y -= lineH * 1.6;
      for (const row of section.rows) {
        if (y < bottomY) { newPage(); page.push({ text: `${section.title} (cont.)`, x: marginX, y, size: 11, font: "F2" }); y -= lineH * 1.4; }
        page.push({ text: row, x: marginX, y, size: 9, font: "F1" });
        y -= lineH;
      }
      y -= lineH; // gap before next section
    }
    if (page.length) pages.push(page);
    return pages;
  };

  const handleExportAtlas = async () => {
    if (!getApp()) { alert("Map canvas not ready."); return; }
    const plates = MAP_THEMES.filter((t) => atlasPlates.has(t.id));
    if (plates.length === 0 && !atlasGazetteer) { alert("Pick at least one plate or the gazetteer."); return; }
    setBusy(true);
    setAtlasProgress("");
    try {
      const savedTheme = activeMapTheme;
      const doc = new PdfDocument(ATLAS_PAGE_W, ATLAS_PAGE_H);

      for (let i = 0; i < plates.length; i++) {
        const theme = plates[i];
        setAtlasProgress(`Rendering plate ${i + 1}/${plates.length}: ${theme.name}…`);
        applyMapTheme(theme);
        // Same bounded-delay heuristic the single-page PNG export already
        // relies on to let the tile layer finish redrawing after a theme
        // switch — there is no "redraw complete" signal to await instead.
        await new Promise((r) => setTimeout(r, 700));
        const img = exportMapSnapshotRaw(1);
        if (!img) continue;
        const rgb = rgbaToRgb(img);
        doc.addImagePage(rgb, img.width, img.height, [
          { text: theme.name, x: 40, y: ATLAS_PAGE_H - 34, size: 16, font: "F2" },
          { text: base, x: 40, y: 22, size: 9, font: "F1" },
        ]);
      }

      if (atlasGazetteer) {
        setAtlasProgress("Building gazetteer…");
        const settlementRows = settlements
          .slice().sort((a, b) => b.population - a.population)
          .map((s) => `${s.name}  —  ${s.size}, pop. ${s.population.toLocaleString()}  (${s.x}, ${s.y})`);
        const riverRows = rivers
          .filter((r) => r.points.length > 0)
          .map((r, i) => `River #${i + 1}${r.major ? " (major)" : ""}${r.navigable ? ", navigable" : ""}  —  source (${r.points[0][0]}, ${r.points[0][1]})`);
        const lakeRows = lakes
          .filter((l) => l.cells.length > 0)
          .map((l, i) => `Lake #${i + 1}${l.endorheic ? " (salt)" : ""}  —  ${l.cells.length} cells, near (${l.cells[0][0]}, ${l.cells[0][1]})`);
        const provinceRows = provinces
          .slice().sort((a, b) => a.name.localeCompare(b.name))
          .map((p) => `${p.name}  —  ${p.culture}, ${p.cells} cells  (seat ${p.seat_x}, ${p.seat_y})`);

        const pages = buildGazetteerPages([
          { title: "Gazetteer — Settlements", rows: settlementRows },
          { title: "Gazetteer — Provinces", rows: provinceRows },
          { title: "Gazetteer — Rivers", rows: riverRows },
          { title: "Gazetteer — Lakes", rows: lakeRows },
        ]);
        for (const lines of pages) doc.addTextPage(lines);
      }

      if (savedTheme) {
        const t = MAP_THEMES.find((t) => t.id === savedTheme);
        if (t) applyMapTheme(t);
      }

      setAtlasProgress("Writing PDF…");
      const bytes = doc.build();
      const b64 = bytesToBase64(bytes);

      let path: string | null = null;
      const def = `${base}_atlas.pdf`;
      try {
        const { save } = await import("@tauri-apps/plugin-dialog");
        const result = await save({ filters: [{ name: "PDF Document", extensions: ["pdf"] }], defaultPath: def });
        if (result) path = result;
      } catch {
        const input = prompt("Save the atlas PDF to path:", def);
        if (input) path = input;
      }
      if (!path) { setBusy(false); setAtlasProgress(""); return; }
      await writeExportImage(path, `data:application/pdf;base64,${b64}`);
      setStatus(`Atlas exported to ${path}`);
      onClose();
    } catch (err) {
      console.error("Atlas export failed:", err);
      alert("Atlas export failed: " + err);
    }
    setBusy(false);
    setAtlasProgress("");
  };

  return (
    <div style={{
      position: "absolute", inset: 0, display: "flex",
      alignItems: "center", justifyContent: "center",
      background: "rgba(0,0,0,0.75)", zIndex: 100,
    }}>
      <div style={{
        background: "#111820", border: "1px solid #1e2e42", borderRadius: 10,
        padding: "24px 28px", minWidth: 380, maxHeight: "80%", overflowY: "auto",
        boxShadow: "0 12px 40px rgba(0,0,0,0.5)",
      }}>
        <h2 style={{ margin: "0 0 12px", color: "#c0d8f0", fontSize: 17, fontWeight: 600 }}>
          Export
        </h2>
        <div style={{ display: "flex", gap: 4, marginBottom: 14 }}>
          {(["map", "layers", "atlas"] as const).map((t) => (
            <button key={t} onClick={() => setTab(t)}
              style={{
                flex: 1, padding: "6px 0", borderRadius: 6, cursor: "pointer", fontSize: 12,
                border: `1px solid ${tab === t ? "#4a90d0" : "#1e2e42"}`,
                background: tab === t ? "#16324a" : "#0d1219",
                color: tab === t ? "#cfe2f6" : "#7090b0", fontWeight: tab === t ? 600 : 400,
              }}>
              {t === "map" ? "Map (PNG)" : t === "layers" ? "Layers (raw)" : "Atlas (PDF)"}
            </button>
          ))}
        </div>

        {tab === "map" ? (
          <>
            <div style={{ color: "#5a7898", fontSize: 11, marginBottom: 10, lineHeight: 1.4 }}>
              Captures the map exactly as shown — base layer, every visible
              overlay (rivers, cities, borders, names, trade routes…) and their
              opacity. Set up the view you want first, or pick a plate below.
            </div>
            <div style={{ marginBottom: 14 }}>
              <label style={dialogLabel}>Map Plate</label>
              <select
                value={activeMapTheme ?? ""}
                onChange={(e) => {
                  const theme = MAP_THEMES.find((t) => t.id === e.target.value);
                  if (theme) applyMapTheme(theme);
                }}
                style={dialogInput}
              >
                <option value="">— current view —</option>
                {MAP_THEMES.map((t) => (
                  <option key={t.id} value={t.id}>{t.name}</option>
                ))}
              </select>
              {activeMapTheme && (
                <div style={{ color: "#405060", fontSize: 9, marginTop: 3 }}>
                  Give the map a moment to finish redrawing before exporting.
                </div>
              )}
            </div>
            <div style={{ marginBottom: 14 }}>
              <label style={dialogLabel}>Resolution</label>
              <div style={{ display: "flex", gap: 4 }}>
                {[1, 2, 3, 4].map((m) => (
                  <button key={m} onClick={() => setResMultiplier(m)}
                    style={{
                      flex: 1, padding: "6px 0", borderRadius: 6, cursor: "pointer", fontSize: 12,
                      border: `1px solid ${resMultiplier === m ? "#4a90d0" : "#1e2e42"}`,
                      background: resMultiplier === m ? "#16324a" : "#0d1219",
                      color: resMultiplier === m ? "#cfe2f6" : "#7090b0", fontWeight: resMultiplier === m ? 600 : 400,
                    }}>
                    {m}×
                  </button>
                ))}
              </div>
              <div style={{ color: "#405060", fontSize: 9, marginTop: 3 }}>
                Extra pixel density over the current view — a real re-render at
                the higher resolution, not a stretched screenshot. Higher
                multiples take longer and produce a larger file.
              </div>
            </div>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button onClick={onClose} disabled={busy}
                style={{ padding: "8px 16px", borderRadius: 6, border: "1px solid #1e2e42", background: "#0d1219", color: "#7090b0", cursor: "pointer", fontSize: 13 }}>
                Cancel
              </button>
              <button onClick={handleExportMap} disabled={busy}
                style={{ padding: "8px 18px", borderRadius: 6, border: "none", background: busy ? "#1a3050" : "#2060a0", color: "#fff", cursor: busy ? "wait" : "pointer", fontSize: 13, fontWeight: 600 }}>
                {busy ? "Exporting..." : "Save Map PNG…"}
              </button>
            </div>
          </>
        ) : tab === "layers" ? (
          <>
            <div style={{ color: "#5a7898", fontSize: 11, marginBottom: 10 }}>
              Each selected layer is saved as <code>{base}_&lt;layer&gt;.png</code> at full
              grid resolution — raw data, no overlays. Use "Map (PNG)" for a real map.
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
              <button onClick={handleExportLayers} disabled={busy || (selected.size === 0 && !heightmap)}
                style={{ padding: "8px 18px", borderRadius: 6, border: "none", background: busy ? "#1a3050" : "#2060a0", color: "#fff", cursor: busy ? "wait" : "pointer", fontSize: 13, fontWeight: 600 }}>
                {busy ? "Exporting..." : "Choose Folder & Export"}
              </button>
            </div>
          </>
        ) : (
          <>
            <div style={{ color: "#5a7898", fontSize: 11, marginBottom: 10, lineHeight: 1.4 }}>
              A multi-page PDF: one full-bleed page per selected plate (real
              vector title/caption text, no font embedding needed) plus an
              optional gazetteer listing settlements, provinces, rivers and
              lakes. Each map page is a raw, uncompressed raster, so a
              several-plate atlas can be a large file.
            </div>
            <div style={{ marginBottom: 14 }}>
              <label style={dialogLabel}>Plates</label>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "2px 12px", maxHeight: 160, overflowY: "auto" }}>
                {MAP_THEMES.map((t) => (
                  <label key={t.id} style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: 12, color: "#a0b8d0", padding: "2px 0" }}>
                    <input type="checkbox" checked={atlasPlates.has(t.id)} onChange={() => toggleAtlasPlate(t.id)}
                      style={{ accentColor: "#4a90d0" }} />
                    {t.name}
                  </label>
                ))}
              </div>
            </div>
            <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: 12, color: "#a0b8d0", marginBottom: 16, paddingTop: 8, borderTop: "1px solid #1a2535" }}>
              <input type="checkbox" checked={atlasGazetteer} onChange={() => setAtlasGazetteer((v) => !v)}
                style={{ accentColor: "#4a90d0" }} />
              Gazetteer pages (settlements, provinces, rivers, lakes)
            </label>
            {atlasProgress && (
              <div style={{ color: "#4a90d0", fontSize: 11, marginBottom: 10 }}>{atlasProgress}</div>
            )}
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button onClick={onClose} disabled={busy}
                style={{ padding: "8px 16px", borderRadius: 6, border: "1px solid #1e2e42", background: "#0d1219", color: "#7090b0", cursor: "pointer", fontSize: 13 }}>
                Cancel
              </button>
              <button onClick={handleExportAtlas} disabled={busy || (atlasPlates.size === 0 && !atlasGazetteer)}
                style={{ padding: "8px 18px", borderRadius: 6, border: "none", background: busy ? "#1a3050" : "#2060a0", color: "#fff", cursor: busy ? "wait" : "pointer", fontSize: 13, fontWeight: 600 }}>
                {busy ? "Building..." : "Build Atlas PDF…"}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function NewWorldDialog({ onCreated, onOpenExisting }: { onCreated: () => void; onOpenExisting: () => void }) {
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
      // A brand-new world is unfinalized — always land in Forge (build) mode.
      useUIStore.getState().setAppMode("forge");
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

        <div style={{ display: "flex", alignItems: "center", gap: 10, margin: "14px 0" }}>
          <div style={{ flex: 1, height: 1, background: "#1e2e42" }} />
          <span style={{ color: "#3a5070", fontSize: 10 }}>or</span>
          <div style={{ flex: 1, height: 1, background: "#1e2e42" }} />
        </div>

        <button
          onClick={onOpenExisting}
          disabled={creating}
          style={{
            width: "100%", padding: "9px 0", borderRadius: 6,
            border: "1px solid #2a3a50", cursor: creating ? "wait" : "pointer",
            background: "transparent", color: "#8ab0d0",
            fontSize: 13, fontWeight: 500,
          }}
        >
          📂 Open Existing World...
        </button>
        <div style={{ color: "#3a5070", fontSize: 10, marginTop: 8, lineHeight: 1.4 }}>
          Upload a .worldforge file saved at ANY point mid-generation — the
          Workflow panel picks up right where it left off, with completed
          steps checked and the rest ready to continue.
        </div>
      </div>
    </div>
  );
}

/// Naming a new campaign. This exists because `window.prompt` is not implemented
/// by ANY of Tauri's three webviews: WKWebView (macOS) and WebKit2GTK (Linux)
/// require the host application to supply a text-input panel, and WebView2
/// (Windows) has no prompt at all. It returns null everywhere, which the caller
/// read as "user cancelled" — so starting a campaign silently did nothing.
///
/// Every other `prompt()` in this file is a FALLBACK inside a `catch` after
/// `@tauri-apps/plugin-dialog` has been tried. This was the one place it was the
/// only path. Keep it that way: never make `prompt`, `alert` or `confirm` the
/// primary route to a feature.
function NewCampaignDialog({
  worldName,
  onStart,
  onCancel,
}: {
  worldName: string;
  onStart: (name: string) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(`${worldName} Campaign`);
  const trimmed = name.trim();

  return (
    <div
      style={{
        position: "absolute", inset: 0, display: "flex",
        alignItems: "center", justifyContent: "center",
        background: "rgba(0,0,0,0.75)", zIndex: 100,
      }}
      onKeyDown={(e) => {
        if (e.key === "Escape") onCancel();
        if (e.key === "Enter" && trimmed) onStart(trimmed);
      }}
    >
      <div style={{
        background: "#111820", border: "1px solid #1e2e42",
        borderRadius: 10, padding: "28px 32px", minWidth: 380,
        boxShadow: "0 12px 40px rgba(0,0,0,0.5)",
      }}>
        <h2 style={{ margin: "0 0 8px", color: "#c0d8f0", fontSize: 18, fontWeight: 600 }}>
          New Campaign
        </h2>
        <p style={{ margin: "0 0 20px", color: "#5a7090", fontSize: 11, lineHeight: 1.5 }}>
          The world's geography is kept. Settlements, economy and history start over.
        </p>

        <div style={{ marginBottom: 18 }}>
          <label style={dialogLabel}>Campaign Name</label>
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            style={dialogInput}
          />
        </div>

        <div style={{ display: "flex", gap: 8 }}>
          <button
            onClick={onCancel}
            style={{
              flex: "0 0 auto", padding: "10px 18px", borderRadius: 6,
              border: "1px solid #1e2e42", cursor: "pointer",
              background: "#0d1219", color: "#5a7090", fontSize: 13,
            }}
          >
            Cancel
          </button>
          <button
            onClick={() => trimmed && onStart(trimmed)}
            disabled={!trimmed}
            style={{
              flex: 1, padding: "10px 0", borderRadius: 6,
              border: "none", cursor: trimmed ? "pointer" : "not-allowed",
              background: trimmed ? "#2060a0" : "#1a3050",
              color: trimmed ? "#fff" : "#54708f",
              fontSize: 14, fontWeight: 600, letterSpacing: 0.3,
              transition: "background 0.15s",
            }}
          >
            Start Campaign
          </button>
        </div>
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
  const setProvinces = useWorldStore((s) => s.setProvinces);
  const setEconomy = useWorldStore((s) => s.setEconomy);
  const setToponyms = useWorldStore((s) => s.setToponyms);
  const showWorkflow = useUIStore((s) => s.showWorkflow);
  const showToolbar = useUIStore((s) => s.showToolbar);
  const appMode = useUIStore((s) => s.appMode);
  const setAppMode = useUIStore((s) => s.setAppMode);
  const setStepsCompleted = useUIStore((s) => s.setStepsCompleted);
  const setWorkflowStep = useUIStore((s) => s.setWorkflowStep);
  const loadGoodsFromWorld = useGoodsStore((s) => s.loadFromWorld);
  const invalidateTiles = useViewportStore((s) => s.invalidateTiles);
  const setStatus = useUIStore((s) => s.setStatus);
  const [showDialog, setShowDialog] = useState(!isLoaded);
  const [showExport, setShowExport] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showCampaignDialog, setShowCampaignDialog] = useState(false);
  const [showLibrary, setShowLibrary] = useState(false);
  const campaignActive = useCampaignStore((s) => s.snapshot?.active === true);

  const handleOpen = async () => {
    try {
      let path: string | null = null;
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const result = await open({
          filters: [{ name: "WorldForge world", extensions: ["worldforge", "db"] }],
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
        // Restore the world's saved appearance palette (file value wins).
        try { const ap = await getAppearance(); useSettingsStore.getState().hydrate(ap ? JSON.parse(ap) : null); } catch { /* keep local palette */ }
        const ov = await getOverlays();
        if (ov.rivers?.length) setRivers(ov.rivers);
        if (ov.lakes?.length) setLakes(ov.lakes);
        if (ov.settlements?.length) setSettlements(ov.settlements);
        if (ov.economy && ov.economy.hubs.length) setEconomy(ov.economy);
        // #26 · toponyms (river/peak/lake/region names) — load them on OPEN so the
        // layer works in a campaign (they were only fetched in the Toponyms step).
        try { const tp = await getToponyms(); if (tp.length) { setToponyms(tp); useUIStore.getState().setOverlayVisible("toponyms", true); } } catch { /* none saved */ }
        // Province partition — restore the list + overlay raster if one was generated.
        try {
          const pl = await getProvinceLayer();
          if (pl.provinces?.length) {
            setProvinces(pl.provinces, decodeProvinceRaster(pl));
          }
        } catch { /* none saved */ }
        // Restore the persisted wizard progress; older saves never wrote it, so
        // fall back to inferring completion from which data is present. The two
        // halves are restored/inferred INDEPENDENTLY: `world_progress` (steps
        // 1-6) ships inside every .worldforge file, but `campaign_progress`
        // (steps 7-10) is deliberately a CAMPAIGN_RUN_KEY and is stripped by
        // `save_world_as` (rule 28) — so on a plain save-mid-generation →
        // reopen round trip, `world_progress` almost always restores something
        // while `campaign_progress` never does. A single combined
        // "restored.length > 0" gate meant the steps-7-10 inference never ran
        // once steps 1-6 had anything to restore, so reopening a world whose
        // settlements/economy WERE already generated silently showed those
        // steps as not done.
        const worldRestored = parseProgress(res.world_progress);
        const campaignRestored = parseProgress(res.campaign_progress);
        const worldSteps = worldRestored.length > 0
          ? worldRestored
          : (ov.rivers?.length ? [1, 2, 3, 4, 5, 6] : []);
        const campaignSteps = campaignRestored.length > 0
          ? campaignRestored
          : [
              ...(ov.settlements?.length ? [7] : []),
              ...((ov.economy && ov.economy.hubs.length) ? [8, 9, 10] : []),
            ];
        setStepsCompleted([...worldSteps, ...campaignSteps]);
        // A finalized (locked) world is read-only — jump straight to the campaign
        // side, but ONLY if it can actually start one. Landing in Chronicle with no
        // economy used to strand the user: Chronicle renders ChroniclePanel, not the
        // WorkflowPanel, so there was no step-10 button to recover with and "Begin
        // Campaign" could only throw. A world missing its economy opens in Forge, on
        // the step that rebuilds it.
        const playable = (ov.economy?.hubs.length ?? 0) >= 2;
        if (res.meta.frozen && playable) {
          setWorkflowStep(11 as never);
          setAppMode("chronicle");
        } else if (res.meta.frozen) {
          setWorkflowStep(10 as never);
          setAppMode("forge");
        } else {
          // An unfinalized world is a Forge product — never strand the UI in
          // Chronicle (e.g. left over from a previously-open frozen world).
          setAppMode("forge");
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
      await loadCampaignFrom(path);
    } catch (err) {
      console.error("Failed to open campaign:", err);
      alert("Failed to open campaign: " + err);
    }
  };

  /// Load one `.campaign` file and re-hydrate everything that hangs off it. Shared by
  /// the Open Campaign dialog and the campaign library, so a save opened either way
  /// lands in exactly the same state.
  const loadCampaignFrom = async (path: string) => {
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
    try { const tp = await getToponyms(); if (tp.length) { setToponyms(tp); useUIStore.getState().setOverlayVisible("toponyms", true); } } catch { /* none saved */ }
    const restored = parseProgress(info.campaign_progress);
    const ui = useUIStore.getState();
    const worldSteps = Object.entries(ui.stepCompleted)
      .filter(([k, v]) => v && Number(k) <= 6)
      .map(([k]) => Number(k));
    setStepsCompleted([...worldSteps, ...restored]);
    // A loaded campaign is a Chronicle product — show it there, and let the resident
    // sim be picked up by the panel's own snapshot read.
    setAppMode("chronicle");
    setStatus(`Campaign loaded: ${info.name}`);
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

  const handleNewCampaign = () => {
    if (!meta?.frozen) {
      alert("Finalize the world first (World wizard, after step 6).");
      return;
    }
    // NOT `prompt()`. Tauri does not implement window.prompt on any of its three
    // webviews — WKWebView and WebKit2GTK need a host-implemented text panel that
    // Tauri does not supply, and WebView2 has no prompt at all. It returns null,
    // the `if (!name) return` swallowed it, and starting a campaign — half the
    // product — was silently unreachable with no error shown.
    setShowCampaignDialog(true);
  };

  const startCampaign = async (name: string) => {
    setShowCampaignDialog(false);
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

        {/* Subproduct switch: Forge (build the world) ↔ Chronicle (play the
            living campaign). Chronicle unlocks only once the world is finalized. */}
        {isLoaded && (
          <div style={{ display: "flex", gap: 0, marginRight: 12,
            border: "1px solid #1e3450", borderRadius: 6, overflow: "hidden" }}>
            <button onClick={() => setAppMode("forge")} style={modeBtn(appMode === "forge")}
              title="World generation — paint & simulate the map (editable until finalized)">
              🛠 Forge
            </button>
            <button
              onClick={() => meta?.frozen && setAppMode("chronicle")}
              disabled={!meta?.frozen}
              style={{ ...modeBtn(appMode === "chronicle"), opacity: meta?.frozen ? 1 : 0.4,
                cursor: meta?.frozen ? "pointer" : "not-allowed" }}
              title={meta?.frozen
                ? "Living campaign — play the economy on the finalized world"
                : "Finish generation through Economy, then Finalize to enter Chronicle"}>
              📜 Chronicle
            </button>
          </div>
        )}

        <div style={{ display: "flex", gap: 2, marginLeft: 4 }}>
          <button onClick={() => setShowDialog(true)} style={headerBtn}>New</button>
          <button onClick={handleOpen} style={headerBtn}>Open</button>
          {isLoaded && appMode === "forge" && (
            <>
              <button onClick={handleSaveAs} style={headerBtn} title="Save the WORLD (geography/climate) to a .worldforge file">Save World</button>
              <button onClick={() => setShowImport(true)} style={headerBtn} title="Copy layers from another world file">Import Layers</button>
            </>
          )}
          {isLoaded && appMode === "chronicle" && (
            <>
              <button onClick={handleNewCampaign} style={headerBtn} title="Start a fresh campaign on this finalized world">New Campaign</button>
              {/* Campaign save/load made prominent (accent) — these resume a running
                  campaign from the exact year you saved, with all economy state. */}
              <button onClick={handleSaveCampaign} style={campaignBtn}
                title="Save the CAMPAIGN — resume later from this exact year with all houses, banks, trade and economy state intact">💾 Save Campaign</button>
              <button onClick={handleOpenCampaign} style={campaignBtn}
                title="Load a saved campaign and continue from the year it was saved">📂 Open Campaign</button>
              <button onClick={() => setShowLibrary(true)} style={campaignBtn}
                title="Browse your campaigns folder — every save, with the year it reached">📚 Campaigns</button>
            </>
          )}
          {/* GENERATION_UX_REDESIGN_PLAN.md Slice 8 (F10) — export is no
              longer Forge-only: the end-of-campaign map (realms, trade flows,
              plague, colonies…) is the one most worth printing. */}
          {isLoaded && (
            <button onClick={() => setShowExport(true)} style={headerBtn}>Export</button>
          )}
        </div>

        <div style={{ flex: 1 }} />
        {isLoaded && <SettlementSearch />}
        <button onClick={() => setShowSettings(true)} style={{ ...headerBtn, marginLeft: 6 }}
          title="Appearance settings (overlay colours)">⚙</button>
      </div>

      {/* Main layout: workflow | map | toolbar */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        {/* Left panel — the active subproduct: Forge = generation workflow,
            Chronicle = the living campaign clock. (Toggleable via the window bar.) */}
        {isLoaded && showWorkflow && (appMode === "forge" ? <WorkflowPanel /> : <ChroniclePanel />)}

        {/* Center: Map */}
        <div style={{ flex: 1, position: "relative", minWidth: 0, minHeight: 0, overflow: "hidden" }}>
          <ErrorBoundary label="Map"><MapCanvas /></ErrorBoundary>
          {/* Forge keeps the flat WindowBar; Chronicle gets the campaign HUD
              (clock + world pulse + grouped ledger menus). */}
          {isLoaded && (appMode === "chronicle" ? <CampaignTopBar /> : <WindowBar />)}
          {/* One broken panel (e.g. the Atlas) must not black out the whole app —
              show its error and clear the likely triggers (open Atlas / era frame). */}
          <ErrorBoundary label="Panels" onReset={() => {
            const s = useUIStore.getState();
            s.setShowAtlas(false);
            s.setEraFrame(null);
          }}>
          <InfoPanel />
          <HubPanel />
          <MarketsPanel />
          <SatelliteConstructionPanel />
          <GoodFlowPanel />
          <GoodsBrowserPanel />
          <HousesPanel />
          <MoneyFinancePanel />
          <WarPanel />
          <StatesPanel />
          <ItineraryPanel />
          <GoodsCodexPanel />
          <EconomyDashboardPanel />
          <HydrologyPanel />
          <ProvincePanel />
          <ProvinceInspector />
          <AtlasPanel />
          <PeoplesPanel />
          <ColonialPanel />
          <BankPanel />
          <NewsFeedPanel />
          <PlaguePanel />
          <ImmigrationPanel />
          <GuildsPanel />
          <FiguresPanel />
          <LandmarksPanel />
          <DynastiesPanel />
          <GoodsMarketPanel />
          <CityRankingPanel />
          <MerchantRoutePanel />
          <FuturesLanePanel />
          <FuturesPanel />
          <WarehousesPanel />
          <TradeMatrixPanel />
          <LayerLegend />
          <GoodQualityLegend />
          </ErrorBoundary>
        </div>

        {/* Right: Toolbar (toggleable via the window bar) */}
        {showToolbar && <Toolbar />}
      </div>

      <StatusBar />

      <GoodsEditor />
      <GoodsChainReview />
      <GoodDetailPanel />
      {showDialog && <NewWorldDialog onCreated={() => setShowDialog(false)} onOpenExisting={handleOpen} />}
      {showCampaignDialog && (
        <NewCampaignDialog
          worldName={meta?.name || "world"}
          onStart={startCampaign}
          onCancel={() => setShowCampaignDialog(false)}
        />
      )}
      {showExport && <ExportDialog name={meta?.name || "world"} onClose={() => setShowExport(false)} />}
      {showImport && <ImportWorldDialog onClose={() => setShowImport(false)} />}
      {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
      {showLibrary && (
        <CampaignLibraryPanel
          onClose={() => setShowLibrary(false)}
          onOpen={loadCampaignFrom}
          canSave={campaignActive}
        />
      )}
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
// Campaign save/load — accented so they're easy to find (the user couldn't tell
// the feature existed). Resume a campaign from the exact saved year.
const campaignBtn: React.CSSProperties = {
  padding: "4px 12px", borderRadius: 4, border: "1px solid #2f5a86",
  background: "#16324a", color: "#bcd9f4", cursor: "pointer", fontSize: 11,
  fontWeight: 600, transition: "background 0.1s",
};
// Subproduct switch segment — active segment reads as a filled accent tab.
const modeBtn = (active: boolean): React.CSSProperties => ({
  padding: "4px 12px", border: "none", cursor: "pointer", fontSize: 11,
  fontWeight: 600, transition: "background 0.1s",
  background: active ? "#2a5a8a" : "transparent",
  color: active ? "#eaf4ff" : "#6a8aaa",
});
