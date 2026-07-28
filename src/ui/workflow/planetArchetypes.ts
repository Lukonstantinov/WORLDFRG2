import type { PreviewSettings } from "@bridge";

/**
 * World ARCHETYPES for the World Characteristics step.
 *
 * A preset that slams six sliders is unregulatable — you get the look or you
 * don't, and there is nowhere to go from there. So each archetype here is a
 * **span**, not a point: `mild` and `strong` endpoints that an intensity dial
 * interpolates between. Dragging the dial to 0 gives a gentle version of the
 * idea, to 1 an extreme one, and every value between is a real world.
 *
 * Only the fields an archetype actually cares about are listed; everything else
 * keeps the user's current value, so picking "Fast Spinner" does not silently
 * throw away the tilt they set two minutes ago.
 *
 * `Earth Standard` is special: it restores EVERY knob to its Earth value,
 * because the whole planetary system is built on Earth parameters being an
 * exact no-op (CLAUDE.md §3.5). It is the reset, and it must be complete.
 */

/** The subset of planetary settings an archetype may drive. */
export type ArchetypeKnobs = Partial<Omit<PreviewSettings, "latRatio">>;

export interface Archetype {
  id: string;
  label: string;
  icon: string;
  /** One line: what this world DOES, not what the sliders read. */
  blurb: string;
  /** What to expect on the map once it generates. */
  expect: string;
  /** Settings at intensity 0 — the gentle reading of the idea. */
  mild: ArchetypeKnobs;
  /** Settings at intensity 1 — the extreme reading. */
  strong: ArchetypeKnobs;
  /** Label for the intensity dial (each archetype means something different by it). */
  dial: string;
  /** Default dial position when the archetype is first picked. */
  defaultIntensity: number;
}

export const EARTH_SETTINGS: PreviewSettings = {
  obliquity: 23.44,
  rotationRate: 1,
  solarLum: 1,
  greenhouse: 1,
  eccentricity: 0.0167,
  dryness: 1,
  equatorOffset: 0.5,
  latScale: 1,
  latRatio: 1,
};

export const ARCHETYPES: Archetype[] = [
  {
    id: "earth",
    label: "Earth Standard",
    icon: "🌍",
    blurb: "The calibrated baseline. Every knob at its Earth value, where the physics is an exact no-op.",
    expect: "Tropics to 30°, deserts in the subtropical highs, storm tracks near 60°, ice caps at both poles.",
    mild: EARTH_SETTINGS,
    strong: EARTH_SETTINGS,
    dial: "(no variation — this is the reset)",
    defaultIntensity: 0,
  },
  {
    id: "desert",
    label: "Desert World",
    icon: "🏜️",
    blurb: "Dry AND structurally dry: less rain everywhere, plus a wider Hadley cell that drags the subtropical high poleward and broadens the dry belt.",
    expect: "Deserts merge into one continuous band per hemisphere. Forest survives only on the ITCZ and windward coasts. Expect arid to pass 40% of land.",
    // Aridity and belt width move together — dryness alone just scales rain
    // down uniformly and reads flat, because it is a post-multiplier, not a
    // mechanism. Rotation is what actually moves the desert belt.
    mild: { dryness: 1.35, rotationRate: 0.85 },
    strong: { dryness: 2.4, rotationRate: 0.6 },
    dial: "Aridity & belt width",
    defaultIntensity: 0.45,
  },
  {
    id: "hothouse",
    label: "Hothouse Jungle",
    icon: "🌴",
    blurb: "Thick air, wet, and barely seasonal. The equator-to-pole gradient flattens, so the whole world calms down.",
    expect: "Ice caps gone or vestigial. Tropical climates reach past 40°. Continental (D) and polar (E) nearly disappear.",
    mild: { greenhouse: 1.45, dryness: 0.85, obliquity: 20 },
    strong: { greenhouse: 2.3, dryness: 0.6, obliquity: 12 },
    dial: "Warmth & humidity",
    defaultIntensity: 0.5,
  },
  {
    id: "icehouse",
    label: "Ice House",
    icon: "🧊",
    blurb: "A thin greenhouse. Ice advances from both poles and the habitable band squeezes toward the equator.",
    expect: "Permanent ice down to roughly 50–60° and a fifth to a quarter of land gone polar, with the temperate belt pinched into a ribbon. The dial stops short of the runaway — push greenhouse below ~0.55 by hand and the planet freezes solid.",
    // Endpoints measured with `sweep_cooling_knobs`, NOT guessed. The ice-albedo
    // collapse is abrupt (alive at greenhouse 0.55, total snowball at 0.50), so
    // the strong end stops at 0.58 — a severe ice age that is still a world.
    // Luminosity is deliberately left alone here; stacking both knobs crosses the
    // cliff, and "Dim Red Sun" already owns the luminosity axis.
    mild: { greenhouse: 0.82 },
    strong: { greenhouse: 0.58 },
    dial: "Depth of the freeze",
    defaultIntensity: 0.4,
  },
  {
    id: "slow",
    label: "Slow Spinner",
    icon: "🐌",
    blurb: "A long day. Held–Hou scaling widens the Hadley cell enormously, so the three-cell structure collapses toward one vast overturning cell per hemisphere.",
    expect: "Tropics reaching 45–70°. Deserts flung to high latitudes. Weak Coriolis, so currents and winds run far more meridionally.",
    mild: { rotationRate: 0.7 },
    strong: { rotationRate: 0.28 },
    dial: "How slow the day",
    defaultIntensity: 0.5,
  },
  {
    id: "fast",
    label: "Fast Spinner",
    icon: "🌀",
    blurb: "A short day. The Hadley cell tightens toward the equator and extra circulation cells appear between the belts.",
    expect: "A striped, banded world: narrow tropics near 15–20°, storm tracks near 40°, many sharp climate bands rather than a few broad ones.",
    mild: { rotationRate: 1.8 },
    strong: { rotationRate: 3.6 },
    dial: "How fast the day",
    defaultIntensity: 0.5,
  },
  {
    id: "toppled",
    label: "Toppled World",
    icon: "🪐",
    blurb: "Axis tipped on its side. Past about 54° of tilt the POLES receive more annual sunlight than the equator, and the usual climate logic inverts.",
    expect: "Seasonally the hottest places on the planet are the polar regions. Violent seasons everywhere; the tropics become the mild, stable band.",
    mild: { obliquity: 42 },
    strong: { obliquity: 75 },
    dial: "Axial tip",
    defaultIntensity: 0.5,
  },
  {
    id: "static",
    label: "Static World",
    icon: "⏸️",
    blurb: "No axial tilt, so no seasonal cycle anywhere. Climate bands become fixed and razor-sharp.",
    expect: "Zones that never move all year. No monsoons at all — a monsoon needs a seasonal wind reversal to exist. Very stable ice caps.",
    mild: { obliquity: 6 },
    strong: { obliquity: 0 },
    dial: "Toward zero tilt",
    defaultIntensity: 1,
  },
  {
    id: "eccentric",
    label: "Eccentric Orbit",
    icon: "🥚",
    blurb: "A markedly elliptical orbit. The two hemispheres stop matching: one gets short fierce summers, the other long mild ones.",
    expect: "Asymmetric hemispheres — the same latitude north and south will not have the same climate. A useful hook for a divided world.",
    mild: { eccentricity: 0.12 },
    strong: { eccentricity: 0.38 },
    dial: "Orbital stretch",
    defaultIntensity: 0.5,
  },
  {
    id: "retrograde",
    label: "Retrograde World",
    icon: "↩️",
    blurb: "The planet spins backwards. Trade winds reverse, and the warm and cold boundary currents swap coasts.",
    expect: "Mediterranean climates and cold fog deserts move to the OPPOSITE side of every continent. Belt latitudes are unchanged — only direction flips.",
    mild: { rotationRate: -1 },
    strong: { rotationRate: -1 },
    dial: "(direction only — pair with another archetype)",
    defaultIntensity: 0,
  },
  {
    id: "dimsun",
    label: "Dim Red Sun",
    icon: "🔴",
    blurb: "A faint star with a thick blanket of greenhouse gas compensating. Two knobs pulling against each other.",
    expect: "Cool but genuinely habitable, with a compressed temperate band and long twilight-toned seasons. Less total energy, so weather is calmer.",
    mild: { solarLum: 0.85, greenhouse: 1.35 },
    strong: { solarLum: 0.65, greenhouse: 1.9 },
    dial: "Faintness of the star",
    defaultIntensity: 0.5,
  },
  {
    id: "superhabitable",
    label: "Superhabitable",
    icon: "🌾",
    blurb: "Slightly warmer and wetter than Earth everywhere, with a gentler pole-to-equator gradient.",
    expect: "Broad temperate belts, small ice caps, generous rainfall. The most forgiving world to settle — expect high habitability almost everywhere.",
    mild: { greenhouse: 1.15, dryness: 0.9 },
    strong: { greenhouse: 1.45, dryness: 0.7, obliquity: 20 },
    dial: "Generosity",
    defaultIntensity: 0.5,
  },
];

/** Latitude-FRAME presets. Not physics — these crop and stretch which latitudes
 *  the map shows, for when you are fitting a drawn or imported map. */
export interface FramePreset {
  id: string;
  label: string;
  blurb: string;
  equatorOffset: number;
  latScale: number;
}

export const FRAME_PRESETS: FramePreset[] = [
  { id: "full", label: "Whole globe", blurb: "Pole to pole, equator centred — the default.", equatorOffset: 0.5, latScale: 1 },
  { id: "north", label: "Northern realm", blurb: "Crops to roughly 20°–80°N: a northern continent filling the map.", equatorOffset: 1.33, latScale: 3 },
  { id: "south", label: "Southern realm", blurb: "The mirror image — roughly 20°–80°S.", equatorOffset: -0.33, latScale: 3 },
  { id: "tropic", label: "Equatorial belt", blurb: "Crops to about ±30°: an archipelago-and-jungle world with no polar regions at all.", equatorOffset: 0.5, latScale: 3 },
  { id: "temperate", label: "Temperate band", blurb: "Roughly 25°–70°N — the classic 'medieval Europe' frame.", equatorOffset: 1.1, latScale: 2.4 },
];

/** Linear interpolation between an archetype's mild and strong endpoints. */
export function archetypeAt(a: Archetype, intensity: number): ArchetypeKnobs {
  const t = Math.max(0, Math.min(1, intensity));
  const out: ArchetypeKnobs = {};
  const keys = new Set([...Object.keys(a.mild), ...Object.keys(a.strong)]) as Set<keyof ArchetypeKnobs>;
  for (const k of keys) {
    const lo = a.mild[k];
    const hi = a.strong[k];
    if (lo === undefined && hi === undefined) continue;
    if (lo === undefined) { out[k] = hi; continue; }
    if (hi === undefined) { out[k] = lo; continue; }
    out[k] = lo + (hi - lo) * t;
  }
  return out;
}

/** Which settings differ from Earth, as short human strings ("greenhouse +0.80×").
 *  Drives the "you are N settings away from Earth" readout. */
export function earthDiff(s: PreviewSettings): string[] {
  const out: string[] = [];
  const d = (label: string, v: number, base: number, unit: string, digits = 2, eps = 1e-3) => {
    if (Math.abs(v - base) <= eps) return;
    const sign = v > base ? "+" : "−";
    out.push(`${label} ${sign}${Math.abs(v - base).toFixed(digits)}${unit}`);
  };
  // Rotation direction is a category, not a magnitude — report it as such.
  if (s.rotationRate < 0) out.push("retrograde");
  d("rotation", Math.abs(s.rotationRate), 1, "×");
  d("sunlight", s.solarLum, 1, "×");
  d("greenhouse", s.greenhouse, 1, "×");
  d("tilt", s.obliquity, 23.44, "°", 1, 0.05);
  d("eccentricity", s.eccentricity, 0.0167, "", 3);
  d("dryness", s.dryness, 1, "×");
  d("equator", s.equatorOffset * 100, 50, "%", 0, 0.5);
  d("expansion", s.latScale * 100, 100, "%", 0, 0.5);
  return out;
}
