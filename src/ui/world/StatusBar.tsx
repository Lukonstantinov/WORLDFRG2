import { useEffect } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { usePaletteStore } from "@state/paletteStore";
import { layerReadout, latLon } from "./hoverReadout";

/** The bottom bar carries the HOVER READOUT — position, terrain, and the active
 *  layer's own value with its unit, for the cell under the cursor.
 *
 *  This is the map key for every layer that cannot carry an exact legend (§8.18).
 *  Rather than invent a colour scale for those — which is how the elevation legend
 *  came to be wrong in the first place — the bar reports the number the simulation
 *  actually holds. */
export function StatusBar() {
  const statusText = useUIStore((s) => s.statusText);
  const activeLayer = useUIStore((s) => s.activeLayer);
  const hover = useUIStore((s) => s.hoverInfo);
  const meta = useWorldStore((s) => s.meta);
  const palettes = usePaletteStore((s) => s.palettes);
  const load = usePaletteStore((s) => s.load);

  useEffect(() => { void load(); }, [load]);

  const elevMax = palettes?.elev_max_m ?? 8848;
  const value = hover ? layerReadout(activeLayer, hover, elevMax) : "";

  return (
    <div style={{
      height: 24, display: "flex", alignItems: "center", gap: 16,
      padding: "0 12px", background: "#0a0f18", borderTop: "1px solid #1a2535",
      fontSize: 11, color: "#4a6080", flexShrink: 0,
    }}>
      <span style={{ color: "#6a8aa8" }}>{statusText}</span>

      {hover && (
        <span style={{
          marginLeft: "auto", display: "flex", alignItems: "center", gap: 10,
          fontFamily: "ui-monospace, monospace", fontSize: 10,
          fontVariantNumeric: "tabular-nums", whiteSpace: "nowrap",
        }}>
          <span style={{ color: "#3a5068" }}>{latLon(hover)}</span>
          <span style={{ color: "#3a5068" }}>{hover.wx},{hover.wy}</span>
          {value && (
            <span style={{ color: "#8fb0cc", fontWeight: 600 }}>{value}</span>
          )}
        </span>
      )}

      {meta && (
        <span style={{
          marginLeft: hover ? 0 : "auto", fontFamily: "ui-monospace, monospace",
          fontSize: 10, color: "#2f4258", whiteSpace: "nowrap",
        }}>
          {meta.grid_width}&#215;{meta.grid_height}
        </span>
      )}
    </div>
  );
}
