import { useUIStore } from "../state/uiStore";
import type { ActiveTool, ActiveLayer } from "../types";
import { GOOD_DEFS, goodOverlayKey } from "../goods";

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
    ],
  },
];

const overlayTypes = [
  { id: "rivers", label: "Rivers" },
  { id: "lakes", label: "Lakes" },
  { id: "settlements", label: "Settlements" },
  { id: "tradeRoutes", label: "Trade Routes" },
  { id: "tradeFlows", label: "Trade Flows" },
  { id: "fisheryBanks", label: "Fishery Banks" },
  { id: "sharkZones", label: "Shark Zones" },
  { id: "markers", label: "Volcanoes" },
  { id: "wind", label: "Wind" },
  { id: "currents", label: "Currents" },
  { id: "latLines", label: "Lat Lines" },
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
  const setLayerOpacity = useUIStore((s) => s.setLayerOpacity);
  const stretchToFit = useUIStore((s) => s.stretchToFit);
  const setStretchToFit = useUIStore((s) => s.setStretchToFit);

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
      </div>

      {/* Trade-good belts (each good is a separate sublayer toggle) */}
      <div style={section}>
        <div style={sectionHeader}>Trade Goods</div>
        {GOOD_DEFS.map((g) => {
          const key = goodOverlayKey(g.name);
          return (
            <label key={key} style={checkboxRow}>
              <input
                type="checkbox"
                checked={!!overlayVisibility[key]}
                onChange={() => toggleOverlay(key)}
                style={{ accentColor: "#4a90d0", width: 12, height: 12 }}
              />
              <span style={{ color: overlayVisibility[key] ? "#b0c8e0" : "#5a6a80" }}>
                {g.emoji} {g.label}
              </span>
            </label>
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
