import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { usePaletteStore } from "@state/paletteStore";
import { useGoodsStore } from "@state/goodsStore";
import { useWorldStore } from "@state/worldStore";
import type { ActiveTool, ActiveLayer } from "@types";
import { GOOD_DEFS, goodOverlayKey, goodCategory, CATEGORY_ORDER } from "@goods";
import { GoodIcon } from "@ui/goods/GoodIcon";
import { layerReady, LAYER_REQUIRES, FORGE_ONLY } from "./mapThemes";
import { LayerSets } from "./LayerSets";
import { useSettingsStore, MAP_STYLES } from "@state/settingsStore";
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
  return "#13100b";
}

const tools: { id: ActiveTool; label: string; icon: string; tip: string }[] = [
  { id: "pan", label: "Pan", icon: "\u270B", tip: "Click-drag to pan the map" },
  { id: "select", label: "Select", icon: "\u25CE", tip: "Click a cell to inspect it" },
  { id: "paint", label: "Paint", icon: "\u270F", tip: "Paint land, Shift = erase to sea" },
  { id: "elevation", label: "Elev", icon: "\u25B2", tip: "Paint elevation, Shift = flatten" },
  { id: "shelf", label: "Shelf", icon: "\u2248", tip: "Paint shelf, Shift = erase" },
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
// GENERATION_UX_REDESIGN_PLAN.md Slice 8 (F6) — exported so the Export
// dialog's raw-layer list can be DERIVED from this, the canonical layer list
// the app actually renders from, rather than a second hand-copied table that
// drifts (F6 measured EXPORTABLE_LAYERS at 15 entries against this list's 26).
export const layerGroups: { group: string; layers: { id: ActiveLayer; label: string }[] }[] = [
  {
    group: "Terrain",
    layers: [
      { id: "land", label: "Land / Sea" },
      { id: "elevation", label: "Elevation & Shelf" },
      { id: "plates", label: "Plates" },
    ],
  },
  {
    group: "Ocean",
    layers: [
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
      { id: "windspeed", label: "Wind Speed" },
    ],
  },
  {
    group: "Climate & Biomes",
    layers: [
      { id: "climate", label: "Climate (Köppen)" },
      { id: "biomes", label: "Biomes" },
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
    group: "Political",
    layers: [
      { id: "political", label: "Realms & Provinces" },
    ],
  },
];

// Climate-circulation overlays (the winds, the ITCZ rain line, the belt latitudes
// and the reference lat lines) — grouped together since they all describe the
// atmosphere/circulation and respond to the Planet panel.
const climateOverlays = [
  { id: "itcz", label: "\u{1F327} ITCZ (rain line)" },
  { id: "wind", label: "Wind & circulation belts" },
  { id: "currents", label: "Currents" },
  { id: "latLines", label: "Lat Lines" },
];

// GENERATION_UX_REDESIGN_PLAN.md Slice 10 (F11) — only the overlay keys
// `OverlayManager.render` routes through `withOpacity` actually respond to a
// per-overlay opacity slider. Showing the slider for every overlay id would
// recreate F11's own bug (a control wired to nothing) for whichever key isn't
// covered, so the affordance is limited to the real set.
const OPACITY_CAPABLE = new Set([
  "provinces", "tradeRegions", "tradeFlows", "dynamicFlow", "campaignCorridors",
  "expeditions", "travelRoute", "goodScarcity", "riverBreaks", "merchantRoutes",
  "goodFlow", "futures", "houseControl", "coinDominance", "plagueZones",
  "guildCities", "dynastyLinks", "tradeCorridors", "tradeBasins", "tradeHeat",
  "colonies", "settlementNames", "hubNames", "toponyms",
  // Extended to every remaining overlay `OverlayManager.render` routes
  // through `withOpacity`, once its draw code stopped being inlined
  // directly in `render()` (rivers/lakes/settlements and friends). The two
  // NOT covered — windBelts, itcz — share drawing helpers (`belt`/`ribbon`/
  // `trace`) defined in their common parent block that close over the
  // OUTER `ctx`, so wrapping just their own sub-block wouldn't actually
  // redirect their drawing; left as a named, deliberate gap rather than
  // wrapped incorrectly.
  "lakes", "rivers", "currents", "wind", "plateMotion", "latLines",
  "fisheryBanks", "cultures", "states",
  "stormZones", "monsoonZones", "reefZones", "tradeRoutes",
  "politicalInfluence", "speculation", "figureMarks", "landmarks",
  "chokepoints", "migrations", "settlements", "deposits",
]);

// Settlements carry two sub-toggles (names, colonies) instead of three rows.
const overlayTypes = [
  { id: "rivers", label: "Rivers" },
  { id: "lakes", label: "Lakes" },
  { id: "settlements", label: "Settlements" },
  { id: "hubNames", label: "Hub Names" },
  { id: "cultures", label: "\u{1F465} Peoples" },
  { id: "tradeRoutes", label: "Trade Routes" },
  { id: "tradeFlows", label: "Trade Flows" },
  { id: "deposits", label: "\u26CF Deposits" },
];


// Biological / political sublayer overlays (grouped separately).
const bioOverlays = [
  // Sea hazards in one switch: reef zones + hurricane belts (hazardSea drives both).
  { id: "hazardSea", label: "\u{1FAA8} Reefs & hurricanes" },
  { id: "monsoonZones", label: "\u{1F327} Monsoon Areas" },
  { id: "plagueZones", label: "\u{2620} Plague Zones" },
];

// Everything that is not biological.
const otherOverlays = [
  { id: "states", label: "\u{1F451} Realms" },
  { id: "dynamicFlow", label: "\u{1F30A} Dynamic Trade Flow" },
  { id: "chokepoints", label: "\u{2693} Chokepoints" },
  { id: "speculation", label: "\u{1FAE7} Speculation Risk" },
  { id: "figureMarks", label: "\u{269C} Notable Figures" },
];
/** Overlays that are really two keys behind one switch. */
const LINKED: Record<string, string[]> = {
  hazardSea: ["reefZones", "stormZones"],
  wind: ["wind", "windBelts"],
};

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
  const merchantRouteRisk = useUIStore((s) => s.merchantRouteRisk);
  const toggleMerchantRouteRisk = useUIStore((s) => s.toggleMerchantRouteRisk);
  const routeColorMode = useUIStore((s) => s.routeColorMode);
  const setRouteColorMode = useUIStore((s) => s.setRouteColorMode);
  const migrationMode = useUIStore((s) => s.migrationMode);
  const setMigrationMode = useUIStore((s) => s.setMigrationMode);
  const showBankIcons = useUIStore((s) => s.showBankIcons);
  const setShowBankIcons = useUIStore((s) => s.setShowBankIcons);
  const setGoodDetail = useUIStore((s) => s.setGoodDetail);
  const activeMapTheme = useUIStore((s) => s.activeMapTheme);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const mapStyle = useSettingsStore((s) => s.mapStyle);
  const setMapStyle = useSettingsStore((s) => s.setMapStyle);
  const elevationStyle = useUIStore((s) => s.elevationStyle);
  const setElevationStyle = useUIStore((s) => s.setElevationStyle);
  const palettes = usePaletteStore((s) => s.palettes);
  const loadPalettes = usePaletteStore((s) => s.load);
  const compareLayer = useUIStore((s) => s.compareLayer);
  const setCompareLayer = useUIStore((s) => s.setCompareLayer);
  const [expandedCats, setExpandedCats] = useState<Record<string, boolean>>({});
  // Collapsible top-level sections (the big lists start collapsed to declutter).
  const [openSection, setOpenSection] = useState<Record<string, boolean>>({
    Plates: true, Layers: true, Overlays: true, Climate: true, Biological: false, Other: false, "Trade Goods": false, Provinces: false, View: true,
  });
  const toggleSection = (k: string) => setOpenSection((s) => ({ ...s, [k]: !s[k] }));
  const campaign = appMode === "chronicle";
  // A switch can drive several overlay keys (LINKED): it reads as on if the first is.
  const isOn = (id: string) => !!overlayVisibility[(LINKED[id] ?? [id])[0]];
  const flip = (id: string) => {
    const keys = LINKED[id];
    if (keys) setOverlaysVisible(keys, !isOn(id)); else toggleOverlay(id);
  };
  useEffect(() => { loadPalettes(); }, [loadPalettes]);
  const bioParams = useUIStore((s) => s.bioParams);
  const setBioParams = useUIStore((s) => s.setBioParams);
  const hubDisplay = useUIStore((s) => s.hubDisplay);
  const setHubDisplay = useUIStore((s) => s.setHubDisplay);
  const goodsSpecs = useGoodsStore((s) => s.specs);
  const goodItems = goodsSpecs.length > 0
    // Manufactured goods are made in cities, not grown in a belt — they have no map
    // overlay (the backend emits no region for them), so hide their toggle here.
    ? goodsSpecs.filter((g) => g.enabled && g.distribution !== "manufactured")
        .map((g) => ({ id: g.id, name: g.name }))
    : GOOD_DEFS.map((g) => ({ id: g.name, name: g.label }));
  const setLayerOpacity = useUIStore((s) => s.setLayerOpacity);
  const overlayOpacity = useUIStore((s) => s.overlayOpacity);
  const setOverlayOpacity = useUIStore((s) => s.setOverlayOpacity);
  const stretchToFit = useUIStore((s) => s.stretchToFit);
  const setStretchToFit = useUIStore((s) => s.setStretchToFit);
  const setShowGoodsBrowser = useUIStore((s) => s.setShowGoodsBrowser);
  const provinces = useWorldStore((s) => s.provinces);
  const showStates = useUIStore((s) => s.showStates);
  const setShowStates = useUIStore((s) => s.setShowStates);
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
      background: "#18140e", borderLeft: "1px solid #352e21",
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
                background: activeTool === t.id ? "#705c3a" : "#262017",
                color: activeTool === t.id ? "#e0eeff" : "#9c8d74",
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

      {/* Map style — how every layer is drawn (Classic / Atlas / Parchment / Game
          Board). The old map-plate compositions are hidden: a plate bundled a base
          layer with overlays that no longer exist. */}
      <div style={section}>
        <SectionHead title="Map style" open={openSection.Plates} onToggle={() => toggleSection("Plates")} />
        {openSection.Plates && (<>
          <div role="radiogroup" aria-label="Map style"
            style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: 3, marginBottom: 6 }}>
            {MAP_STYLES.map((m) => {
              const on = mapStyle === m.id;
              return (
                <button key={m.id} role="radio" aria-checked={on} title={m.blurb}
                  onClick={() => setMapStyle(m.id)}
                  style={{
                    padding: "4px 2px", borderRadius: 4, fontSize: 10, cursor: "pointer",
                    border: `1px solid ${on ? "#c9a24a" : "#453925"}`,
                    background: on ? "#3a2f1a" : "#221c12",
                    color: on ? "#f0dcaa" : "#ac9c81", fontWeight: on ? 600 : 400,
                  }}>{m.name}</button>
              );
            })}
          </div>
          <div style={{ fontSize: 9, color: "#897a61", lineHeight: 1.45 }}>
            {MAP_STYLES.find((m) => m.id === mapStyle)?.blurb}
          </div>
          <LayerSets />
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
                    background: activeLayer === l.id ? "#4e4028" : "transparent",
                    color: activeLayer === l.id ? "#c0ddf0" : ready ? "#9c8a6c" : "#5a5141",
                    fontWeight: activeLayer === l.id ? 600 : 400,
                    borderLeft: activeLayer === l.id ? "2px solid #c09b5a" : "2px solid transparent",
                  }}
                >
                  <span style={{ flex: 1, minWidth: 0 }}>{l.label}</span>
                  {!ready && (
                    <span style={{ fontSize: 8.5, color: "#554a38", fontVariantNumeric: "tabular-nums" }}>
                      step {LAYER_REQUIRES[l.id]}
                    </span>
                  )}
                </div>
              );
            })}
          </div>
        ))}
      </div>

      {/* ELEVATION STYLES: a named alternative palette+relief treatment for the
          "elevation"/"terrain" layers only — Bartholomew/Times classed layer
          tinting, Imhof Alpine, Arid, Polar, a monochrome Analytical hillshade,
          a sepia Antique plate, and an Abyssal bathymetry showcase. Rides in the
          layer key (`#style=`) exactly like class isolation, so it only appears
          while one of those two layers is active. */}
      {(activeLayer === "elevation" || activeLayer === "terrain") && (
        <div style={section}>
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span style={sliderLabel}>Style</span>
            <select
              value={elevationStyle ?? ""}
              onChange={(e) => setElevationStyle(e.target.value || null)}
              style={{
                flex: 1, minWidth: 0, background: "#221c12", color: "#d1c3ab",
                border: "1px solid #453925", borderRadius: 4, fontSize: 10, padding: "2px 4px",
              }}
            >
              <option value="">Default</option>
              {(palettes?.elevation_styles ?? []).map((s) => (
                <option key={s.key} value={s.key}>{s.label}</option>
              ))}
            </select>
          </div>
        </div>
      )}

      {/* Plate motion arrows (§8.24 B2) — the Euler-pole velocity field that
          already drives boundary classification, made visible for the first
          time. Only appears while the Plates layer is active, since the arrows
          are meaningless without the boundary tinting under them. */}
      {activeLayer === "plates" && (
        <div style={section}>
          <label style={checkboxRow}>
            <input
              type="checkbox"
              checked={!!overlayVisibility.plateMotion}
              onChange={() => toggleOverlay("plateMotion")}
              style={{ accentColor: "#c09b5a", width: 12, height: 12 }}
            />
            <span style={{ color: overlayVisibility.plateMotion ? "#dccdb4" : "#7b715f" }}>
              {"➤"} Plate Motion
            </span>
          </label>
        </div>
      )}

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
              flex: 1, minWidth: 0, background: "#221c12", color: "#d1c3ab",
              border: "1px solid #453925", borderRadius: 4, fontSize: 10, padding: "2px 4px",
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
          <div style={{ fontSize: 9, color: "#897a61", marginTop: 4, lineHeight: 1.4 }}>
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
        {overlayTypes.filter((o) => !(campaign && FORGE_ONLY.has(o.id))).map((o) => (
          <div key={o.id}>
            <label style={checkboxRow}>
              <input
                type="checkbox"
                checked={isOn(o.id)}
                onChange={() => flip(o.id)}
                style={{ accentColor: "#c09b5a", width: 12, height: 12 }}
              />
              <span style={{ color: isOn(o.id) ? "#dccdb4" : "#7b715f" }}>
                {o.label}
              </span>
            </label>
            {isOn(o.id) && OPACITY_CAPABLE.has(o.id) && (
              <div style={{ display: "flex", alignItems: "center", gap: 4, margin: "0 0 3px 22px" }}>
                <input
                  type="range" min={0} max={100}
                  value={Math.round((overlayOpacity[o.id] ?? 1) * 100)}
                  onChange={(e) => setOverlayOpacity(o.id, Number(e.target.value) / 100)}
                  style={{ ...rangeStyle, height: 3 }}
                />
                <span style={{ fontSize: 8.5, color: "#897a61", minWidth: 24, textAlign: "right" }}>
                  {Math.round((overlayOpacity[o.id] ?? 1) * 100)}%
                </span>
              </div>
            )}
            {/* Settlements: names and colonies are sub-toggles of the one overlay. */}
            {o.id === "settlements" && isOn("settlements") && (
              <div style={{ margin: "0 0 3px 18px" }}>
                {([["settlementNames", "Names"], ["colonies", "Colonies"]] as const).map(([k, label]) => (
                  <label key={k} style={{ ...checkboxRow, padding: "1px 0", fontSize: 10.5 }}>
                    <input type="checkbox" checked={!!overlayVisibility[k]} onChange={() => toggleOverlay(k)}
                      style={{ accentColor: "#c09b5a", width: 11, height: 11 }} />
                    <span style={{ color: overlayVisibility[k] ? "#dccdb4" : "#7b715f" }}>{label}</span>
                  </label>
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
            style={{ accentColor: "#c09b5a", width: 12, height: 12 }}
          />
          <span style={{ color: showBankIcons ? "#dccdb4" : "#7b715f" }}>{"\u{1F3E6}"} Banks</span>
        </label>
        </>)}
      </div>

      {/* Climate circulation overlays (ITCZ line + winds with their belts). */}
      <div style={section}>
        <SectionHead title="Climate" open={openSection.Climate} onToggle={() => toggleSection("Climate")} />
        {openSection.Climate && (
          <div>
            <div style={{ fontSize: 9, color: "#897a61", marginBottom: 4, lineHeight: 1.4 }}>
              Circulation from the planet settings (Workflow step 0). This column is display only.
            </div>
            {climateOverlays.map((o) => (
              <label key={o.id} style={checkboxRow}>
                <input
                  type="checkbox"
                  checked={isOn(o.id)}
                  onChange={() => flip(o.id)}
                  style={{ accentColor: "#c09b5a", width: 12, height: 12 }}
                />
                <span style={{ color: isOn(o.id) ? "#dccdb4" : "#7b715f" }}>
                  {o.label}
                </span>
              </label>
            ))}
          </div>
        )}
      </div>

      {/* Biological hazards */}
      <div style={section}>
        <SectionHead title="Biological" open={openSection.Biological} onToggle={() => toggleSection("Biological")} />
        {openSection.Biological && (<>
        {bioOverlays.map((o) => (
          <div key={o.id}>
            <label style={checkboxRow}>
              <input
                type="checkbox"
                checked={isOn(o.id)}
                onChange={() => flip(o.id)}
                style={{ accentColor: "#c09b5a", width: 12, height: 12 }}
              />
              <span style={{ color: isOn(o.id) ? "#dccdb4" : "#7b715f" }}>
                {o.label}
              </span>
            </label>
            {isOn(o.id) && OPACITY_CAPABLE.has(o.id) && (
              <div style={{ display: "flex", alignItems: "center", gap: 4, margin: "0 0 3px 22px" }}>
                <input
                  type="range" min={0} max={100}
                  value={Math.round((overlayOpacity[o.id] ?? 1) * 100)}
                  onChange={(e) => setOverlayOpacity(o.id, Number(e.target.value) / 100)}
                  style={{ ...rangeStyle, height: 3 }}
                />
                <span style={{ fontSize: 8.5, color: "#897a61", minWidth: 24, textAlign: "right" }}>
                  {Math.round((overlayOpacity[o.id] ?? 1) * 100)}%
                </span>
              </div>
            )}
          </div>
        ))}
        {/* Seasonal storm month (worldgen only): 0 = combined annual extent, 1..months
            scrubs the cyclone season. */}
        {!campaign && (
          <div style={{ marginTop: 4, opacity: isOn("hazardSea") ? 1 : 0.5 }}>
            <div style={{ fontSize: 10, color: "#b5a68d", display: "flex", justifyContent: "space-between" }}>
              <span>{"\u{1F300}"} Storm month</span>
              <span style={{ color: "#dccdb4" }}>
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
        )}
        </>)}
      </div>

      {/* Everything that is not biological. */}
      <div style={section}>
        <SectionHead title="Other" open={openSection.Other} onToggle={() => toggleSection("Other")} />
        {openSection.Other && otherOverlays.map((o) => (
          <div key={o.id}>
            <label style={checkboxRow}>
              <input
                type="checkbox"
                checked={isOn(o.id)}
                onChange={() => flip(o.id)}
                style={{ accentColor: "#c09b5a", width: 12, height: 12 }}
              />
              <span style={{ color: isOn(o.id) ? "#dccdb4" : "#7b715f" }}>
                {o.label}
              </span>
            </label>
            {isOn(o.id) && OPACITY_CAPABLE.has(o.id) && (
              <div style={{ display: "flex", alignItems: "center", gap: 4, margin: "0 0 3px 22px" }}>
                <input
                  type="range" min={0} max={100}
                  value={Math.round((overlayOpacity[o.id] ?? 1) * 100)}
                  onChange={(e) => setOverlayOpacity(o.id, Number(e.target.value) / 100)}
                  style={{ ...rangeStyle, height: 3 }}
                />
                <span style={{ fontSize: 8.5, color: "#897a61", minWidth: 24, textAlign: "right" }}>
                  {Math.round((overlayOpacity[o.id] ?? 1) * 100)}%
                </span>
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Trade-good belts (each good is a separate sublayer toggle). Driven by the
          world's editable spec list, falling back to the static defaults. */}
      <div style={section}>
        <SectionHead title="Trade Goods" open={openSection["Trade Goods"]} onToggle={() => toggleSection("Trade Goods")} />
        {openSection["Trade Goods"] && (<>
        <button
          onClick={() => setShowGoodsBrowser(true)}
          style={{ width: "100%", marginBottom: 6, padding: "4px 6px", fontSize: 10,
            background: "#352b1b", color: "#cfe0f4", border: "1px solid #524630",
            borderRadius: 4, cursor: "pointer" }}>
          📖 Browse goods by origin
        </button>
        <button
          onClick={() => { void useGoodsStore.getState().loadFromWorld(); useGoodsStore.getState().setOpen(true); }}
          title="Add your own goods, edit recipes and values, switch goods on or off"
          style={{ width: "100%", marginBottom: 6, padding: "4px 6px", fontSize: 10,
            background: "#4e4028", color: "#f0dcaa", border: "1px solid #7a6540",
            borderRadius: 4, cursor: "pointer" }}>
          ✎ Goods Builder (add custom goods)
        </button>
        {/* CLAUDE.md §8.19 (goods localities, shipped) D9 · the TWO layers a belt carries, switched
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
                    color: active ? "#1c170e" : "#a89980",
                    background: active ? "#c3ac84" : "transparent",
                    border: `1px solid ${active ? "#c3ac84" : "#524630"}`,
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
              fontSize: 9.5, color: "#baad96", marginBottom: 6, padding: "1px 2px",
              userSelect: "none",
            }}>
            <span>Quality paint:</span>
            <span style={{ color: !goodQualityHeatmap ? "#cfe0f4" : "#8a7c65", fontWeight: !goodQualityHeatmap ? 600 : 400 }}>
              hue
            </span>
            <span style={{
              position: "relative", width: 26, height: 13, borderRadius: 7,
              background: goodQualityHeatmap ? "#7c6844" : "#443a29",
              border: "1px solid #524630", flex: "0 0 auto",
            }}>
              <span style={{
                position: "absolute", top: 1, left: goodQualityHeatmap ? 13 : 1,
                width: 11, height: 11, borderRadius: "50%",
                background: goodQualityHeatmap
                  ? "linear-gradient(135deg, #c42020, #f0d640)"
                  : "linear-gradient(135deg, #c3ac84, #eceae0)",
                transition: "left 0.12s ease",
              }} />
            </span>
            <span style={{ color: goodQualityHeatmap ? "#cfe0f4" : "#8a7c65", fontWeight: goodQualityHeatmap ? 600 : 400 }}>
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
                  style={{ cursor: "pointer", color: "#8a7c65", fontSize: 9, width: 9, userSelect: "none" }}
                  title={expanded ? "Collapse" : "Expand to toggle individual goods"}
                >{expanded ? "▼" : "▶"}</span>
                <input
                  type="checkbox"
                  checked={allOn}
                  ref={(el) => { if (el) el.indeterminate = someOn; }}
                  onChange={() => setOverlaysVisible(keys, !allOn)}
                  style={{ accentColor: "#c09b5a", width: 12, height: 12 }}
                />
                <span
                  onClick={() => setExpandedCats((e) => ({ ...e, [cat]: !e[cat] }))}
                  style={{ cursor: "pointer", color: shownCount > 0 ? "#ccbda3" : "#8a7c65",
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
                      style={{ accentColor: "#c09b5a", width: 12, height: 12 }}
                    />
                    <span
                      onClick={() => setGoodDetail(g.id)}
                      title="Show seeding climates & heatmap"
                      style={{ color: overlayVisibility[key] ? "#dccdb4" : "#7b715f", cursor: "pointer", flex: 1 }}
                    >
                      <GoodIcon name={g.id} size={16} style={{ display: "inline-block", verticalAlign: "middle", marginRight: 3 }} />{g.name}
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
                style={{ accentColor: "#c09b5a", width: 12, height: 12 }}
                disabled={provinces.length === 0}
              />
              <span style={{ color: overlayVisibility.provinces ? "#dccdb4" : "#7b715f" }}>
                🗺 Culture fill + borders
              </span>
            </label>
            {provinces.length === 0 && (
              <div style={{ fontSize: 9, color: "#897a61", lineHeight: 1.4 }}>
                No provinces yet. Generate them in the Settlements step.
              </div>
            )}
            {provinces.length > 0 && (
              <div style={{ fontSize: 9, color: "#897a61" }}>
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
              onClick={() => setShowStates(!showStates)}
              style={{
                padding: "3px 6px", fontSize: 10, borderRadius: 4, cursor: "pointer",
                border: `1px solid ${showStates ? "#a58855" : "#3e3422"}`,
                background: showStates ? "#413622" : "#221c12",
                color: showStates ? "#cfe2f6" : "#a28f6e",
              }}>
              {showStates ? "Close" : "Open"} Realms &amp; Provinces
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
            style={{ accentColor: "#c09b5a", width: 12, height: 12 }}
          />
          <span style={{ color: stretchToFit ? "#dccdb4" : "#7b715f" }}>
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
  fontSize: 10, color: "#827052", textTransform: "uppercase", letterSpacing: 1.2,
  marginBottom: 5, fontWeight: 600,
};

const groupLabel: React.CSSProperties = {
  fontSize: 9, color: "#5f533f", marginTop: 2, marginBottom: 1, paddingLeft: 8,
  textTransform: "uppercase", letterSpacing: 0.8,
};

const divider: React.CSSProperties = {
  height: 1, background: "#322a1d", margin: "2px 8px",
};

const sliderRow: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 2,
};

const sliderLabel: React.CSSProperties = {
  fontSize: 10, color: "#897a61",
};

const sliderValue: React.CSSProperties = {
  fontSize: 10, color: "#bcab8e", fontFamily: "monospace",
};

const rangeStyle: React.CSSProperties = {
  width: "100%", height: 4, cursor: "pointer",
};

const checkboxRow: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 6, cursor: "pointer",
  padding: "2px 0", fontSize: 11,
};
