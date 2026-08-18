import { useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useGoodsStore } from "@state/goodsStore";
import { useWorldStore } from "@state/worldStore";
import type { ActiveTool, ActiveLayer } from "@types";
import { GOOD_DEFS, goodOverlayKey, goodCategory, CATEGORY_ORDER } from "@goods";
import { MAP_THEMES, applyMapTheme, themeReady, layerReady, LAYER_REQUIRES } from "./mapThemes";
/** Best-effort CSS colour → #rrggbb for an <input type="color"> (which only takes
 *  hex). Understands #rgb/#rrggbb and rgb()/rgba(); falls back to dark grey. */
function hexOfBorder(css: string): string {
  const s = css.trim();
  if (s.startsWith("#")) {
    const m = s.slice(1);
    if (m.length === 3) return "#" + m.split("").map((c) => c + c).join("");
    if (m.length >= 6) return "#" + m.slice(0, 6);
  }
  const m = s.match(/rgba?\(\s*(\d+)[,\s]+(\d+)[,\s]+(\d+)/i);
  if (m) {
    const h = (n: string) => Math.max(0, Math.min(255, parseInt(n, 10))).toString(16).padStart(2, "0");
    return "#" + h(m[1]) + h(m[2]) + h(m[3]);
  }
  return "#0a0e14";
}

const tools: { id: ActiveTool; label: string; icon: string; tip: string }[] = [
  { id: "pan", label: "Pan", icon: "\u270B", tip: "Click-drag to pan the map" },
  { id: "select", label: "Select", icon: "\u25CE", tip: "Click a cell to inspect it" },
  { id: "paint", label: "Paint", icon: "\u270F", tip: "Paint land, Shift = erase to sea" },
  { id: "elevation", label: "Elev", icon: "\u25B2", tip: "Paint elevation, Shift = flatten" },
  { id: "shelf", label: "Shelf", icon: "\u2248", tip: "Paint shelf, Shift = erase" },
  { id: "volcano", label: "Volc", icon: "\u25C6", tip: "Click to place volcano, Shift = remove" },
];

/** Layers grouped by what they actually ARE.
 *
 *  The previous grouping misfiled in both directions: "Biosphere" held `climate`
 *  (Köppen is an atmospheric classification), `soil` (pedosphere) and
 *  `habitability` (a human settlement score), while "Ocean" held four biological
 *  layers plus `storm`, which is a cyclone belt and so atmospheric. The cause was
 *  under-population — the app has only TWO genuinely biological non-hazard layers,
 *  so a plausible-sized "Biosphere" group was made by sweeping in things that
 *  aren't biology.
 *
 *  Two changes carry that fix. HAZARDS becomes its own group: the five risk fields
 *  share one semantic ("where is it dangerous") and were split across two groups
 *  for no reason. SETTLEMENT becomes its own group: `soil → fertility →
 *  habitability` is literally the causal chain phases 6 and 7 compute.
 *
 *  `ridges` is listed here for the first time — it has always existed in
 *  `ActiveLayer` and in `render_tile` (`render_ridges`), but belonged to no group,
 *  so it was unreachable from the UI. */
const layerGroups: { group: string; layers: { id: ActiveLayer; label: string }[] }[] = [
  {
    group: "Terrain",
    layers: [
      { id: "land", label: "Land / Sea" },
      { id: "elevation", label: "Elevation" },
      { id: "terrain", label: "Hillshade" },
      { id: "ridges", label: "Ridges" },
      { id: "plates", label: "Plates" },
      { id: "shelf", label: "Continental Shelf" },
    ],
  },
  {
    group: "Ocean",
    layers: [
      { id: "currents", label: "Currents" },
      { id: "sst", label: "Sea-Surface Temp" },
      { id: "salinity", label: "Salinity" },
      { id: "fisheries", label: "Fisheries" },
    ],
  },
  {
    group: "Atmosphere",
    layers: [
      { id: "temperature", label: "Temperature" },
      { id: "precipitation", label: "Precipitation" },
      { id: "snow", label: "Snow Cover" },
      { id: "wind", label: "Wind" },
      { id: "windspeed", label: "Wind Speed" },
    ],
  },
  {
    group: "Climate & Biomes",
    layers: [
      { id: "climate", label: "Climate (Köppen)" },
      { id: "biomes", label: "Biomes" },
      { id: "natural", label: "Natural Colour" },
    ],
  },
  {
    group: "Settlement",
    layers: [
      { id: "soil", label: "Soil" },
      { id: "fertility", label: "Fertility" },
      { id: "habitability", label: "Habitability" },
    ],
  },
  {
    group: "Hazards",
    layers: [
      { id: "storm", label: "Storm Belts" },
      { id: "reef", label: "Reef Hazards" },
      { id: "shark", label: "Shark Waters" },
      { id: "shipworm", label: "Shipworm Waters" },
      { id: "disease", label: "Disease (Malaria)" },
    ],
  },
];

// Climate-circulation overlays (the winds, the ITCZ rain line, the belt latitudes
// and the reference lat lines) — grouped together since they all describe the
// atmosphere/circulation and respond to the Planet panel.
const climateOverlays = [
  { id: "itcz", label: "\u{1F327} ITCZ (rain line)" },
  { id: "windBelts", label: "\u{1F9ED} Circulation Belts" },
  { id: "wind", label: "Wind" },
  { id: "currents", label: "Currents" },
  { id: "latLines", label: "Lat Lines" },
];

const overlayTypes = [
  { id: "rivers", label: "Rivers" },
  { id: "lakes", label: "Lakes" },
  { id: "settlements", label: "Settlements" },
  { id: "colonies", label: "⛶ Colonies" },
  { id: "settlementNames", label: "Settlement Names" },
  { id: "hubNames", label: "Hub Names" },
  { id: "cultures", label: "\u{1F465} Peoples" },
  { id: "tradeRoutes", label: "Trade Routes" },
  { id: "tradeFlows", label: "Trade Flows" },
  { id: "fisheryBanks", label: "Fishery Banks" },
  { id: "markers", label: "Volcanoes" },
];

// Biological / political sublayer overlays (grouped separately).
const bioOverlays = [
  { id: "sharkZones", label: "\u{1F988} Shark Zones" },
  { id: "shipwormZones", label: "\u{1FAB1} Shipworm Zones" },
  { id: "reefZones", label: "\u{1FAA8} Reef Zones" },
  // Natural Disasters — hurricane belts (sea) + monsoon flood areas (land).
  { id: "stormZones", label: "\u{1F300} Hurricanes" },
  { id: "monsoonZones", label: "\u{1F327} Monsoon Areas" },
  { id: "politicalInfluence", label: "\u{1F535} Trade Hubs" },
  { id: "houseControl", label: "\u{2696} House Control" },
  { id: "states", label: "\u{1F451} Realms" },
  { id: "merchantRoutes", label: "\u{1F6A2} Merchant Routes" },
  { id: "futures", label: "\u{1F4DC} Futures" },
  { id: "dynamicFlow", label: "\u{1F30A} Dynamic Trade Flow" },
  { id: "campaignCorridors", label: "\u{1F42B} Caravan Corridors" },
  { id: "expeditions", label: "\u{26F5} Expeditions" },
  { id: "tradeHeat", label: "\u{1F525} Trade Heat" },
  { id: "tradeBasins", label: "\u{1F3DE} Trade Basins" },
  { id: "migrations", label: "\u{1F6B6} Migrations" },
  { id: "tradeRegions", label: "\u{1F7E6} Trade Regions" },
  { id: "tradeCorridors", label: "\u{2194} Trade Corridors" },
  { id: "chokepoints", label: "\u{2693} Chokepoints" },
  { id: "speculation", label: "\u{1FAE7} Speculation Risk" },
  { id: "coinDominance", label: "\u{1FA99} Coin Dominance" },
  { id: "plagueZones", label: "\u{2620} Plague Zones" },
  { id: "guildCities", label: "\u{1F3DB} Guild Cities" },
  { id: "figureMarks", label: "\u{269C} Notable Figures" },
  { id: "landmarks", label: "\u{1F5FF} Landmarks" },
  { id: "dynastyLinks", label: "\u{26AD} Dynasty Ties" },
];

export function Toolbar() {
  const appMode = useUIStore((s) => s.appMode);
  const activeTool = useUIStore((s) => s.activeTool);
  const activeLayer = useUIStore((s) => s.activeLayer);
  const brushRadius = useUIStore((s) => s.brushRadius);
  const elevationValue = useUIStore((s) => s.elevationValue);
  const overlayVisibility = useUIStore((s) => s.overlayVisibility);
  const layerOpacity = useUIStore((s) => s.layerOpacity);
  const setTool = useUIStore((s) => s.setTool);
  const setLayer = useUIStore((s) => s.setLayer);
  const setBrushRadius = useUIStore((s) => s.setBrushRadius);
  const setElevationValue = useUIStore((s) => s.setElevationValue);
  const toggleOverlay = useUIStore((s) => s.toggleOverlay);
  const setOverlaysVisible = useUIStore((s) => s.setOverlaysVisible);
  const goodQualityHeatmap = useUIStore((s) => s.goodQualityHeatmap);
  const toggleGoodQualityHeatmap = useUIStore((s) => s.toggleGoodQualityHeatmap);
  const migrationMode = useUIStore((s) => s.migrationMode);
  const setMigrationMode = useUIStore((s) => s.setMigrationMode);
  const showBankIcons = useUIStore((s) => s.showBankIcons);
  const setShowBankIcons = useUIStore((s) => s.setShowBankIcons);
  const setGoodDetail = useUIStore((s) => s.setGoodDetail);
  const activeMapTheme = useUIStore((s) => s.activeMapTheme);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const activePlate = MAP_THEMES.find((t) => t.id === activeMapTheme);
  const compareLayer = useUIStore((s) => s.compareLayer);
  const setCompareLayer = useUIStore((s) => s.setCompareLayer);
  const [expandedCats, setExpandedCats] = useState<Record<string, boolean>>({});
  // Collapsible top-level sections (the big lists start collapsed to declutter).
  const [openSection, setOpenSection] = useState<Record<string, boolean>>({
    Plates: true, Layers: true, Overlays: true, Climate: true, Biological: false, "Trade Goods": false, Provinces: false, View: true,
  });
  const toggleSection = (k: string) => setOpenSection((s) => ({ ...s, [k]: !s[k] }));
  const bioParams = useUIStore((s) => s.bioParams);
  const setBioParams = useUIStore((s) => s.setBioParams);
  const hubDisplay = useUIStore((s) => s.hubDisplay);
  const setHubDisplay = useUIStore((s) => s.setHubDisplay);
  const goodsSpecs = useGoodsStore((s) => s.specs);
  const goodItems = goodsSpecs.length > 0
    // Manufactured goods are made in cities, not grown in a belt — they have no map
    // overlay (the backend emits no region for them), so hide their toggle here.
    ? goodsSpecs.filter((g) => g.enabled && g.distribution !== "manufactured")
        .map((g) => ({ id: g.id, icon: g.icon, name: g.name }))
    : GOOD_DEFS.map((g) => ({ id: g.name, icon: g.emoji, name: g.label }));
  const setLayerOpacity = useUIStore((s) => s.setLayerOpacity);
  const stretchToFit = useUIStore((s) => s.stretchToFit);
  const setStretchToFit = useUIStore((s) => s.setStretchToFit);
  const setShowGoodsBrowser = useUIStore((s) => s.setShowGoodsBrowser);
  const showProvinces = useUIStore((s) => s.showProvinces);
  const setShowProvinces = useUIStore((s) => s.setShowProvinces);
  const provinces = useWorldStore((s) => s.provinces);
  const provinceOpacity = useUIStore((s) => s.provinceOpacity);
  const setProvinceOpacity = useUIStore((s) => s.setProvinceOpacity);
  const provinceFillMode = useUIStore((s) => s.provinceFillMode);
  const setProvinceFillMode = useUIStore((s) => s.setProvinceFillMode);
  const provinceSingleColor = useUIStore((s) => s.provinceSingleColor);
  const setProvinceSingleColor = useUIStore((s) => s.setProvinceSingleColor);
  const provinceBorderColor = useUIStore((s) => s.provinceBorderColor);
  const setProvinceBorderColor = useUIStore((s) => s.setProvinceBorderColor);

  const showBrush = activeTool === "paint" || activeTool === "elevation" || activeTool === "shelf";

  // In Chronicle the map is a finalized, read-only stage — hide the paint/sculpt
  // tools (they mutate geography) and keep only the non-destructive Pan / Select.
  const visibleTools = appMode === "chronicle"
    ? tools.filter((t) => t.id === "pan" || t.id === "select")
    : tools;

  return (
    <div style={{
      display: "flex", flexDirection: "column", gap: 0,
      background: "#0d1219", borderLeft: "1px solid #1e2a38",
      width: 170, overflowY: "auto", fontSize: 11,
    }}>
      {/* Tools row */}
      <div style={section}>
        <div style={sectionHeader}>Tools</div>
        <div style={{ display: "flex", gap: 3, flexWrap: "wrap" }}>
          {visibleTools.map((t) => (
            <button
              key={t.id}
              onClick={() => setTool(t.id)}
              title={t.tip}
              style={{
                padding: "4px 0", border: "none", borderRadius: 4, cursor: "pointer",
                fontSize: 11, width: 48, textAlign: "center",
                background: activeTool === t.id ? "#2a5080" : "#151d28",
                color: activeTool === t.id ? "#e0eeff" : "#7088a0",
                fontWeight: activeTool === t.id ? 600 : 400,
              }}
            >
              <div style={{ fontSize: 14, lineHeight: "16px" }}>{t.icon}</div>
              <div style={{ fontSize: 9 }}>{t.label}</div>
            </button>
          ))}
        </div>
      </div>

      {/* Brush controls */}
      {showBrush && (
        <div style={section}>
          <div style={sliderRow}>
            <span style={sliderLabel}>Brush</span>
            <span style={sliderValue}>{brushRadius}</span>
          </div>
          <input
            type="range" min={1} max={20} value={brushRadius}
            onChange={(e) => setBrushRadius(Number(e.target.value))}
            style={rangeStyle}
          />
        </div>
      )}

      {activeTool === "elevation" && (
        <div style={section}>
          <div style={sliderRow}>
            <span style={sliderLabel}>Height</span>
            <span style={sliderValue}>{Math.round(elevationValue * 8848)}m</span>
          </div>
          <input
            type="range" min={0} max={100} value={Math.round(elevationValue * 100)}
            onChange={(e) => setElevationValue(Number(e.target.value) / 100)}
            style={rangeStyle}
          />
        </div>
      )}

      <div style={divider} />

      {/* Map plates — a whole composition (base raster + overlays + label
          typography), the way an atlas is actually organised. Display-only: a
          plate sets what you SEE, never what the world IS (rule 14). */}
      <div style={section}>
        <SectionHead title="Plates" open={openSection.Plates} onToggle={() => toggleSection("Plates")} />
        {openSection.Plates && (<>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 3 }}>
            {MAP_THEMES.map((t) => {
              const ready = themeReady(t, stepCompleted);
              const on = activeMapTheme === t.id;
              return (
                <button
                  key={t.id}
                  onClick={() => { if (ready) applyMapTheme(t); }}
                  disabled={!ready}
                  title={ready ? t.blurb : `Needs step ${t.requires} — not generated yet`}
                  style={{
                    display: "flex", alignItems: "center", gap: 5, overflow: "hidden",
                    padding: "4px 6px", borderRadius: 4, fontSize: 10.5, textAlign: "left",
                    cursor: ready ? "pointer" : "not-allowed",
                    border: `1px solid ${on ? "#4a90d0" : "#20304a"}`,
                    background: on ? "#1e3a58" : "#0e1826",
                    color: !ready ? "#3d4c5e" : on ? "#cfe2f6" : "#7d97b0",
                    fontWeight: on ? 600 : 400,
                  }}
                >
                  <span style={{ fontSize: 12, lineHeight: 1, opacity: ready ? 1 : 0.5 }}>{t.glyph}</span>
                  <span style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{t.name}</span>
                </button>
              );
            })}
          </div>
          <div style={{ fontSize: 9, color: "#5a7390", marginTop: 5, lineHeight: 1.45 }}>
            {activePlate
              ? activePlate.blurb
              : "Custom view. A plate composes base layer, overlays and label typography together."}
          </div>
        </>)}
      </div>

      <div style={divider} />

      {/* Layers */}
      <div style={section}>
        <SectionHead title="Layers" open={openSection.Layers} onToggle={() => toggleSection("Layers")} />
        {openSection.Layers && layerGroups.map((group) => (
          <div key={group.group} style={{ marginBottom: 4 }}>
            <div style={groupLabel}>{group.group}</div>
            {group.layers.map((l) => {
              // A layer whose phase hasn't run renders blank or stale with no
              // explanation — the most confusing first-run moment in the app. Dim
              // it and name the step instead of letting it look broken. Still
              // clickable: seeing what the finished world will offer is the point.
              const ready = layerReady(l.id, stepCompleted);
              return (
                <div
                  key={l.id}
                  onClick={() => setLayer(l.id)}
                  title={ready ? undefined : `Needs step ${LAYER_REQUIRES[l.id]} — not generated yet`}
                  style={{
                    display: "flex", alignItems: "center", gap: 6,
                    padding: "3px 8px", borderRadius: 3, cursor: "pointer",
                    fontSize: 11, marginBottom: 1,
                    background: activeLayer === l.id ? "#1e3a58" : "transparent",
                    color: activeLayer === l.id ? "#c0ddf0" : ready ? "#6880a0" : "#3d4c5e",
                    fontWeight: activeLayer === l.id ? 600 : 400,
                    borderLeft: activeLayer === l.id ? "2px solid #4a90d0" : "2px solid transparent",
                  }}
                >
                  <span style={{ flex: 1, minWidth: 0 }}>{l.label}</span>
                  {!ready && (
                    <span style={{ fontSize: 8.5, color: "#33445a", fontVariantNumeric: "tabular-nums" }}>
                      step {LAYER_REQUIRES[l.id]}
                    </span>
                  )}
                </div>
              );
            })}
          </div>
        ))}
      </div>

      {/* SWIPE COMPARE — every causal chain here is a two-layer question
          (precipitation vs elevation for rain shadow, currents vs temperature,
          biomes vs Köppen), and they used to be answered by flipping back and
          forth from memory. Display-only, like everything else in this column. */}
      <div style={section}>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={sliderLabel}>Compare</span>
          <select
            value={compareLayer ?? ""}
            onChange={(e) => setCompareLayer((e.target.value || null) as ActiveLayer | null)}
            style={{
              flex: 1, minWidth: 0, background: "#0e1826", color: "#a8bed4",
              border: "1px solid #20304a", borderRadius: 4, fontSize: 10, padding: "2px 4px",
            }}
          >
            <option value="">off</option>
            {layerGroups.flatMap((g) =>
              g.layers
                .filter((l) => l.id !== activeLayer)
                .map((l) => <option key={l.id} value={l.id}>{l.label}</option>)
            )}
          </select>
        </div>
        {compareLayer && (
          <div style={{ fontSize: 9, color: "#5a7390", marginTop: 4, lineHeight: 1.4 }}>
            Drag the divider on the map. {compareLayer} draws to its right, over the
            same ground.
          </div>
        )}
      </div>

      {/* Layer opacity */}
      <div style={section}>
        <div style={sliderRow}>
          <span style={sliderLabel}>Opacity</span>
          <span style={sliderValue}>{Math.round(layerOpacity * 100)}%</span>
        </div>
        <input
          type="range" min={0} max={100} value={Math.round(layerOpacity * 100)}
          onChange={(e) => setLayerOpacity(Number(e.target.value) / 100)}
          style={rangeStyle}
        />
      </div>

      <div style={divider} />

      {/* Overlays */}
      <div style={section}>
        <SectionHead title="Overlays" open={openSection.Overlays} onToggle={() => toggleSection("Overlays")} />
        {openSection.Overlays && (<>
        {/* The trade map means different things per mode: in Forge the routes/flows
            are the worldgen equilibrium *preview* (potential); in Chronicle the
            routes remain but flow activity is the *live* simulated volume. */}
        <div style={{ fontSize: 9, color: "#5a7390", marginBottom: 5, lineHeight: 1.4 }}>
          {appMode === "chronicle"
            ? "Trade: routes inherited from the world · flow = live campaign volume (Dynamic Trade Flow)"
            : "Trade routes & flows show the worldgen equilibrium (potential)"}
        </div>
        {overlayTypes.map((o) => (
          <div key={o.id}>
            <label style={checkboxRow}>
              <input
                type="checkbox"
                checked={!!overlayVisibility[o.id]}
                onChange={() => toggleOverlay(o.id)}
                style={{ accentColor: "#4a90d0", width: 12, height: 12 }}
              />
              <span style={{ color: overlayVisibility[o.id] ? "#b0c8e0" : "#5a6a80" }}>
                {o.label}
              </span>
            </label>
            {/* Route-bound migration: ribbon / dots / focus mode (people follow trade routes). */}
            {o.id === "migrations" && overlayVisibility[o.id] && (
              <div style={{ display: "flex", gap: 3, margin: "1px 0 4px 22px" }}>
                {(["ribbon", "dots", "focus"] as const).map((m) => (
                  <button key={m} onClick={() => setMigrationMode(m)}
                    style={{
                      flex: 1, fontSize: 9.5, padding: "2px 0", borderRadius: 4, cursor: "pointer",
                      border: `1px solid ${migrationMode === m ? "#4a90d0" : "#26374d"}`,
                      background: migrationMode === m ? "#16324a" : "#0e1826",
                      color: migrationMode === m ? "#cfe2f6" : "#6a86a6",
                    }}
                    title={m === "focus" ? "Only the selected city's inbound flows" : m === "dots" ? "Dots riding the trade routes" : "Culture ribbons, width = volume"}>
                    {m}
                  </button>
                ))}
              </div>
            )}
          </div>
        ))}
        {/* Bank seats — a separate campaign overlay (not a worldgen layer). */}
        <label style={checkboxRow}>
          <input
            type="checkbox"
            checked={showBankIcons}
            onChange={() => setShowBankIcons(!showBankIcons)}
            style={{ accentColor: "#4a90d0", width: 12, height: 12 }}
          />
          <span style={{ color: showBankIcons ? "#b0c8e0" : "#5a6a80" }}>{"\u{1F3E6}"} Banks</span>
        </label>
        </>)}
      </div>

      {/* Climate circulation overlays (ITCZ line + rotation-driven belts + winds).
          Sits beside the Planet panel below, since both describe the atmosphere. */}
      <div style={section}>
        <SectionHead title="Climate" open={openSection.Climate} onToggle={() => toggleSection("Climate")} />
        {openSection.Climate && (
          <div>
            <div style={{ fontSize: 9, color: "#5a7390", marginBottom: 4, lineHeight: 1.4 }}>
              Circulation bands from the planet settings — the ITCZ rain line and the
              subtropical-high / polar-front belts move with rotation &amp; warmth.
              Set them in <b>Workflow step 0 · World Characteristics</b> (left panel),
              which now holds every generation setting; this column is display only.
            </div>
            {climateOverlays.map((o) => (
              <label key={o.id} style={checkboxRow}>
                <input
                  type="checkbox"
                  checked={!!overlayVisibility[o.id]}
                  onChange={() => toggleOverlay(o.id)}
                  style={{ accentColor: "#4a90d0", width: 12, height: 12 }}
                />
                <span style={{ color: overlayVisibility[o.id] ? "#b0c8e0" : "#5a6a80" }}>
                  {o.label}
                </span>
              </label>
            ))}
          </div>
        )}
      </div>

      {/* NOTE: the latitude framing and the planetary knobs used to live here.
          They are GENERATION settings (they change what the simulation produces),
          so they now live in one place on the LEFT, in Workflow step 0 "World
          Characteristics". This column keeps display-only options. */}

      {/* Biological / political hazard + influence sublayers */}
      <div style={section}>
        <SectionHead title="Biological" open={openSection.Biological} onToggle={() => toggleSection("Biological")} />
        {openSection.Biological && (<>
        {bioOverlays.map((o) => (
          <label key={o.id} style={checkboxRow}>
            <input
              type="checkbox"
              checked={!!overlayVisibility[o.id]}
              onChange={() => toggleOverlay(o.id)}
              style={{ accentColor: "#4a90d0", width: 12, height: 12 }}
            />
            <span style={{ color: overlayVisibility[o.id] ? "#b0c8e0" : "#5a6a80" }}>
              {o.label}
            </span>
          </label>
        ))}
        {/* Seasonal storm month: 0 = combined annual extent, 1..months scrubs
            the cyclone season (zones fade out in their calm months). */}
        <div style={{ marginTop: 4, opacity: overlayVisibility.stormZones ? 1 : 0.5 }}>
          <div style={{ fontSize: 10, color: "#8aa0b8", display: "flex", justifyContent: "space-between" }}>
            <span>{"\u{1F300}"} Storm month</span>
            <span style={{ color: "#b0c8e0" }}>
              {bioParams.stormMonth === 0 ? "All year" : `Moon ${bioParams.stormMonth}`}
            </span>
          </div>
          <input
            type="range"
            min={0}
            max={bioParams.calendarMonths}
            step={1}
            value={bioParams.stormMonth}
            onChange={(e) => setBioParams({ stormMonth: parseInt(e.target.value, 10) })}
            style={{ width: "100%", accentColor: "#c050d0" }}
          />
        </div>

        {/* Trade-hub marker display: size + highlight intensity (affects the
            "Trade Hubs" overlay markers across the map). */}
        <div style={{ marginTop: 6, opacity: overlayVisibility.politicalInfluence ? 1 : 0.5 }}>
          <div style={{ fontSize: 10, color: "#8aa0b8", display: "flex", justifyContent: "space-between" }}>
            <span>{"\u{1F535}"} Hub size</span>
            <span style={{ color: "#b0c8e0" }}>{hubDisplay.size.toFixed(1)}×</span>
          </div>
          <input type="range" min={0.5} max={4} step={0.1} value={hubDisplay.size}
            onChange={(e) => setHubDisplay({ size: parseFloat(e.target.value) })}
            style={{ width: "100%", accentColor: "#3a86d6" }} />
          <div style={{ fontSize: 10, color: "#8aa0b8", display: "flex", justifyContent: "space-between" }}>
            <span>Hub highlight</span>
            <span style={{ color: "#b0c8e0" }}>{Math.round(hubDisplay.intensity * 100)}%</span>
          </div>
          <input type="range" min={0} max={1} step={0.05} value={hubDisplay.intensity}
            onChange={(e) => setHubDisplay({ intensity: parseFloat(e.target.value) })}
            style={{ width: "100%", accentColor: "#3a86d6" }} />
        </div>
        </>)}
      </div>

      {/* Trade-good belts (each good is a separate sublayer toggle). Driven by the
          world's editable spec list, falling back to the static defaults. */}
      <div style={section}>
        <SectionHead title="Trade Goods" open={openSection["Trade Goods"]} onToggle={() => toggleSection("Trade Goods")} />
        {openSection["Trade Goods"] && (<>
        <button
          onClick={() => setShowGoodsBrowser(true)}
          style={{ width: "100%", marginBottom: 6, padding: "4px 6px", fontSize: 10,
            background: "#16243a", color: "#cfe0f4", border: "1px solid #2a3e58",
            borderRadius: 4, cursor: "pointer" }}>
          📖 Browse goods by origin
        </button>
        {/* GOODS_LOCALITIES_PLAN.md D9 · the TWO layers a belt carries, switched
            independently of each other and of WHICH goods are shown below. Coverage
            = "can it grow here" (the belt's honest extent + its outline, so a belt
            thinned by the locality pass is visible on the map rather than buried in
            a diagnostic, §5.1); quality = "is it fine here", shaded on ONE absolute
            0–1 scale shared by every good (D10) — never per-good normalised. Two
            switches rather than two per good: ~90 checkboxes would be unusable. */}
        <div style={{ display: "flex", gap: 4, marginBottom: 5 }}>
          {([["goodCoverage", "coverage", "Where the belt reaches — its honest extent, outlined along the coastline"],
             ["goodQuality", "quality", "How good the land is, on one absolute scale shared by every good"]] as const)
            .map(([key, label, title]) => {
              const active = !!overlayVisibility[key];
              return (
                <span key={key} onClick={() => toggleOverlay(key)} title={title}
                  style={{
                    fontSize: 10, padding: "1px 7px", borderRadius: 3, userSelect: "none",
                    cursor: "pointer", flex: 1, textAlign: "center",
                    color: active ? "#0a1620" : "#7d93ab",
                    background: active ? "#7fa8c8" : "transparent",
                    border: `1px solid ${active ? "#7fa8c8" : "#2a3e58"}`,
                  }}>
                  {active ? "☑" : "☐"} {label}
                </span>
              );
            })}
        </div>
        {/* The quality layer's PAINT: hue-mix (toward the good's own colour, the
            default) or a heat ramp (dark blue = poorest land, red = finest) that
            ignores the good's colour entirely. Added because a muted or dark good's
            hue-mix reading can be too close to the pale ground tint to show any
            change at all — the heat ramp has no such blind spot, since every good's
            best land is red and worst is blue however that good happens to be
            coloured on the map. Same numbers, different paint; only matters while
            "quality" above is on. */}
        {overlayVisibility.goodQuality && (
          <div
            onClick={toggleGoodQualityHeatmap}
            title="Quality paint: hue-mix (toward the good's own colour) vs. a heat ramp (blue = poor, red = fine) that reads more clearly for muted colours"
            style={{
              display: "flex", alignItems: "center", gap: 5, cursor: "pointer",
              fontSize: 9.5, color: "#93a8bd", marginBottom: 6, padding: "1px 2px",
              userSelect: "none",
            }}>
            <span>Quality paint:</span>
            <span style={{ color: !goodQualityHeatmap ? "#cfe0f4" : "#5f7390", fontWeight: !goodQualityHeatmap ? 600 : 400 }}>
              hue
            </span>
            <span style={{
              position: "relative", width: 26, height: 13, borderRadius: 7,
              background: goodQualityHeatmap ? "#3a5a86" : "#243449",
              border: "1px solid #2a3e58", flex: "0 0 auto",
            }}>
              <span style={{
                position: "absolute", top: 1, left: goodQualityHeatmap ? 13 : 1,
                width: 11, height: 11, borderRadius: "50%",
                background: goodQualityHeatmap
                  ? "linear-gradient(135deg, #c42020, #f0d640)"
                  : "linear-gradient(135deg, #7fa8c8, #eceae0)",
                transition: "left 0.12s ease",
              }} />
            </span>
            <span style={{ color: goodQualityHeatmap ? "#cfe0f4" : "#5f7390", fontWeight: goodQualityHeatmap ? 600 : 400 }}>
              🌡 heatmap
            </span>
          </div>
        )}
        {/* One toggle per category (master checkbox shows all its goods, each in
            its own colour/icon); expand the caret to toggle a single good. */}
        {CATEGORY_ORDER.map((cat) => {
          const items = goodItems.filter((g) => goodCategory(g.id) === cat);
          if (items.length === 0) return null;
          const keys = items.map((g) => goodOverlayKey(g.id));
          const shownCount = keys.filter((k) => overlayVisibility[k]).length;
          const allOn = shownCount === keys.length;
          const someOn = shownCount > 0 && !allOn;
          const expanded = !!expandedCats[cat];
          return (
            <div key={cat} style={{ marginBottom: 2 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 4, margin: "3px 0 1px" }}>
                <span
                  onClick={() => setExpandedCats((e) => ({ ...e, [cat]: !e[cat] }))}
                  style={{ cursor: "pointer", color: "#5f7390", fontSize: 9, width: 9, userSelect: "none" }}
                  title={expanded ? "Collapse" : "Expand to toggle individual goods"}
                >{expanded ? "▼" : "▶"}</span>
                <input
                  type="checkbox"
                  checked={allOn}
                  ref={(el) => { if (el) el.indeterminate = someOn; }}
                  onChange={() => setOverlaysVisible(keys, !allOn)}
                  style={{ accentColor: "#4a90d0", width: 12, height: 12 }}
                />
                <span
                  onClick={() => setExpandedCats((e) => ({ ...e, [cat]: !e[cat] }))}
                  style={{ cursor: "pointer", color: shownCount > 0 ? "#9fb6d0" : "#5f7390",
                    fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4, flex: 1, userSelect: "none" }}
                >
                  {cat}{shownCount > 0 ? ` (${shownCount}/${keys.length})` : ""}
                </span>
              </div>
              {expanded && items.map((g) => {
                const key = goodOverlayKey(g.id);
                return (
                  <div key={key} style={{ ...checkboxRow, paddingLeft: 15 }}>
                    <input
                      type="checkbox"
                      checked={!!overlayVisibility[key]}
                      onChange={() => toggleOverlay(key)}
                      style={{ accentColor: "#4a90d0", width: 12, height: 12 }}
                    />
                    <span
                      onClick={() => setGoodDetail(g.id)}
                      title="Show seeding climates & heatmap"
                      style={{ color: overlayVisibility[key] ? "#b0c8e0" : "#5a6a80", cursor: "pointer", flex: 1 }}
                    >
                      {g.icon} {g.name}
                    </span>
                  </div>
                );
              })}
            </div>
          );
        })}
        </>)}
      </div>

      {/* Province partition overlay */}
      <div style={section}>
        <SectionHead title="Provinces" open={openSection.Provinces} onToggle={() => toggleSection("Provinces")} />
        {openSection.Provinces && (
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label style={checkboxRow}>
              <input
                type="checkbox"
                checked={!!overlayVisibility.provinces}
                onChange={() => toggleOverlay("provinces")}
                style={{ accentColor: "#4a90d0", width: 12, height: 12 }}
                disabled={provinces.length === 0}
              />
              <span style={{ color: overlayVisibility.provinces ? "#b0c8e0" : "#5a6a80" }}>
                🗺 Culture fill + borders
              </span>
            </label>
            {provinces.length === 0 && (
              <div style={{ fontSize: 9, color: "#5a7390", lineHeight: 1.4 }}>
                No provinces yet. Open the Provinces window (top bar) after Step 7 to generate them.
              </div>
            )}
            {provinces.length > 0 && (
              <div style={{ fontSize: 9, color: "#5a7390" }}>
                {provinces.length} provinces · click a cell to inspect
              </div>
            )}
            {/* Fill opacity — borders and names keep full strength, so winding this
                to 0 leaves a clean political outline over the terrain. */}
            {provinces.length > 0 && overlayVisibility.provinces && (
              <div style={{ marginTop: 2 }}>
                <div style={sliderRow}>
                  <span style={sliderLabel}>Fill opacity</span>
                  <span style={sliderValue}>{Math.round(provinceOpacity * 100)}%</span>
                </div>
                <input
                  type="range" min={0} max={100} value={Math.round(provinceOpacity * 100)}
                  onChange={(e) => setProvinceOpacity(Number(e.target.value) / 100)}
                  style={rangeStyle}
                />
                {/* Fill style: distinct colours vs one flat colour + a custom border,
                    so the map can read as a clean single-tone political sheet. */}
                <label style={{ ...checkboxRow, marginTop: 4 }}>
                  <input
                    type="checkbox"
                    checked={provinceFillMode === "single"}
                    onChange={(e) => setProvinceFillMode(e.target.checked ? "single" : "distinct")}
                  />
                  Single fill colour
                </label>
                <div style={{ display: "flex", gap: 10, marginTop: 2, alignItems: "center" }}>
                  {provinceFillMode === "single" && (
                    <label style={{ ...sliderLabel, display: "flex", alignItems: "center", gap: 4 }}>
                      Fill
                      <input
                        type="color" value={provinceSingleColor}
                        onChange={(e) => setProvinceSingleColor(e.target.value)}
                        style={{ width: 24, height: 18, padding: 0, border: "none", background: "none" }}
                      />
                    </label>
                  )}
                  <label style={{ ...sliderLabel, display: "flex", alignItems: "center", gap: 4 }}>
                    Border
                    <input
                      type="color" value={hexOfBorder(provinceBorderColor)}
                      onChange={(e) => setProvinceBorderColor(e.target.value)}
                      style={{ width: 24, height: 18, padding: 0, border: "none", background: "none" }}
                    />
                  </label>
                </div>
              </div>
            )}
            <button
              onClick={() => setShowProvinces(!showProvinces)}
              style={{
                padding: "3px 6px", fontSize: 10, borderRadius: 4, cursor: "pointer",
                border: `1px solid ${showProvinces ? "#3a80c0" : "#1e2e42"}`,
                background: showProvinces ? "#19324a" : "#0e1826",
                color: showProvinces ? "#cfe2f6" : "#6a86a6",
              }}>
              {showProvinces ? "Close" : "Open"} Province Panel
            </button>
          </div>
        )}
      </div>

      <div style={divider} />

      {/* View */}
      <div style={section}>
        <SectionHead title="View" open={openSection.View} onToggle={() => toggleSection("View")} />
        {openSection.View && (
        <label style={checkboxRow}>
          <input
            type="checkbox"
            checked={stretchToFit}
            onChange={(e) => setStretchToFit(e.target.checked)}
            style={{ accentColor: "#4a90d0", width: 12, height: 12 }}
          />
          <span style={{ color: stretchToFit ? "#b0c8e0" : "#5a6a80" }}>
            Stretch to fill
          </span>
        </label>
        )}
      </div>
    </div>
  );
}

/** Clickable, collapsible section header with a caret. */
function SectionHead({ title, open, onToggle }: { title: string; open: boolean; onToggle: () => void }) {
  return (
    <div
      onClick={onToggle}
      style={{ ...sectionHeader, display: "flex", alignItems: "center", justifyContent: "space-between", cursor: "pointer", userSelect: "none" }}
      title={open ? "Collapse" : "Expand"}
    >
      <span>{title}</span>
      <span style={{ fontSize: 8 }}>{open ? "▼" : "▶"}</span>
    </div>
  );
}

const section: React.CSSProperties = {
  padding: "6px 8px",
};

const sectionHeader: React.CSSProperties = {
  fontSize: 10, color: "#4a6a8a", textTransform: "uppercase", letterSpacing: 1.2,
  marginBottom: 5, fontWeight: 600,
};

const groupLabel: React.CSSProperties = {
  fontSize: 9, color: "#3a4e64", marginTop: 2, marginBottom: 1, paddingLeft: 8,
  textTransform: "uppercase", letterSpacing: 0.8,
};

const divider: React.CSSProperties = {
  height: 1, background: "#1a2535", margin: "2px 8px",
};

const sliderRow: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 2,
};

const sliderLabel: React.CSSProperties = {
  fontSize: 10, color: "#5a7090",
};

const sliderValue: React.CSSProperties = {
  fontSize: 10, color: "#8aa0c0", fontFamily: "monospace",
};

const rangeStyle: React.CSSProperties = {
  width: "100%", height: 4, cursor: "pointer",
};

const checkboxRow: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 6, cursor: "pointer",
  padding: "2px 0", fontSize: 11,
};
