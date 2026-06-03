import { useState, useEffect } from "react";
import { useUIStore } from "../state/uiStore";
import { getCellInfo } from "../bridge/tauri";
import type { CellInfo } from "../types";
import { GOOD_DEFS } from "../goods";

const goodLabel = (name: string) => {
  const d = GOOD_DEFS.find((g) => g.name === name);
  return d ? `${d.emoji} ${d.label}` : name;
};

const KOPPEN_NAMES: Record<number, string> = {
  0: "None",
  1: "Af - Tropical rainforest",
  2: "Am - Tropical monsoon",
  3: "Aw - Tropical savanna",
  4: "BWh - Hot desert",
  5: "BWk - Cold desert",
  6: "BSh - Hot steppe",
  7: "BSk - Cold steppe",
  8: "Csa - Mediterranean hot summer",
  9: "Csb - Mediterranean warm summer",
  10: "Csc - Mediterranean cold summer",
  11: "Cfa - Humid subtropical",
  12: "Cfb - Oceanic",
  13: "Cfc - Subpolar oceanic",
  14: "Dfa - Hot-summer continental",
  15: "Dfb - Warm-summer continental",
  16: "Dfc - Subarctic",
  17: "Dfd - Extreme subarctic",
  18: "Dsa - Mediterranean continental hot",
  19: "Dsb - Mediterranean continental warm",
  20: "Dsc - Mediterranean continental cold",
  21: "ET - Tundra",
  22: "EF - Ice cap",
};

const SOIL_NAMES: Record<number, string> = {
  0: "None",
  1: "Oxisol",
  2: "Ultisol",
  3: "Mollisol",
  4: "Alfisol",
  5: "Spodosol",
  6: "Aridisol",
  7: "Histosol",
  8: "Entisol",
  9: "Andisol",
  10: "Gelisol",
  11: "Alluvial",
};

const CURRENT_TYPE_NAMES: Record<number, string> = {
  0: "Neutral",
  1: "Warm",
  2: "Cold",
};

function vectorToCompass(vx: number, vy: number): string {
  if (Math.abs(vx) < 0.01 && Math.abs(vy) < 0.01) return "Calm";
  const angle = Math.atan2(vy, vx) * (180 / Math.PI);
  // vx>0=E, vy>0=S (screen coords)
  const dirs = ["E", "SE", "S", "SW", "W", "NW", "N", "NE"];
  const idx = (Math.round(((angle + 360) % 360) / 45) % 8);
  return dirs[idx];
}

export function InfoPanel() {
  const inspectedCell = useUIStore((s) => s.inspectedCell);
  const setInspectedCell = useUIStore((s) => s.setInspectedCell);
  const [info, setInfo] = useState<CellInfo | null>(null);

  useEffect(() => {
    if (!inspectedCell) {
      setInfo(null);
      return;
    }
    getCellInfo(inspectedCell.wx, inspectedCell.wy)
      .then(setInfo)
      .catch(() => setInfo(null));
  }, [inspectedCell]);

  if (!info) return null;

  const lat = 90 - (info.wy / info.grid_height) * 180;
  const lon = (info.wx / info.grid_width) * 360 - 180;
  const latDir = lat >= 0 ? "N" : "S";
  const lonDir = lon >= 0 ? "E" : "W";

  const row = (label: string, value: string) => (
    <div style={{ display: "flex", justifyContent: "space-between", padding: "1px 0" }}>
      <span style={{ color: "#607090" }}>{label}</span>
      <span style={{ color: "#c0d0e0" }}>{value}</span>
    </div>
  );

  const bar = (label: string, value: number) => (
    <div style={{ marginBottom: 2 }}>
      <div style={{ display: "flex", justifyContent: "space-between" }}>
        <span style={{ color: "#607090" }}>{label}</span>
        <span style={{ color: "#c0d0e0" }}>{(value * 100).toFixed(0)}%</span>
      </div>
      <div style={{ height: 4, background: "#1a2a40", borderRadius: 2 }}>
        <div
          style={{
            height: "100%",
            width: `${value * 100}%`,
            background: "#4a8acf",
            borderRadius: 2,
          }}
        />
      </div>
    </div>
  );

  const section = (title: string) => (
    <div style={{ color: "#3a6a9f", fontSize: 10, fontWeight: 600, marginTop: 6, marginBottom: 2, borderTop: "1px solid #1e2e42", paddingTop: 4 }}>
      {title}
    </div>
  );

  return (
    <div
      style={{
        position: "absolute",
        top: 8,
        right: 8,
        width: 240,
        background: "rgba(10, 15, 24, 0.94)",
        border: "1px solid #1e2e42",
        borderRadius: 6,
        padding: 10,
        fontSize: 11,
        zIndex: 20,
        backdropFilter: "blur(8px)",
        maxHeight: "calc(100vh - 60px)",
        overflowY: "auto",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
        <span style={{ color: "#4a8acf", fontWeight: 700, fontSize: 12 }}>
          Cell Info ({info.wx}, {info.wy})
        </span>
        <span
          onClick={() => setInspectedCell(null)}
          style={{ color: "#607090", cursor: "pointer", fontSize: 14, lineHeight: 1 }}
          title="Close"
        >
          \u00D7
        </span>
      </div>

      {row("Latitude", `${Math.abs(lat).toFixed(1)}\u00B0 ${latDir}`)}
      {row("Longitude", `${Math.abs(lon).toFixed(1)}\u00B0 ${lonDir}`)}
      {row("Terrain", info.terrain === "land" ? "Land" : "Sea")}
      {row("Plate", `#${info.plate_index}`)}

      {info.terrain === "land" ? (
        <>
          {section("Terrain")}
          {row("Elevation", `${(info.elevation * 8848).toFixed(0)}m`)}
          {row("Biome", info.biome)}
          {info.is_volcanic && row("Volcanic", "Yes")}

          {section("Climate")}
          {row("Temperature", `${info.temperature.toFixed(1)}\u00B0C`)}
          {row("Precipitation", `${info.precipitation.toFixed(0)} mm/yr`)}
          {row("K\u00F6ppen", KOPPEN_NAMES[info.koppen] || `Code ${info.koppen}`)}
          {row("Dist. to Coast", info.distance_to_ocean < 0.01 ? "Coastal" : `${(info.distance_to_ocean * 60).toFixed(0)} cells`)}

          {section("Atmosphere")}
          {row("Wind", vectorToCompass(info.wind_vx, info.wind_vy))}

          {section("Soil & Resources")}
          {row("Soil", SOIL_NAMES[info.soil_type] || `Type ${info.soil_type}`)}
          {bar("Fertility", info.fertility)}

          {info.goods.length > 0 && section("Trade Goods")}
          {info.goods.map((g) => row(goodLabel(g.name), `${Math.round((g.amount / 255) * 100)}%`))}
        </>
      ) : (
        <>
          {section("Ocean")}
          {row("Depth", `${(info.sea_depth * 4000).toFixed(0)}m`)}
          {info.is_shelf && row("Shelf", "Yes")}

          {(info.current_vx !== 0 || info.current_vy !== 0) && (
            <>
              {row("Current", `${vectorToCompass(info.current_vx, info.current_vy)} (${CURRENT_TYPE_NAMES[info.current_type] || "?"})`)}
              {row("Speed", `${Math.sqrt(info.current_vx ** 2 + info.current_vy ** 2).toFixed(2)}`)}
            </>
          )}

          {section("Ocean Chemistry")}
          {row("Salinity", `${info.salinity.toFixed(1)} PSU`)}
          {bar("Shark Risk", info.shark_risk)}
          {bar("Shipworm Risk", info.shipworm_risk)}

          {section("Resources")}
          {bar("Fishery", info.fishery)}
          {info.goods.length > 0 && section("Marine Goods")}
          {info.goods.map((g) => row(goodLabel(g.name), `${Math.round((g.amount / 255) * 100)}%`))}
        </>
      )}
    </div>
  );
}
