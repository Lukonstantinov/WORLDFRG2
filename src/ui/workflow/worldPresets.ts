import type { ArchetypeKnobs, Archetype } from "@ui/workflow/planetArchetypes";
import { archetypeAt, ARCHETYPES, EARTH_SETTINGS } from "@ui/workflow/planetArchetypes";
import type { TerrainParams } from "@state/uiStore";

/**
 * GENERATION_UX_REDESIGN_PLAN.md Slice 5 — WORLD PRESETS, the headline
 * user-facing win. A `WorldPreset` composes what was scattered across three
 * places (`ARCHETYPES` for planetary knobs, `TERRAIN_PRESETS` for elevation
 * sliders, and — until Slice 2 — nowhere at all for ocean fraction /
 * continent count, since those were built and unreachable). Picking one and
 * pressing Generate produces a whole world in one step, which the app did
 * not previously have.
 *
 * Two rules carried over from `planetArchetypes.ts`, because it already
 * learned them the hard way:
 * - **A preset only sets the axes it is about.** Every field here is
 *   OPTIONAL; `applyWorldPreset` only touches what is present, so picking
 *   "Archipelago" never silently discards a tilt set two minutes ago.
 * - **Planetary knobs stay a SPAN, not a point** — `planet` reuses an
 *   existing `Archetype`'s own mild/strong pair (or a preset-local one) and
 *   is interpolated by the SAME `archetypeAt` helper the World
 *   Characteristics step already uses, at the preset's own `defaultIntensity`
 *   dial rather than a hardcoded number.
 *
 * The landmass axes (ocean fraction / continent goal / elevation model +
 * sliders) are simple point values, not spans — a "how much ocean" preset
 * reads naturally as a single target, unlike a planetary knob that wants a
 * mild↔strong range to explore.
 */
export interface WorldPreset {
  id: string;
  label: string;
  icon: string;
  /** One line: what this world IS. */
  blurb: string;
  /** What to expect on the generated map. */
  expect: string;
  /** Share of the world that ends up sea (0..1). */
  oceanFraction?: number;
  /** -1/undefined = as many continents as possible; 0 = fewest (Pangaea); n>0 = nearest n. */
  continentGoal?: number;
  /** Elevation model + its four sliders (all optional; a preset may set only some). */
  elevMode?: TerrainParams["mode"];
  elevDensity?: number;
  elevHeight?: number;
  elevSpread?: number;
  elevRoughness?: number;
  /** Reuse an existing planetary Archetype's span (by id), interpolated at
   *  `planetIntensity` (defaults to that archetype's own `defaultIntensity`). */
  planetArchetype?: string;
  planetIntensity?: number;
  /** Or a preset-local span, for a knob combination no existing Archetype names. */
  planetMild?: ArchetypeKnobs;
  planetStrong?: ArchetypeKnobs;
}

export const WORLD_PRESETS: WorldPreset[] = [
  {
    id: "earthlike",
    label: "Earthlike",
    icon: "🌍",
    blurb: "The calibrated baseline: Earth's ocean fraction, several continents, Earth planetary knobs.",
    expect: "~70% ocean, multiple separate landmasses, temperate-to-tropical climate mix.",
    oceanFraction: 0.70,
    continentGoal: -1,
    elevMode: "plates",
    planetMild: EARTH_SETTINGS,
    planetStrong: EARTH_SETTINGS,
    planetIntensity: 0,
  },
  {
    id: "many-continents",
    label: "Many Continents",
    icon: "🗺️",
    blurb: "The ocean-fill selection pushed toward scattering, not fusing — several separate landmasses guaranteed.",
    expect: "Six or more distinct continents/large islands, Earth-like ocean share.",
    oceanFraction: 0.68,
    continentGoal: 99,
    elevMode: "plates",
  },
  {
    id: "pangaea",
    label: "Pangaea",
    icon: "🦕",
    blurb: "One fused supercontinent — the ocean-fill objective reversed to prefer the FEWEST landmasses.",
    expect: "A single (or near-single) giant landmass surrounded by open ocean.",
    oceanFraction: 0.65,
    continentGoal: 0,
    elevMode: "plates",
  },
  {
    id: "archipelago",
    label: "Archipelago",
    icon: "🏝️",
    blurb: "Mostly sea, with land scattered into many small islands and archipelagos.",
    expect: "~85% ocean, no continent-scale landmass — a world of islands and straits.",
    oceanFraction: 0.85,
    continentGoal: 99,
    elevMode: "shape",
    elevSpread: 0.7,
  },
  {
    id: "ocean-world",
    label: "Ocean World",
    icon: "🌊",
    blurb: "Land is the exception, not the rule — a handful of small continents in a vast sea.",
    expect: "~92% ocean. Coastal, maritime cultures; little interior to speak of.",
    oceanFraction: 0.92,
    continentGoal: -1,
    elevMode: "shape",
  },
  {
    id: "desert-world",
    label: "Desert World",
    icon: "🏜️",
    blurb: "Reuses the Desert World planetary archetype (wider Hadley cell, drier globally) over an Earth-sized landmass.",
    expect: "Deserts merge into one continuous band per hemisphere; arid land can pass 40%.",
    oceanFraction: 0.70,
    continentGoal: -1,
    elevMode: "plates",
    planetArchetype: "desert",
    planetIntensity: 0.7,
  },
  {
    id: "ice-age",
    label: "Ice Age",
    icon: "🧊",
    blurb: "Reuses the Ice House planetary archetype (cooler globally, wider ice caps) over more land than ocean.",
    expect: "Extensive polar and alpine ice; the temperate band is compressed toward the equator.",
    oceanFraction: 0.62,
    continentGoal: -1,
    elevMode: "plates",
    planetArchetype: "icehouse",
    planetIntensity: 0.6,
  },
  {
    id: "hothouse-jungle",
    label: "Hothouse Jungle",
    icon: "🌴",
    blurb: "Reuses the Hothouse planetary archetype (stronger greenhouse, narrower ice caps) over a wet, continental world.",
    expect: "Rainforest and swamp dominate; ice caps shrink to the poles alone, if they survive at all.",
    oceanFraction: 0.68,
    continentGoal: -1,
    elevMode: "plates",
    planetArchetype: "hothouse",
    planetIntensity: 0.6,
  },
  {
    id: "highlands",
    label: "Highlands",
    icon: "⛰️",
    blurb: "Less ocean and the Cordillera elevation model, for a world of continuous mountain chains and high plateaus.",
    expect: "Coast-parallel mountain ranges with a continental divide; little flat lowland.",
    oceanFraction: 0.55,
    continentGoal: -1,
    elevMode: "cordillera",
    elevHeight: 0.75,
    elevDensity: 0.6,
  },
  {
    id: "tidal-locked-extreme",
    label: "Tidal-Locked Extreme",
    icon: "🌗",
    blurb: "Reuses the Slow Spinner planetary archetype (near-zero rotation) at high intensity — an extreme day-night climate split.",
    expect: "Circulation belts collapse toward the poles; one hemisphere reads starkly different from the other in the settings preview.",
    oceanFraction: 0.65,
    continentGoal: -1,
    elevMode: "plates",
    planetArchetype: "slow",
    planetIntensity: 1,
  },
];

/** Resolve a preset's planetary knobs (if any) to a concrete `ArchetypeKnobs`,
 *  reusing `archetypeAt` — the same interpolation the World Characteristics
 *  step already uses, so a preset's planetary span behaves identically to
 *  picking that archetype by hand. */
export function worldPresetPlanetKnobs(p: WorldPreset): ArchetypeKnobs | null {
  if (p.planetArchetype) {
    const a = ARCHETYPES.find((x: Archetype) => x.id === p.planetArchetype);
    if (!a) return null;
    return archetypeAt(a, p.planetIntensity ?? a.defaultIntensity);
  }
  if (p.planetMild || p.planetStrong) {
    const fake: Archetype = {
      id: p.id, label: p.label, icon: p.icon, blurb: p.blurb, expect: p.expect,
      mild: p.planetMild ?? {}, strong: p.planetStrong ?? {},
      dial: "", defaultIntensity: p.planetIntensity ?? 1,
    };
    return archetypeAt(fake, p.planetIntensity ?? 1);
  }
  return null;
}
