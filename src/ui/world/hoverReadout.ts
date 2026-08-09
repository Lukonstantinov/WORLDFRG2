import type { ActiveLayer, CellInfo } from "@types";
import { koppenName } from "./climate";
import { SOIL_NAMES } from "./InfoPanel";

/** THE HOVER READOUT — what the active layer actually says at the cursor.
 *
 *  Only six layers can carry an exact legend (§8.18); the rest have their ramps
 *  written inline in Rust, and inventing a key for them is precisely how the old
 *  elevation legend came to be wrong. So instead of guessing a colour scale, the
 *  StatusBar reports the REAL value under the cursor, with its unit.
 *
 *  Every field here comes from `get_cell_info`, so a number shown is a number the
 *  simulation holds — never one re-derived from a pixel colour. */

const pct = (v: number) => `${Math.round(v * 100)}%`;

/** A risk field reads as a word first and a number second — "high (72%)" is what a
 *  player actually wants, and the bare fraction alone means nothing on first sight. */
function risk(v: number): string {
  const word = v < 0.05 ? "none" : v < 0.2 ? "low" : v < 0.45 ? "moderate" : v < 0.7 ? "high" : "severe";
  return v < 0.05 ? word : `${word} (${pct(v)})`;
}

function speed(vx: number, vy: number): number {
  return Math.sqrt(vx * vx + vy * vy);
}

/** The active layer's own value at this cell, or "" when the layer has nothing
 *  cell-local to report (in which case the bar just shows position and terrain). */
export function layerReadout(layer: ActiveLayer, c: CellInfo, elevMaxM: number): string {
  const isSea = c.terrain !== "land";

  switch (layer) {
    case "elevation":
    case "terrain":
    case "ridges":
      return isSea
        ? `sea floor · ${pct(c.sea_depth)} of max depth`
        : `${Math.round(c.elevation * elevMaxM).toLocaleString()} m`;
    case "land":
      return isSea ? "sea" : "land";
    case "temperature":
      return `${c.temperature.toFixed(1)} °C`;
    case "sst":
      // CellInfo carries air temperature, not SST — say so rather than mislabel it.
      return isSea ? `air ${c.temperature.toFixed(1)} °C` : "land";
    case "precipitation":
      return `${Math.round(c.precipitation).toLocaleString()} mm/yr`;
    case "climate":
      return koppenName(c.koppen);
    case "biomes":
      return c.biome || "unclassified";
    case "soil":
      return isSea ? "sea" : SOIL_NAMES[c.soil_type] || `type ${c.soil_type}`;
    case "fertility":
      return isSea ? "sea" : `fertility ${pct(c.fertility)}`;
    case "fisheries":
      return `fishery ${pct(c.fishery)}`;
    case "salinity":
      return isSea ? `${c.salinity.toFixed(1)} PSU` : "land";
    case "shelf":
      return isSea ? (c.is_shelf ? "continental shelf" : "deep ocean") : "land";
    case "plates":
      return `plate ${c.plate_index}${c.is_volcanic ? " · volcanic" : ""}`;
    case "currents":
      return isSea ? `current ${speed(c.current_vx, c.current_vy).toFixed(2)}` : "land";
    case "wind":
    case "windspeed":
      return `wind ${speed(c.wind_vx, c.wind_vy).toFixed(2)}`;
    case "shark":
      return `shark risk ${risk(c.shark_risk)}`;
    case "shipworm":
      return `shipworm risk ${risk(c.shipworm_risk)}`;
    case "storm":
      return `storm risk ${risk(c.storm_risk)}`;
    case "reef":
      return `reef risk ${risk(c.reef_risk)}`;
    case "disease":
      return `disease risk ${risk(c.disease_risk)}`;
    default:
      // habitability / snow and anything added later: no cell-local field yet.
      return "";
  }
}

/** Latitude/longitude from the cell's position in an equirectangular grid. The
 *  world's own latitude frame can rescale this, so it is reported as the plain
 *  geometric value the projection implies — the InfoPanel remains the precise read. */
export function latLon(c: CellInfo): string {
  const lat = 90 - (c.wy / Math.max(1, c.grid_height)) * 180;
  const lon = (c.wx / Math.max(1, c.grid_width)) * 360 - 180;
  const ns = lat >= 0 ? "N" : "S";
  const ew = lon >= 0 ? "E" : "W";
  return `${Math.abs(lat).toFixed(1)}°${ns} ${Math.abs(lon).toFixed(1)}°${ew}`;
}
