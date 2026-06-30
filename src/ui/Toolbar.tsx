import { useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useGoodsStore } from "../state/goodsStore";
import type { ActiveTool, ActiveLayer } from "../types";
import { GOOD_DEFS, goodOverlayKey, goodCategory, CATEGORY_ORDER } from "../goods";
import { LatitudeControl } from "./LatitudeControl";

const tools: { id: ActiveTool; label: string; icon: string; tip: string }[] = [
  { id: "pan", label: "Pan", icon: "\u270B", tip: "Click-drag to pan the map" },
  { id: "select", label: "Select", icon: "\u25CE", tip: "Click a cell to inspect it" },
  { id: "paint", label: "Paint", icon: "\u270F", tip: "Paint land, Shift = erase to sea" },
  { id: "elevation", label: "Elev", icon: "\u25B2", tip: "Paint elevation, Shift = flatten" },
  { id: "shelf", label: "Shelf", icon: "\u2248", tip: "Paint shelf, Shift = erase" },
  { id: "volcano", label: "Volc", icon: "\u25C6", tip: "Click to place volcano, Shift = remove" },
];

const layerGroups: { group: string; layers: { id: ActiveLayer; label: string }[] }[] = [
  {
    group: "Physical",
    layers: [
      { id: "land", label: "Land / Sea" },
      { id: "elevation", label: "Elevation" },
      { id: "terrain", label: "Hillshade" },
      { id: "plates", label: "Plates" },
    ],
  },
  {
    group: "Ocean",
    layers: [
      { id: "shelf", label: "Shelf" },
      { id: "fisheries", label: "Fisheries" },
      { id: "currents", label: "Currents" },
      { id: "salinity", label: "Salinity" },
      { id: "shark", label: "Shark Waters" },
      { id: "shipworm", label: "Shipworm Waters" },
      { id: "storm", label: "Storm Belts" },
      { id: "reef", label: "Reef Hazards" },
    ],
  },
  {
    group: "Atmosphere",
    layers: [
      { id: "temperature", label: "Temperature" },
      { id: "precipitation", label: "Precipitation" },
      { id: "wind", label: "Wind" },
    ],
  },
  {
    group: "Biosphere",
    layers: [
      { id: "climate", label: "Climate" },
      { id: "biomes", label: "Biomes" },
      { id: "soil", label: "Soil" },
      { id: "fertility", label: "Fertility" },
      { id: "habitability", label: "Habitability" },
      { id: "disease", label: "Disease (Malaria)" },
    ],
  },
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
  { id: "wind", label: "Wind" },
  { id: "currents", label: "Currents" },
  { id: "latLines", label: "Lat Lines" },
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
  { id: "merchantRoutes", label: "\u{1F6A2} Merchant Routes" },
  { id: "futures", label: "\u{1F4DC} Futures" },
  { id: "dynamicFlow", label: "\u{1F30A} Dynamic Trade Flow" },
  { id: "tradeRegions", label: "\u{1F7E6} Trade Regions" },
  { id: "tradeCorridors", label: "\u{2194} Trade Corridors" },
  { id: "chokepoints", label: "\u{2693} Chokepoints" },
  { id: "speculation", label: "\u{1FAE7} Speculation Risk" },
];

export function Toolbar() {
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
  const showBankIcons = useUIStore((s) => s.showBankIcons);
  const setShowBankIcons = useUIStore((s) => s.setShowBankIcons);
  const setGoodDetail = useUIStore((s) => s.setGoodDetail);
  const [expandedCats, setExpandedCats] = useState<Record<string, boolean>>({});
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

  const showBrush = activeTool === "paint" || activeTool === "elevation" || activeTool === "shelf";

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
          {tools.map((t) => (
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

      {/* Layers */}
      <div style={section}>
        <div style={sectionHeader}>Layers</div>
        {layerGroups.map((group) => (
          <div key={group.group} style={{ marginBottom: 4 }}>
            <div style={groupLabel}>{group.group}</div>
            {group.layers.map((l) => (
              <div
                key={l.id}
                onClick={() => setLayer(l.id)}
                style={{
                  padding: "3px 8px", borderRadius: 3, cursor: "pointer",
                  fontSize: 11, marginBottom: 1,
                  background: activeLayer === l.id ? "#1e3a58" : "transparent",
                  color: activeLayer === l.id ? "#c0ddf0" : "#6880a0",
                  fontWeight: activeLayer === l.id ? 600 : 400,
                  borderLeft: activeLayer === l.id ? "2px solid #4a90d0" : "2px solid transparent",
                }}
              >
                {l.label}
              </div>
            ))}
          </div>
        ))}
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
        <div style={sectionHeader}>Overlays</div>
        {overlayTypes.map((o) => (
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
      </div>

      {/* Dynamic latitude framing (move equator / expand 0–60 bands) */}
      <LatitudeControl />

      {/* Biological / political hazard + influence sublayers */}
      <div style={section}>
        <div style={sectionHeader}>Biological</div>
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
      </div>

      {/* Trade-good belts (each good is a separate sublayer toggle). Driven by the
          world's editable spec list, falling back to the static defaults. */}
      <div style={section}>
        <div style={sectionHeader}>Trade Goods</div>
        <button
          onClick={() => setShowGoodsBrowser(true)}
          style={{ width: "100%", marginBottom: 6, padding: "4px 6px", fontSize: 10,
            background: "#16243a", color: "#cfe0f4", border: "1px solid #2a3e58",
            borderRadius: 4, cursor: "pointer" }}>
          📖 Browse goods by origin
        </button>
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
      </div>

      <div style={divider} />

      {/* View */}
      <div style={section}>
        <div style={sectionHeader}>View</div>
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
      </div>
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
