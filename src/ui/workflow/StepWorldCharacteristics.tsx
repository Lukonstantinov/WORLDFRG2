import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { getPlanetConfig, setPlanetConfig, setLatitudeConfig, type PlanetConfig } from "@bridge";
import { PlanetSlider } from "@ui/world/LatitudeControl";
import { genBtn } from "@ui/workflow/WorkflowPanel";

/**
 * World Characteristics — the planet-scale knobs (rotation direction/speed,
 * sunlight, greenhouse, eccentricity, dryness, axial tilt) that decide where
 * the wind belts, deserts and seasons sit. These are read by EVERY later
 * generation step (Ocean & Atmosphere, Climate, Rivers, Soil…), so they belong
 * first — set them before generating, not after.
 *
 * A duplicate of the same planetary controls also lives in the right-side
 * Toolbar (LatitudeControl) for quick access mid-session; both read/write the
 * same backend state via `getPlanetConfig`/`setPlanetConfig`, so either stays
 * in sync with the other.
 */
export function StepWorldCharacteristics() {
  const meta = useWorldStore((s) => s.meta);
  const setMeta = useWorldStore((s) => s.setMeta);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const stepCompleted = useUIStore((s) => s.stepCompleted);

  const [planet, setPlanet] = useState<PlanetConfig | null>(null);
  useEffect(() => { getPlanetConfig().then(setPlanet).catch(() => {}); }, [meta?.name]);

  // Settings-only step — nothing to "generate", so it's always advanceable.
  // Mark it done once on mount so Continue is never blocked here.
  useEffect(() => {
    if (stepCompleted[0] !== true) markStepCompleted(0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const commitPlanet = (next: PlanetConfig) => {
    setPlanet(next); // optimistic
    setPlanetConfig(next).then(setPlanet).catch(() => {});
  };

  const obliquity = meta?.obliquity ?? 23.44;
  const [tiltStr, setTiltStr] = useState("");
  useEffect(() => { setTiltStr(obliquity.toFixed(1)); }, [obliquity]);
  const setTilt = (deg: number) => {
    const tilt = Math.max(0, Math.min(80, deg));
    const lc = useWorldStore.getState().latConfig;
    setLatitudeConfig(lc.equatorOffset, lc.latScale, lc.lineRatio, tilt).then(setMeta).catch(() => {});
  };
  const commitTilt = () => {
    const v = parseFloat(tiltStr);
    if (Number.isFinite(v)) setTilt(v);
    else setTiltStr(obliquity.toFixed(1));
  };
  const onEnter = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") (e.target as HTMLInputElement).blur();
  };

  if (!meta) {
    return <div style={{ color: "#607090", fontSize: 11 }}>Create a world first.</div>;
  }
  if (!planet) {
    return <div style={{ color: "#607090", fontSize: 11 }}>Loading…</div>;
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <div style={{ color: "#506080", fontSize: 10.5, lineHeight: 1.4, marginBottom: 2 }}>
        These decide where climate belts, deserts and seasons land — set them
        before generating Ocean &amp; Atmosphere. Changing one later just means
        re-running from Step 3 onward.
      </div>

      <PlanetSlider
        label="Rotation" unit="× Earth" value={Math.abs(planet.rotationRate)}
        min={0.25} max={4} step={0.05} digits={2}
        onChange={(v) => commitPlanet({ ...planet, rotationRate: planet.rotationRate < 0 ? -v : v })}
        presets={[["Slow ½×", 0.5], ["Earth 1×", 1], ["Fast 2×", 2]]}
        hint="Sets the wind belts / Hadley cell: slower = wider tropics, deserts & storm tracks pushed poleward, fewer bands; faster = tighter banding."
      />
      <label style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 10, color: "#5a7090", cursor: "pointer" }}>
        <input type="checkbox" checked={planet.rotationRate < 0}
          onChange={(e) => commitPlanet({ ...planet, rotationRate: Math.abs(planet.rotationRate) * (e.target.checked ? -1 : 1) })}
          style={{ accentColor: "#3a80c0" }} />
        Retrograde (spins backwards — mirrors trade-wind direction and which
        coasts get the warm/cold boundary currents; belt latitudes are unchanged)
      </label>

      <div style={{ marginTop: 4 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 2 }}>
          <span style={{ fontSize: 10, color: "#5a7090" }}>Axial tilt</span>
          <span style={{ fontSize: 10, color: "#8aa0c0", fontFamily: "monospace" }}>{obliquity.toFixed(1)}°</span>
        </div>
        <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <input
            type="range" min={0} max={80} step={0.5} value={obliquity}
            onChange={(e) => setTiltStr(String(e.target.value))}
            onPointerUp={(e) => setTilt(Number((e.target as HTMLInputElement).value))}
            onKeyUp={(e) => setTilt(Number((e.target as HTMLInputElement).value))}
            style={{ flex: 1, minWidth: 0, height: 4, cursor: "pointer" }}
          />
          <input
            value={tiltStr} onChange={(e) => setTiltStr(e.target.value)}
            onBlur={commitTilt} onKeyDown={onEnter} inputMode="decimal"
            style={{ width: 50, flexShrink: 0, padding: "2px 4px", fontSize: 11, fontFamily: "monospace",
              background: "#0d1219", color: "#c0d8f0", border: "1px solid #1e2a38", borderRadius: 4, textAlign: "right" }}
          />
        </div>
        <div style={{ fontSize: 9, color: "#405060", marginTop: 3, lineHeight: 1.35 }}>
          Drives the seasons: 0° = none anywhere, 23.4° = Earth-like, higher = harsher extremes.
        </div>
      </div>

      <PlanetSlider
        label="Greenhouse" unit="× Earth" value={planet.greenhouse}
        min={0} max={3} step={0.05} digits={2}
        onChange={(v) => commitPlanet({ ...planet, greenhouse: v })}
        presets={[["Icehouse ½×", 0.5], ["Earth 1×", 1], ["Hothouse 2×", 2]]}
        hint="Global warming from atmospheric trapping: higher warms the whole planet and flattens the equator-pole gradient; low enough tips toward a snowball."
      />
      <PlanetSlider
        label="Sunlight" unit="× Earth" value={planet.solarLum}
        min={0.5} max={1.6} step={0.01} digits={2}
        onChange={(v) => commitPlanet({ ...planet, solarLum: v })}
        presets={[["Dim 0.9×", 0.9], ["Earth 1×", 1], ["Bright 1.1×", 1.1]]}
        hint="Stellar irradiance (a fainter/brighter star or a wider/closer orbit): scales total insolation and the global-mean temperature."
      />
      <PlanetSlider
        label="Eccentricity" unit="" value={planet.eccentricity}
        min={0} max={0.4} step={0.005} digits={3}
        onChange={(v) => commitPlanet({ ...planet, eccentricity: v })}
        presets={[["Circular 0", 0], ["Earth .017", 0.0167], ["High .2", 0.2]]}
        hint="Orbit shape: higher makes one hemisphere's seasons shorter & sharper than the other's."
      />
      <PlanetSlider
        label="Dryness" unit="× Earth" value={planet.dryness}
        min={0.3} max={3} step={0.05} digits={2}
        onChange={(v) => commitPlanet({ ...planet, dryness: v })}
        presets={[["Wet ½×", 0.5], ["Earth 1×", 1], ["Arid 2×", 2]]}
        hint="Global precipitation multiplier: higher shrinks rainfall everywhere (deserts expand), lower expands rainforest/monsoon belts. A coarse knob, not a mechanism."
      />

      <div style={{ fontSize: 9, color: "#405060", marginTop: 2, lineHeight: 1.35 }}>
        All default to Earth (no-op). Fine latitude framing (equator position,
        band expansion, line spacing) is a separate, purely visual control in
        the right-side Toolbar under Climate.
      </div>

      <button disabled style={{ ...genBtn, opacity: 0.5, cursor: "default", textAlign: "center" }}>
        Settings saved automatically
      </button>
    </div>
  );
}
