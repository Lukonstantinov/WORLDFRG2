import type { ActiveLayer } from "@types";
import { useUIStore } from "@state/uiStore";
import { useSettingsStore, COLOR_PRESETS, type PresetKey } from "@state/settingsStore";

/** MAP PLATES — an atlas is not a stack of single layers.
 *
 *  A published atlas never shows one raster on its own: a climate plate is Köppen
 *  fill PLUS the circulation that produces it PLUS the graticule, set in the face
 *  that plate uses. A political plate is a province wash PLUS borders PLUS city
 *  dots sized by rank. What distinguishes one plate from another is the whole
 *  composition, not the base colour.
 *
 *  A `MapTheme` is exactly that composition, expressed in state that already
 *  exists: one base render layer (`ActiveLayer`), a set of vector overlays, and
 *  optionally the label typography (§8.11) and overlay-line palette that suit it.
 *  Nothing here is new simulation state and nothing is persisted — a plate is a
 *  VIEW, so switching plates can never alter a world.
 *
 *  Rule 14 (CLAUDE.md §10): plates are display-only. A plate may never write a
 *  generation setting — it sets what you SEE, never what the world IS. */

export interface MapTheme {
  id: string;
  /** Plate name as it reads in the picker. */
  name: string;
  /** One glyph for the compact Toolbar chip. */
  glyph: string;
  /** The question this plate answers — shown under the name. */
  blurb: string;
  /** The base raster. */
  layer: ActiveLayer;
  /** Overlays switched ON. Every other MANAGED overlay is switched off, so a
   *  plate always lands in a known composition rather than inheriting leftovers. */
  overlays: string[];
  /** Base-raster opacity (default 1). */
  layerOpacity?: number;
  /** Province fill strength, for plates that wash the partition (default 0.5). */
  provinceOpacity?: number;
  /** Label typography theme (§8.11) — omit to leave the user's choice alone. */
  labelTheme?: string;
  /** Overlay-line palette preset — omit to leave the user's choice alone. */
  linePreset?: PresetKey;
  /** Workflow step whose data this plate needs. Shown dimmed until it is done. */
  requires: number;
  /** An elevation STYLE (§ elevation styles) to apply when `layer` is
   *  "elevation"/"terrain" — omit for the plain default rendering. Lets a plate
   *  give those two layers a real identity of their own instead of sharing the
   *  same flat/hillshade pair every other view would also see. */
  elevationStyle?: string;
}

/** The plates, ordered the way the pipeline builds the world: the land first, then
 *  the air and sea over it, then what grows, then who lives there, then what they
 *  trade and what can kill them. */
export const MAP_THEMES: MapTheme[] = [
  {
    id: "physical",
    name: "Physical",
    glyph: "▲",
    blurb: "Shaded relief with its water and the named country of the land.",
    layer: "terrain",
    overlays: [
      "rivers", "lakes", "riverBreaks", "latLines",
      "toponyms", "toponymsMountain", "toponymsRiver", "toponymsLake",
      "toponymsDesert", "toponymsForest", "toponymsTundra",
    ],
    labelTheme: "Engraved Antique",
    requires: 2,
  },
  {
    id: "natural",
    name: "Natural Colour",
    glyph: "\u25D5",
    blurb: "The land as it would look from orbit — cover, not height — over shaded relief and shaded sea.",
    layer: "natural",
    overlays: ["rivers", "lakes", "latLines"],
    labelTheme: "Modern Cartographic",
    requires: 6,
  },
  {
    id: "relief",
    name: "Relief & Height",
    glyph: "◭",
    blurb: "Hypsometric tint — height read as colour rather than as shadow.",
    layer: "elevation",
    overlays: ["rivers", "lakes", "latLines"],
    labelTheme: "Classic Atlas",
    requires: 2,
    // Layer Colouring: classed hypsometric bands (the Bartholomew/Times
    // convention) instead of the plain unshaded default — the default
    // "elevation" layer carries no shading at all, which used to leave this
    // plate reading as a flatter, less informative copy of "Physical".
    elevationStyle: "layers",
  },
  {
    id: "ocean",
    name: "Ocean & Currents",
    glyph: "≈",
    blurb: "Sea-surface temperature with the gyres and the banks they feed.",
    layer: "sst",
    overlays: ["currents", "fisheryBanks", "latLines"],
    labelTheme: "Modern Cartographic",
    requires: 3,
  },
  {
    id: "climate",
    name: "Climate",
    glyph: "◐",
    blurb: "Köppen zones over the circulation that produces them.",
    layer: "climate",
    overlays: ["itcz", "windBelts", "wind", "latLines"],
    labelTheme: "Modern Cartographic",
    requires: 4,
  },
  {
    id: "hydrology",
    name: "Hydrology",
    glyph: "≋",
    blurb: "Every channel and lake, named, over bare land.",
    layer: "land",
    overlays: ["rivers", "lakes", "riverBreaks", "toponyms", "toponymsRiver", "toponymsLake"],
    labelTheme: "Classic Atlas",
    requires: 5,
  },
  {
    id: "ecology",
    name: "Ecology",
    glyph: "❋",
    blurb: "The 41 biomes over their water — what actually grows here.",
    layer: "biomes",
    overlays: [
      "rivers", "lakes", "toponyms",
      "toponymsForest", "toponymsTundra", "toponymsDesert",
    ],
    labelTheme: "Classic Atlas",
    requires: 6,
  },
  {
    id: "settlement",
    name: "Settlement",
    glyph: "⌂",
    blurb: "Where people can live, and where they did settle.",
    layer: "habitability",
    overlays: ["rivers", "settlements", "settlementNames"],
    labelTheme: "Modern Cartographic",
    requires: 7,
  },
  {
    id: "peoples",
    name: "Peoples",
    glyph: "☗",
    blurb: "Culture territory and the migration that moves it.",
    layer: "land",
    overlays: [
      "cultures", "settlements", "settlementNames", "migrations",
      "toponyms", "toponymsRegion",
    ],
    labelTheme: "Classic Atlas",
    requires: 7,
  },
  {
    id: "political",
    name: "Political",
    glyph: "❖",
    blurb: "Province writ, realm borders and the reach of each seat.",
    layer: "land",
    overlays: [
      "provinces", "provinceBorders", "states", "politicalInfluence",
      "settlements", "settlementNames", "hubNames",
    ],
    provinceOpacity: 0.55,
    labelTheme: "Modern Cartographic",
    requires: 7,
  },
  {
    id: "goods",
    name: "Goods & Trade",
    glyph: "⚖",
    blurb: "Trunks, corridors and the chokepoints that price them.",
    layer: "land",
    overlays: [
      "tradeRoutes", "tradeCorridors", "tradeRegions", "tradeBasins",
      "chokepoints", "settlements", "hubNames",
    ],
    labelTheme: "Modern Cartographic",
    requires: 8,
  },
  {
    id: "hazards",
    name: "Hazards",
    glyph: "⚠",
    blurb: "Every risk field at once — what the sea and the air can do to you.",
    layer: "land",
    overlays: [
      "sharkZones", "shipwormZones", "reefZones", "stormZones", "monsoonZones",
      "settlements",
    ],
    linePreset: "High contrast",
    requires: 8,
  },
];

/** Every overlay any plate touches. Applying a plate sets each of these
 *  explicitly — on if the plate lists it, off otherwise — so a plate is a
 *  COMPOSITION rather than an additive wash over whatever was already on.
 *
 *  Deriving it from the plates (rather than hand-listing it) is what keeps the
 *  blast radius honest: per-good overlays, campaign-only layers and anything else
 *  no plate mentions are never touched, so a plate can't silently clear work the
 *  user did in another panel. */
export const MANAGED_OVERLAYS: string[] = Array.from(
  new Set(MAP_THEMES.flatMap((t) => t.overlays)),
);

export function mapThemeById(id: string | null): MapTheme | undefined {
  return id ? MAP_THEMES.find((t) => t.id === id) : undefined;
}

/** The pipeline step each base layer's data comes from.
 *
 *  Selecting a layer whose phase hasn't run gives a blank or stale map with no
 *  explanation — the app's single most confusing first-run moment. Knowing the step
 *  lets the Toolbar dim the row and say what it is waiting for, which is the same
 *  thing QGIS's "filter legend by map content" does for the same reason. */
export const LAYER_REQUIRES: Record<ActiveLayer, number> = {
  land: 1,
  plates: 1,
  elevation: 2,
  terrain: 2,
  // Natural colour reads the BIOME column, so it waits for phase 6b like the
  // biomes plate does. It degrades to a continuous climate tint rather than to
  // blank if selected earlier (see `render::natural`), but the gate names the
  // step that makes it truthful.
  natural: 6,
  ridges: 2,
  shelf: 2,
  currents: 3,
  sst: 3,
  salinity: 3,
  temperature: 3,
  precipitation: 3,
  snow: 3,
  wind: 3,
  windspeed: 3,
  climate: 4,
  fisheries: 6,
  biomes: 6,
  soil: 6,
  fertility: 6,
  habitability: 7,
  storm: 8,
  reef: 8,
  shark: 8,
  shipworm: 8,
  disease: 8,
};

export function layerReady(layer: ActiveLayer, stepCompleted: Record<number, boolean>): boolean {
  return !!stepCompleted[LAYER_REQUIRES[layer] ?? 1];
}

/** Is this plate's data actually generated yet? */
export function themeReady(theme: MapTheme, stepCompleted: Record<number, boolean>): boolean {
  return !!stepCompleted[theme.requires];
}

/** Apply a plate. One `setState` for the whole UI composition (so the switch reads
 *  as a single change rather than a cascade), then the appearance stores for the
 *  typography and line palette the plate calls for. */
export function applyMapTheme(theme: MapTheme): void {
  const patch: Record<string, boolean> = {};
  for (const key of MANAGED_OVERLAYS) patch[key] = false;
  for (const key of theme.overlays) patch[key] = true;

  useUIStore.setState((s) => ({
    activeLayer: theme.layer,
    overlayVisibility: { ...s.overlayVisibility, ...patch },
    layerOpacity: theme.layerOpacity ?? 1,
    provinceOpacity: theme.provinceOpacity ?? s.provinceOpacity,
    activeMapTheme: theme.id,
    // Cleared to `null` (never left stale) when a plate doesn't name one, or
    // switching from "Relief & Height" to another elevation-styled plate would
    // silently carry the previous plate's style over.
    elevationStyle: theme.elevationStyle ?? null,
  }));

  const settings = useSettingsStore.getState();
  if (theme.labelTheme) settings.applyLabelTheme(theme.labelTheme);
  if (theme.linePreset && theme.linePreset in COLOR_PRESETS) {
    settings.applyPreset(theme.linePreset);
  }
}
