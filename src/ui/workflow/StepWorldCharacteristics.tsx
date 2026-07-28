import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { getPlanetConfig, setPlanetConfig, setLatitudeConfig, type PlanetConfig } from "@bridge";
import { PlanetSlider, LatitudeFrame, row, lbl, val, ctrlRow, range, numInput, hint, chip }
  from "@ui/workflow/PlanetControls";
import { genBtn } from "@ui/workflow/WorkflowPanel";

/**
 * World Characteristics — **the single home for every setting that shapes the
 * world before generation runs.**
 *
 * These used to be split across both columns: the planetary knobs were
 * duplicated here AND in the right-side Toolbar, the latitude framing lived only
 * on the right, and the calendar length had no control at all despite driving
 * storm seasons and trade seasons. Everything that feeds the simulation is now
 * here, on the left, next to the generation steps that consume it; the right
 * column keeps only DISPLAY options (opacity, palettes, overlay toggles).
 *
 * Grouped into collapsible sections because the list is long and most sessions
 * only touch one group:
 *
 *   🪐 Planet         rotation (incl. retrograde) · sunlight · greenhouse · eccentricity
 *   🌍 Axis & Seasons axial tilt · calendar length
 *   💧 Water & Air    dryness
 *   🧭 Latitude Frame equator position · band expansion · line proportion
 *
 * Every knob defaults to Earth, where the physics is a no-op by construction
 * (see CLAUDE.md §3.5), so an untouched world generates exactly as before.
 * Settings-only — nothing to "generate" — so the step always auto-completes and
 * never blocks Continue.
 */
export function StepWorldCharacteristics() {
  const meta = useWorldStore((s) => s.meta);
  const setMeta = useWorldStore((s) => s.setMeta);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const bioParams = useUIStore((s) => s.bioParams);
  const setBioParams = useUIStore((s) => s.setBioParams);

  const [planet, setPlanet] = useState<PlanetConfig | null>(null);
  useEffect(() => { getPlanetConfig().then(setPlanet).catch(() => {}); }, [meta?.name]);

  // Collapsible sections. Planet and Latitude Frame open by default — those are
  // the two you can only really judge against a map you can see, which is why
  // this step now sits after Landmass. Axis and Water are one click away.
  const [open, setOpen] = useState<Record<string, boolean>>({
    Planet: true, Axis: false, Water: false, Latitude: true,
  });
  const toggle = (k: string) => setOpen((s) => ({ ...s, [k]: !s[k] }));

  // Settings-only step — mark done once on mount so Continue is never blocked.
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
        Your land is on the map — now decide the world it sits in. Everything that
        governs where climate belts, deserts and seasons fall lives here, and all
        of it is read first by <b>Ocean &amp; Atmosphere (3)</b>, so set it before
        that. Elevation ignores these entirely, and changing one later just means
        re-running from step 3 onward. All default to Earth, where they are exactly
        no-ops. Drag <b>Equator</b> below and the latitude lines appear on the map
        so you can place the tropics across your continents by eye.
      </div>

      {/* ── 🪐 Planet ── */}
      <Section title="🪐 Planet" open={open.Planet} onToggle={() => toggle("Planet")}>
        <PlanetSlider
          label="Rotation" unit="× Earth" value={Math.abs(planet.rotationRate)}
          min={0.25} max={4} step={0.05} digits={2}
          onChange={(v) => commitPlanet({ ...planet, rotationRate: planet.rotationRate < 0 ? -v : v })}
          presets={[["Slow ½×", 0.5], ["Earth 1×", 1], ["Fast 2×", 2]]}
          hint="Sets the wind belts / Hadley cell: slower = wider tropics, deserts & storm tracks pushed poleward, fewer bands; faster = tighter banding."
        />
        <label style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 10, color: "#5a7090", cursor: "pointer", marginTop: 3 }}>
          <input type="checkbox" checked={planet.rotationRate < 0}
            onChange={(e) => commitPlanet({ ...planet, rotationRate: Math.abs(planet.rotationRate) * (e.target.checked ? -1 : 1) })}
            style={{ accentColor: "#3a80c0" }} />
          Retrograde (spins backwards — mirrors trade-wind direction and which
          coasts get the warm/cold boundary currents; belt latitudes are unchanged)
        </label>
        <PlanetSlider
          label="Sunlight" unit="× Earth" value={planet.solarLum}
          min={0.5} max={1.6} step={0.01} digits={2}
          onChange={(v) => commitPlanet({ ...planet, solarLum: v })}
          presets={[["Dim 0.9×", 0.9], ["Earth 1×", 1], ["Bright 1.1×", 1.1]]}
          hint="Stellar irradiance (a fainter/brighter star or a wider/closer orbit): scales total insolation and the global-mean temperature."
        />
        <PlanetSlider
          label="Greenhouse" unit="× Earth" value={planet.greenhouse}
          min={0} max={3} step={0.05} digits={2}
          onChange={(v) => commitPlanet({ ...planet, greenhouse: v })}
          presets={[["Icehouse ½×", 0.5], ["Earth 1×", 1], ["Hothouse 2×", 2]]}
          hint="Global warming from atmospheric trapping: higher warms the whole planet and flattens the equator-pole gradient; low enough tips toward a snowball."
        />
        <PlanetSlider
          label="Eccentricity" unit="" value={planet.eccentricity}
          min={0} max={0.4} step={0.005} digits={3}
          onChange={(v) => commitPlanet({ ...planet, eccentricity: v })}
          presets={[["Circular 0", 0], ["Earth .017", 0.0167], ["High .2", 0.2]]}
          hint="Orbit shape: higher makes one hemisphere's seasons shorter & sharper than the other's."
        />
      </Section>

      {/* ── 🌍 Axis & Seasons ── */}
      <Section title="🌍 Axis & Seasons" open={open.Axis} onToggle={() => toggle("Axis")}>
        <div style={row}>
          <span style={lbl}>Axial tilt</span>
          <span style={val}>{obliquity.toFixed(1)}°</span>
        </div>
        <div style={ctrlRow}>
          <input
            type="range" min={0} max={80} step={0.5} value={obliquity}
            onChange={(e) => setTiltStr(String(e.target.value))}
            onPointerUp={(e) => setTilt(Number((e.target as HTMLInputElement).value))}
            onKeyUp={(e) => setTilt(Number((e.target as HTMLInputElement).value))}
            style={range}
          />
          <input
            value={tiltStr} onChange={(e) => setTiltStr(e.target.value)}
            onBlur={commitTilt} onKeyDown={onEnter} inputMode="decimal" style={numInput}
          />
        </div>
        <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
          <button style={chip(Math.abs(obliquity - 0) < 1e-3)} onClick={() => setTilt(0)}>None 0°</button>
          <button style={chip(Math.abs(obliquity - 23.44) < 0.05)} onClick={() => setTilt(23.44)}>Earth 23.4°</button>
          <button style={chip(Math.abs(obliquity - 45) < 0.05)} onClick={() => setTilt(45)}>Extreme 45°</button>
        </div>
        <div style={hint}>
          Drives the seasons: 0° = none anywhere, 23.4° = Earth-like, higher = harsher
          winters/summers (the tropics reach the poles past 45°).
        </div>

        <div style={{ ...row, marginTop: 8 }}>
          <span style={lbl}>Calendar length</span>
          <span style={val}>{bioParams.calendarMonths} moons</span>
        </div>
        <input
          type="range" min={4} max={24} step={1} value={bioParams.calendarMonths}
          onChange={(e) => setBioParams({ calendarMonths: Number(e.target.value) })}
          style={{ ...range, width: "100%" }}
        />
        <div style={hint}>
          How many "moons" the year is divided into. Sets the scale for the seasonal
          storm calendar and for seasonal trade closures (Biological-Trade step and
          the storm-month scrubber in the right-hand Overlays panel).
        </div>
      </Section>

      {/* ── 💧 Water & Air ── */}
      <Section title="💧 Water & Air" open={open.Water} onToggle={() => toggle("Water")}>
        <PlanetSlider
          label="Dryness" unit="× Earth" value={planet.dryness}
          min={0.3} max={3} step={0.05} digits={2}
          onChange={(v) => commitPlanet({ ...planet, dryness: v })}
          presets={[["Wet ½×", 0.5], ["Earth 1×", 1], ["Arid 2×", 2]]}
          hint="Global precipitation multiplier: higher shrinks rainfall everywhere (deserts expand), lower expands rainforest/monsoon belts. A coarse final knob, not a mechanism."
        />
      </Section>

      {/* ── 🧭 Latitude Frame (moved here from the right-side Toolbar) ── */}
      <Section title="🧭 Latitude Frame" open={open.Latitude} onToggle={() => toggle("Latitude")}>
        <LatitudeFrame />
      </Section>

      <button disabled style={{ ...genBtn, opacity: 0.5, cursor: "default", textAlign: "center" }}>
        Settings saved automatically
      </button>
    </div>
  );
}

/** A collapsible settings group. */
function Section({ title, open, onToggle, children }: {
  title: string; open: boolean; onToggle: () => void; children: React.ReactNode;
}) {
  return (
    <div style={{ background: "#0a1018", border: "1px solid #1e2a38", borderRadius: 4, marginBottom: 2 }}>
      <div
        onClick={onToggle}
        title={open ? "Collapse" : "Expand"}
        style={{
          display: "flex", alignItems: "center", justifyContent: "space-between",
          padding: "5px 7px", cursor: "pointer", userSelect: "none",
          fontSize: 10, fontWeight: 600, color: "#6090b0",
          letterSpacing: 0.4,
        }}
      >
        <span>{title}</span>
        <span style={{ fontSize: 8, color: "#4a6a8a" }}>{open ? "▼" : "▶"}</span>
      </div>
      {open && <div style={{ padding: "0 7px 7px" }}>{children}</div>}
    </div>
  );
}
