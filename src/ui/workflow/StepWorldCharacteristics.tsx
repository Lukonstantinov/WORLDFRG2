import { useCallback, useEffect, useRef, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { getPlanetConfig, setPlanetConfig, setLatitudeConfig, previewZonalProfile,
  previewCoarseClimate, type PlanetConfig, type PreviewSettings } from "@bridge";
import type { ZonalProfile, CoarsePreview } from "@types";
import { PlanetSlider, LatitudeFrame, row, lbl, val, ctrlRow, range, numInput, hint, chip }
  from "@ui/workflow/PlanetControls";
import { ZonalStrip, ZonalReadout } from "@ui/workflow/ZonalStrip";
import { ClimateMixPreview } from "@ui/workflow/ClimateMixPreview";
import { ARCHETYPES, FRAME_PRESETS, EARTH_SETTINGS, archetypeAt, earthDiff }
  from "@ui/workflow/planetArchetypes";
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
 *
 * Two PREVIEWS make the knobs legible instead of guesswork:
 *
 *  - The **zonal strip** redraws on every change. It is a 1-D problem (two EBM
 *    solves plus closed-form circulation), so it costs microseconds and needs no
 *    land. It answers "how warm, how seasonal, where are the belts".
 *  - The **climate mix** runs the real phase-3 → Köppen chain on a downsampled
 *    copy of the user's own landmass. It costs a few hundred ms, so it sits
 *    behind a button, and it answers the question the strip cannot: where the
 *    deserts actually land, and what share of THIS world each climate takes.
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
    Planet: true, Axis: false, Water: false, Latitude: true, Archetypes: true,
  });
  const toggle = (k: string) => setOpen((s) => ({ ...s, [k]: !s[k] }));

  // ── Previews ──
  const latConfig = useWorldStore((s) => s.latConfig);
  const [zonal, setZonal] = useState<ZonalProfile | null>(null);
  const [coarse, setCoarse] = useState<CoarsePreview | null>(null);
  const [coarseBusy, setCoarseBusy] = useState(false);
  const [coarseErr, setCoarseErr] = useState<string | null>(null);
  // Which archetype is selected and how hard, so the dial can be re-dragged
  // without re-picking the archetype.
  const [archId, setArchId] = useState<string | null>(null);
  const [intensity, setIntensity] = useState(0.5);

  /** The settings the previews should run against — the live UI values, which
   *  may differ from what is persisted mid-drag. */
  const settings: PreviewSettings | null = planet && meta ? {
    obliquity: meta.obliquity ?? 23.44,
    rotationRate: planet.rotationRate,
    solarLum: planet.solarLum,
    greenhouse: planet.greenhouse,
    eccentricity: planet.eccentricity,
    dryness: planet.dryness,
    equatorOffset: latConfig.equatorOffset,
    latScale: latConfig.latScale,
    latRatio: latConfig.lineRatio,
  } : null;

  // Tier 1 redraws whenever anything changes. It is cheap enough that a plain
  // effect is fine — but the call is still async, so guard against an older
  // response landing after a newer one and clobbering it.
  const zonalSeq = useRef(0);
  const sig = settings ? JSON.stringify(settings) : "";
  useEffect(() => {
    if (!settings) return;
    const mine = ++zonalSeq.current;
    previewZonalProfile(settings)
      .then((z) => { if (mine === zonalSeq.current) setZonal(z); })
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sig]);

  // Tier 2 is explicit — it costs real time and loads the full-resolution
  // landmass, so it must never fire from a slider.
  const runCoarse = useCallback(async () => {
    if (!settings || coarseBusy) return;
    setCoarseBusy(true);
    setCoarseErr(null);
    try {
      setCoarse(await previewCoarseClimate(settings));
    } catch (e) {
      setCoarse(null);
      setCoarseErr(String(e));
    }
    setCoarseBusy(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sig, coarseBusy]);

  /** Apply an archetype at a given dial position. Only the knobs the archetype
   *  names are touched — everything else keeps the user's value, so picking one
   *  never silently discards an unrelated setting they had tuned. */
  const applyArchetype = (id: string, t: number) => {
    if (!planet) return;
    const a = ARCHETYPES.find((x) => x.id === id);
    if (!a) return;
    const k = a.id === "earth" ? EARTH_SETTINGS : archetypeAt(a, t);
    const next: PlanetConfig = {
      ...planet,
      rotationRate: k.rotationRate ?? planet.rotationRate,
      solarLum: k.solarLum ?? planet.solarLum,
      greenhouse: k.greenhouse ?? planet.greenhouse,
      eccentricity: k.eccentricity ?? planet.eccentricity,
      dryness: k.dryness ?? planet.dryness,
    };
    commitPlanet(next);
    // Tilt and the latitude frame live on world metadata, not PlanetConfig.
    const lc = useWorldStore.getState().latConfig;
    const tilt = k.obliquity ?? (meta?.obliquity ?? 23.44);
    const eq = k.equatorOffset ?? lc.equatorOffset;
    const sc = k.latScale ?? lc.latScale;
    if (k.obliquity !== undefined || k.equatorOffset !== undefined || k.latScale !== undefined) {
      useWorldStore.getState().setLatConfig(eq, sc, lc.lineRatio);
      setLatitudeConfig(eq, sc, lc.lineRatio, tilt).then(setMeta).catch(() => {});
    }
    // A fresh archetype invalidates the coarse map — it is no longer of this world.
    setCoarse(null);
  };

  const applyFrame = (eq: number, sc: number) => {
    const lc = useWorldStore.getState().latConfig;
    useWorldStore.getState().setLatConfig(eq, sc, lc.lineRatio);
    setLatitudeConfig(eq, sc, lc.lineRatio, meta?.obliquity ?? 23.44).then(setMeta).catch(() => {});
    setCoarse(null);
  };

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

      {/* ── The zonal strip: always visible, redraws on every change ── */}
      <div style={{ background: "#0a1018", border: "1px solid #1e2a38", borderRadius: 4, padding: 6 }}>
        <div style={{ color: "#6090b0", fontSize: 10, fontWeight: 600, marginBottom: 3 }}>
          What this world does
        </div>
        <ZonalStrip profile={zonal} />
        <ZonalReadout profile={zonal} />

        {/* Guardrails. These are real cliffs in the physics, not style advice. */}
        {zonal?.snowballRisk && (
          <div style={warnBox("#3a1c14", "#e08050")}>
            ⚠ <b>Runaway glaciation.</b> Permanent ice has reached {Math.round(zonal.iceLine)}°.
            The ice-albedo feedback amplifies itself from here — colder settings will
            freeze the planet solid and leave almost nothing habitable.
          </div>
        )}
        {zonal?.beltsCollapsed && (
          <div style={warnBox("#2a2414", "#d8b860")}>
            ⚠ <b>Circulation collapsed.</b> The Hadley cell has widened until it
            meets the polar front, so the familiar three-cell structure is gone —
            one vast overturning cell per hemisphere. Climate bands will be very
            unlike Earth's.
          </div>
        )}

        {/* How far from Earth are we? */}
        {settings && (() => {
          const diff = earthDiff(settings);
          return (
            <div style={{ fontSize: 9, color: diff.length ? "#7f95ad" : "#456", marginTop: 5,
              borderTop: "1px solid #16202c", paddingTop: 4, lineHeight: 1.45 }}>
              {diff.length === 0
                ? "Earth standard — every knob at its calibrated value (exact no-op)."
                : <>{diff.length} setting{diff.length === 1 ? "" : "s"} differ from Earth: {diff.join(" · ")}</>}
            </div>
          );
        })()}
      </div>

      {/* ── Archetypes ── */}
      <Section title="🧭 World archetypes" open={open.Archetypes} onToggle={() => toggle("Archetypes")}>
        <div style={{ color: "#506880", fontSize: 9, lineHeight: 1.4, marginBottom: 5 }}>
          Each archetype is a <b>span</b>, not a fixed point — pick one, then use its
          dial to decide how far to take it. Only the knobs an archetype names are
          touched, so your other settings survive.
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 3 }}>
          {ARCHETYPES.map((a) => {
            const active = archId === a.id;
            return (
              <button key={a.id} title={a.blurb}
                onClick={() => { setArchId(a.id); setIntensity(a.defaultIntensity); applyArchetype(a.id, a.defaultIntensity); }}
                style={{
                  padding: "3px 5px", borderRadius: 3, cursor: "pointer", fontSize: 9,
                  border: active ? "1px solid #3a7ac0" : "1px solid #1e2e42",
                  background: active ? "#1a3a5a" : "#0d1219",
                  color: active ? "#c0ddf0" : "#6a8aaa",
                  fontWeight: active ? 600 : 400,
                }}>
                {a.icon} {a.label}
              </button>
            );
          })}
        </div>

        {archId && (() => {
          const a = ARCHETYPES.find((x) => x.id === archId)!;
          const variable = a.id !== "earth" && a.id !== "retrograde";
          return (
            <div style={{ marginTop: 6 }}>
              <div style={{ color: "#8aa0b8", fontSize: 9.5, lineHeight: 1.45, marginBottom: 4 }}>
                {a.blurb}
              </div>
              {variable && (
                <>
                  <div style={row}>
                    <span style={lbl}>{a.dial}</span>
                    <span style={val}>{Math.round(intensity * 100)}%</span>
                  </div>
                  <input type="range" min={0} max={100} step={1} value={Math.round(intensity * 100)}
                    onChange={(e) => setIntensity(Number(e.target.value) / 100)}
                    onPointerUp={(e) => applyArchetype(a.id, Number((e.target as HTMLInputElement).value) / 100)}
                    onKeyUp={(e) => applyArchetype(a.id, Number((e.target as HTMLInputElement).value) / 100)}
                    style={{ ...range, width: "100%" }} />
                </>
              )}
              {!variable && (
                <div style={{ fontSize: 9, color: "#506880" }}>{a.dial}</div>
              )}
              <div style={{ fontSize: 9, color: "#5a7390", marginTop: 4, lineHeight: 1.45,
                background: "#0d1622", borderLeft: "2px solid #24405c", padding: "4px 6px", borderRadius: 2 }}>
                <b style={{ color: "#7fa8c8" }}>Expect:</b> {a.expect}
              </div>
            </div>
          );
        })()}

        <div style={{ color: "#3a5e70", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.6,
          marginTop: 8, marginBottom: 3 }}>
          Map frame
        </div>
        <div style={{ color: "#506880", fontSize: 9, lineHeight: 1.4, marginBottom: 4 }}>
          Not physics — which latitudes the map SHOWS. Useful when fitting a drawn
          or imported map.
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 3 }}>
          {FRAME_PRESETS.map((f) => (
            <button key={f.id} title={f.blurb}
              onClick={() => applyFrame(f.equatorOffset, f.latScale)}
              style={{
                padding: "3px 5px", borderRadius: 3, cursor: "pointer", fontSize: 9,
                border: "1px solid #1e2e42", background: "#0d1219", color: "#6a8aaa",
              }}>
              {f.label}
            </button>
          ))}
        </div>
      </Section>

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

      {/* ── Climate mix: the real chain on the real landmass ── */}
      <div style={{ background: "#0a1018", border: "1px solid #223c30", borderRadius: 4, padding: 6 }}>
        <div style={{ color: "#6ab08a", fontSize: 10, fontWeight: 600, marginBottom: 3 }}>
          🗺 Climate mix on your land
        </div>
        <div style={{ color: "#506880", fontSize: 9, lineHeight: 1.4, marginBottom: 5 }}>
          The strip above knows how wide the desert BELT is. Only this knows whether
          your continent sits under it. Runs the real Ocean &amp; Atmosphere → Köppen
          chain on a downsampled copy of your landmass — a second or so, and nothing
          is written to the world.
        </div>
        <button onClick={runCoarse} disabled={coarseBusy}
          style={{ ...genBtn, background: "#13301f", color: "#a8d8b8", textAlign: "center", marginBottom: 0 }}>
          {coarseBusy ? "Running climate…" : coarse ? "↻ Re-run preview" : "▶ Preview climate mix"}
        </button>
        {coarseErr && (
          <div style={{ ...warnBox("#3a1c14", "#e08050"), marginTop: 5 }}>{coarseErr}</div>
        )}
        {coarse && !coarseBusy && (
          <div style={{ marginTop: 6 }}>
            <ClimateMixPreview preview={coarse} />
          </div>
        )}
      </div>

      <button disabled style={{ ...genBtn, opacity: 0.5, cursor: "default", textAlign: "center" }}>
        Settings saved automatically
      </button>
    </div>
  );
}

/** A warning callout. Used for the two real cliffs in the physics — runaway
 *  glaciation and circulation collapse — which are worth flagging BEFORE the
 *  user falls off them rather than after a 16-second generation. */
function warnBox(bg: string, fg: string): React.CSSProperties {
  return {
    marginTop: 5, padding: "4px 6px", borderRadius: 3,
    background: bg, color: fg, fontSize: 9, lineHeight: 1.45,
  };
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
