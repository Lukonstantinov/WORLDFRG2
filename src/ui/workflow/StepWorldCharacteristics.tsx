import { useEffect } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { LatitudeControl } from "@ui/world/LatitudeControl";

/**
 * World Characteristics — the latitude framing (equator position, band expansion,
 * line proportion) plus the planet-scale knobs (rotation direction/speed, axial
 * tilt, sunlight, greenhouse, eccentricity, dryness) that decide where the wind
 * belts, deserts and seasons sit.
 *
 * It sits RIGHT AFTER Landmass in the wizard: you draw your continents first, then
 * frame their latitudes and set the planet before generating Ocean & Atmosphere
 * (which is the first step that reads these). This is the SINGLE home for these
 * controls — the former duplicate in the right-side Toolbar has been removed. The
 * whole UI is the shared `LatitudeControl`, so nothing here can drift out of sync.
 */
export function StepWorldCharacteristics() {
  const meta = useWorldStore((s) => s.meta);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const stepCompleted = useUIStore((s) => s.stepCompleted);

  // Settings-only step — nothing to "generate", so it's always advanceable.
  // Mark it done once on mount so Continue is never blocked here.
  useEffect(() => {
    if (stepCompleted[0] !== true) markStepCompleted(0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!meta) {
    return <div style={{ color: "#607090", fontSize: 11 }}>Create a world first.</div>;
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <div style={{ color: "#506080", fontSize: 10.5, lineHeight: 1.4, marginBottom: 2 }}>
        Frame the latitudes and set the planet — these decide where climate belts,
        deserts and seasons land. Set them after drawing your landmass and before
        generating Ocean &amp; Atmosphere; changing one later just means re-running
        from Step 3 onward. Settings save automatically.
      </div>
      <LatitudeControl />
    </div>
  );
}
