import type { RiverData, LakeData, Settlement, VectorSample, Streamline, TradeRoute, FisheryBank, SharkZone, GoodRegion, CultureRegion, TradeTrunk, TradeCorridor, PoliticalCenter, EconChokepoint, EconChain, EconRegion, EconCorridor, HouseBrief, MerchantRoute, FuturesLane, SpecCenter, CoinUseCity, ExpeditionView, ExpeditionFail, RidgeLine, StateRegion } from "@types";
import type { ClimateBands } from "@bridge";
import { GOOD_DEFS, goodOverlayKey, goodSubtypes, type SubtypeDef } from "@goods";
import { drawGoodIcon } from "./goodIcons";
import { latLineY } from "./projection";

/** Convex hull (Andrew's monotone chain) of a set of points — used to draw a
 *  merchant house's translucent "sphere of business" around its cities. Returns
 *  the hull vertices in order (fewer than 3 if the points are collinear). */
function convexHull(pts: [number, number][]): [number, number][] {
  const ps = pts.map((p) => [p[0], p[1]] as [number, number]).sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const uniq: [number, number][] = [];
  for (const p of ps) {
    const last = uniq[uniq.length - 1];
    if (!last || last[0] !== p[0] || last[1] !== p[1]) uniq.push(p);
  }
  if (uniq.length < 3) return uniq;
  const cross = (o: [number, number], a: [number, number], b: [number, number]) =>
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const lower: [number, number][] = [];
  for (const p of uniq) {
    while (lower.length >= 2 && cross(lower[lower.length - 2], lower[lower.length - 1], p) <= 0) lower.pop();
    lower.push(p);
  }
  const upper: [number, number][] = [];
  for (let i = uniq.length - 1; i >= 0; i--) {
    const p = uniq[i];
    while (upper.length >= 2 && cross(upper[upper.length - 2], upper[upper.length - 1], p) <= 0) upper.pop();
    upper.push(p);
  }
  lower.pop();
  upper.pop();
  return lower.concat(upper);
}

/** Per-cell abundance → one of 4 discrete quality tiers (1 negligible … 4 very
 *  high). The cell's fill opacity steps with the tier so richer deposits read as
 *  more solid and poor deposits as faint. */
const TIER_ALPHA = [0, 0.32, 0.55, 0.78, 1.0]; // index 1..4 (legacy; kept for reference)
void TIER_ALPHA;

/** Atlas 2.0 · rotating palette for named trade basins (hull + label tints). */
const BASIN_COLORS = [
  "#4fd0c0", "#d8b24a", "#c08cff", "#7fd08a", "#e08a6a", "#6aa9e8",
  "#e0a0d0", "#c9c96a", "#8ad0e0", "#d88a8a", "#a9c96a", "#c0a06a",
];

/** Parse "#rrggbb" or "rgb(r,g,b)" → [r,g,b], or null. */
function parseColor(c: string): [number, number, number] | null {
  const h = /^#?([0-9a-f]{6})$/i.exec(c);
  if (h) { const n = parseInt(h[1], 16); return [(n >> 16) & 255, (n >> 8) & 255, n & 255]; }
  const m = /rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/i.exec(c);
  if (m) return [+m[1], +m[2], +m[3]];
  return null;
}
/** Terroir ramp: blend a colour toward a pale tint at low quality `t`, full
 *  saturation at high `t` — the wine-region heatmap gradient. */
function rampToward(color: string, t: number): string {
  const rgb = parseColor(color);
  if (!rgb) return color;
  const pale = 0.72 * (1 - t); // how far toward pale (toward white) at low quality
  const r = Math.round(rgb[0] + (236 - rgb[0]) * pale);
  const g = Math.round(rgb[1] + (230 - rgb[1]) * pale);
  const b = Math.round(rgb[2] + (224 - rgb[2]) * pale);
  return `rgb(${r},${g},${b})`;
}

const GOOD_BY_NAME = new Map(GOOD_DEFS.map((g) => [g.name, g]));
const SHARK_COLOR = "#e04040";
const SHIPWORM_COLOR = "#b98a4a";
const STORM_COLOR = "#c050d0";
const MONSOON_COLOR = "#3a9ad0";
const REEF_COLOR = "#30c0b0";
const POLITICAL_COLOR = "#d65fd0"; // trade-hub marker (magenta) — legacy
const STAR_COLOR = "#ffd24a"; // power-tier stars on major hubs (gold) — legacy
const HUB_BLUE = "#3a86d6"; // trade-hub circle

const RIVER_COLOR = "#2288cc";
/** River shade: light blue for ordinary streams, a deeper blue once a river has
 *  grown into a MAJOR trunk (flagged by length in rivers.rs). Two flat shades —
 *  the colour change is the cue that "this is now a major river". */
function riverShade(major: boolean | undefined): string {
  return major ? "rgb(34,96,165)" : "rgb(120,190,225)";
}

/** Stroke a cell path as a smooth Catmull-Rom spline (converted to cubic Béziers)
 *  so rivers read as natural meanders instead of the 8-neighbour grid staircase.
 *  Falls back to straight segments for 2-point paths. Points are cell coords;
 *  +0.5 centres the line in the cell. */
function strokeSmoothPath(ctx: CanvasRenderingContext2D, pts: [number, number][]) {
  const n = pts.length;
  ctx.beginPath();
  ctx.moveTo(pts[0][0] + 0.5, pts[0][1] + 0.5);
  if (n === 2) {
    ctx.lineTo(pts[1][0] + 0.5, pts[1][1] + 0.5);
    ctx.stroke();
    return;
  }
  for (let i = 0; i < n - 1; i++) {
    const p0 = pts[i === 0 ? 0 : i - 1];
    const p1 = pts[i];
    const p2 = pts[i + 1];
    const p3 = pts[i + 2 < n ? i + 2 : n - 1];
    // Catmull-Rom → cubic Bézier control points (tension 1/6).
    const c1x = p1[0] + (p2[0] - p0[0]) / 6;
    const c1y = p1[1] + (p2[1] - p0[1]) / 6;
    const c2x = p2[0] - (p3[0] - p1[0]) / 6;
    const c2y = p2[1] - (p3[1] - p1[1]) / 6;
    ctx.bezierCurveTo(c1x + 0.5, c1y + 0.5, c2x + 0.5, c2y + 0.5, p2[0] + 0.5, p2[1] + 0.5);
  }
  ctx.stroke();
}
/** Displace a cell path with gentle, deterministic MEANDERS so rivers read as
 *  natural winding channels rather than the straight diagonal grid-lines the
 *  steepest-descent drainage produces on flats. Endpoints are anchored (the
 *  envelope tapers to 0 at both ends) so mouths and confluences stay put, and a
 *  seam guard skips the ±worldW jump so wrap-around rivers don't spike. Amplitude
 *  and wavelength grow with stream order (a great river swings in broad bends, a
 *  creek in tight ones). Cheap O(n); the result is cached per points array. */
function meanderPath(pts: [number, number][], seed: number, order: number, worldW: number, scale: number): [number, number][] {
  const n = pts.length;
  if (n < 4 || scale <= 0.02) return pts;
  // Gradient-gated amplitude: `scale` (from rivers.rs, 0 steep → 1 flat) collapses
  // the meander toward zero on steep headwater/alpine reaches so the channel hugs
  // the fall line and only wanders on true lowland floodplains.
  const amp = Math.min(1.2, 0.45 + order * 0.13) * scale;
  const wav = 6 + order * 2.4; // cells per meander cycle
  const k = (Math.PI * 2) / wav;
  let ph = Math.sin(seed * 12.9898) * 43758.5453;
  ph = (ph - Math.floor(ph)) * Math.PI * 2;
  const half = worldW > 0 ? worldW / 2 : Infinity;
  const out: [number, number][] = new Array(n);
  out[0] = pts[0];
  out[n - 1] = pts[n - 1];
  let cum = 0;
  for (let i = 1; i < n - 1; i++) {
    const [px, py] = pts[i - 1];
    const [x, y] = pts[i];
    const [nx, ny] = pts[i + 1];
    // Seam / gap guard: leave points near a wrap seam (or a long jump) untouched.
    if (Math.abs(nx - px) > half || Math.abs(x - px) > half || Math.abs(nx - x) > half) {
      out[i] = pts[i];
      continue;
    }
    cum += Math.hypot(x - px, y - py);
    let tx = nx - px, ty = ny - py;
    const len = Math.hypot(tx, ty) || 1;
    tx /= len; ty /= len;
    const f = i / (n - 1);
    const env = Math.sin(Math.PI * f); // 0 at ends, 1 mid-course
    const off = amp * env * Math.sin(ph + cum * k);
    out[i] = [x - ty * off, y + tx * off]; // perpendicular to the local tangent
  }
  return out;
}
// Freshwater lake — a brighter, more saturated cyan-blue at higher opacity so
// open water reads as clean, glinting water rather than a pale wash of the land
// showing through (the "make lakes shinier" note).
const LAKE_COLOR = "rgba(58, 176, 238, 0.86)";
/** Oxbow / backwater lake — a stiller, weedier green-blue than open lake water. */
const OXBOW_COLOR = "rgba(64, 150, 150, 0.72)";
/** Lake fill by character: oxbow backwater, then a brine ramp toward the vivid
 *  pink of a hypersaline lake (halophile / Dunaliella bloom — Lake Retba, the
 *  Great Salt Lake north arm), else open blue water. */
/** Deterministic province tint keyed by CULTURE, so provinces of one people read
 *  as a colour family (the EU4-style political map). */
function cultureColor(culture: string): string {
  let h = 2166136261 >>> 0;
  const s = culture || "—";
  for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619) >>> 0; }
  const hue = h % 360;
  const sat = 45 + (h >>> 9) % 25;   // 45–70%
  const lig = 42 + (h >>> 17) % 14;  // 42–56%
  return `hsl(${hue}, ${sat}%, ${lig}%)`;
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  s /= 100; l /= 100;
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return [Math.round(f(0) * 255), Math.round(f(8) * 255), Math.round(f(4) * 255)];
}

/** A DISTINCT random tint per province id (not per culture) so every province reads
 *  as its own colour and adjacent ones never blend into one blob. Deterministic +
 *  well-spread via an integer hash → the hue wheel. Returns 0–255 RGB (for ImageData). */
function provinceColorRGB(id: number): [number, number, number] {
  let h = (id + 1) >>> 0;
  h = Math.imul(h ^ (h >>> 16), 2246822519) >>> 0;
  h = Math.imul(h ^ (h >>> 13), 3266489917) >>> 0;
  h = (h ^ (h >>> 16)) >>> 0;
  const hue = h % 360;
  const sat = 55 + (h >>> 9) % 30;   // 55–85% — vivid so neighbours separate
  const lig = 46 + (h >>> 17) % 16;  // 46–62%
  return hslToRgb(hue, sat, lig);
}

/** #rgb / #rrggbb → 0–255 RGB triple (for the single-colour province fill). */
function hexToRgb(hex: string): [number, number, number] {
  const m = hex.replace("#", "");
  const s = m.length === 3 ? m.split("").map((c) => c + c).join("") : m;
  const n = parseInt(s, 16) >>> 0;
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function lakeFill(lake: LakeData): string {
  if (lake.kind === 1) return OXBOW_COLOR;
  const s = lake.salinity_ppt ?? 0;
  if (lake.endorheic || s >= 1) {
    if (s >= 120) return "rgba(216, 90, 134, 0.78)";  // hypersaline — vivid pink
    if (s >= 35) return "rgba(201, 122, 150, 0.74)";  // saline — pale rose
    return "rgba(70, 150, 158, 0.72)";                // brackish — steely teal
  }
  return LAKE_COLOR;
}

const SETTLEMENT_COLORS: Record<string, string> = {
  capital: "#ffd700",
  city: "#ff8844",
  town: "#e8e8e8",
  village: "#e6cf9a", // warm cream — reads on both green land and blue sea
  outpost: "#c9a96a", // tan — visible (was near-black #111111, invisible on terrain)
};

const SETTLEMENT_SIZES: Record<string, number> = {
  capital: 3,
  city: 2.2,
  town: 1.6,
  village: 1,
  outpost: 0.8, // small
};

/** Satellite city marker (dependent port/granary/workshop town of a metropolis). */
const SATELLITE_COLOR = "#e0503a";

const WARM_CURRENT = "#ee5533";
const COLD_CURRENT = "#3399ee";
const NEUTRAL_CURRENT = "#9bb0c0";
const WIND_COLOR = "#aaccee";
const LAT_LINE_COLOR = "#cccc66";
// Climate-band overlay colours: ITCZ rain line (cyan), subtropical-high dry belts
// (amber), polar-front storm tracks (blue).
const ITCZ_COLOR = "#39d6e0";
const SUBTROPICAL_COLOR = "#e0a83a";
const POLAR_FRONT_COLOR = "#6a9cf0";
const FISHERY_BANK = "#39d3c0"; // grand-bank fishing ground (teal)

/** Adjustable trade/connection-line colours — seeded with the historical defaults
 *  and overridden live from the in-game Settings panel (`settingsStore` calls
 *  `setLineColors`). Every trade/settlement-connection renderer reads from here so
 *  the whole palette is user-configurable and saveable with the world. */
export const LINE_COLOR_DEFAULTS = {
  tradeTrunk: "#e0c060",      // major bundled commodity trunk (amber)
  tradeTrunkMinor: "#b8a878", // minor/low-volume trunk (muted amber)
  dynamicFlow: "#4fd0c0",     // live yearly-averaged trade flow (teal)
  tradeLand: "#caa15a",       // overland caravan route (tan)
  tradeSea: "#7fd0d8",        // maritime route (pale cyan)
  tradeRiver: "#9fe07a",      // river-following inland route (green)
  corridor: "#5fc8a8",        // live econ trade corridor (sea-green)
  corridorArrow: "#7fe0c0",   // corridor arrowhead
  merchantIn: "#5fd0ff",      // merchant route inbound (cyan)
  merchantOut: "#ffce5f",     // merchant route outbound (gold)
  manufactory: "#4fc06a",     // house/guild manufactory holding line (green)
  estate: "#ffe14a",          // estate holding line (shiny yellow)
  settlementColony: "#c08cff", // settlement colony pin + lane (violet/purple)
  houseOutpost: "#9aa7b4",    // house trade outpost marker (grey; frame uses owner colour)
  colonyLane: "#c08cff",      // colony↔metropolis supply/monopoly lane (violet)
};
/** A colony/outpost map marker (settlement colony or house trade outpost). */
export interface ColonyMarker {
  x: number; y: number; name: string;
  kind: number;        // 1 = settlement colony · 2 = house outpost
  stage: number;       // colony_stage 1..4 (settlement only)
  ownerColor: string;  // owner-house colour (outpost frame / lane tint)
  founderX: number; founderY: number; // metropolis/owner-home (lane endpoint); <0 = none
}

export type LineColorKey = keyof typeof LINE_COLOR_DEFAULTS;
export const lineColors: Record<LineColorKey, string> = { ...LINE_COLOR_DEFAULTS };
/** Apply a partial palette override (from the Settings store). */
export function setLineColors(partial: Partial<Record<LineColorKey, string>>) {
  Object.assign(lineColors, partial);
}

// ── Map label typography ──────────────────────────────────────────────────────
// Every place name used to be styled at its own call site, so provinces and
// settlements came out in an IDENTICAL font and colour — the two classes you most
// need to tell apart — while rivers, lakes and peaks differed only by tint. This
// registry gives each class one styled identity, following the cartographic
// convention that a label's FACE tells you what kind of thing it names before you
// read it: nature is serif and leans, human works are sans and stand upright.
//
// Same shape as `lineColors` above, so the Settings store drives both identically.

/** System-font stacks: Windows → macOS → Linux, no bundled font files. */
export const LABEL_FONTS = {
  serifNature: `"Palatino Linotype", Palatino, "Book Antiqua", "URW Palladio L", Georgia, serif`,
  sansHuman: `Candara, Optima, "Gill Sans MT", Carlito, "Segoe UI", system-ui, sans-serif`,
  engraved: `"Copperplate Gothic Light", Copperplate, "Copperplate Gothic", "Trajan Pro", Georgia, serif`,
  garamond: `Garamond, "EB Garamond", "Adobe Garamond Pro", "Book Antiqua", serif`,
} as const;
export type LabelFontKey = keyof typeof LABEL_FONTS;
/** Friendly names for the Settings font dropdown. */
export const LABEL_FONT_LABELS: Record<LabelFontKey, string> = {
  serifNature: "Palatino (serif)",
  sansHuman: "Candara (humanist sans)",
  engraved: "Copperplate (engraved)",
  garamond: "Garamond (old-style serif)",
};

export interface LabelStyle {
  /** CSS font-family stack (one of `LABEL_FONTS`, or anything else). */
  family: string;
  weight: number;    // 400 · 600 · 700
  italic: boolean;
  caps: boolean;     // render the name uppercased
  /** Letter-spacing in em. Drawn MANUALLY (see `drawLabel`) — `ctx.letterSpacing`
   *  is Chromium-only in practice and Tauri runs WebKit on Linux and macOS. */
  tracking: number;
  color: string;
  /** Multiplier on the class's existing base size (1 = exactly as before). */
  size: number;
}

export type LabelKey =
  | "province" | "settlement"
  | "river" | "lake" | "mountain" | "desert" | "forest" | "tundra"
  | "cultureRegion" | "peopleTerritory" | "tradeBasin" | "state";

const F = LABEL_FONTS;
/** The shipped baseline — the "Mixed Contrast" theme. */
export const LABEL_STYLE_DEFAULTS: Record<LabelKey, LabelStyle> = {
  // ── Human works: humanist sans, upright ──
  province:        { family: F.sansHuman,  weight: 600, italic: false, caps: true,  tracking: 0.18, color: "#e8dcc0", size: 1 },
  settlement:      { family: F.sansHuman,  weight: 600, italic: false, caps: false, tracking: 0,    color: "#f2f6fb", size: 1 },
  // ── Water: serif, leaning ──
  river:           { family: F.serifNature, weight: 400, italic: true,  caps: false, tracking: 0.04, color: "#7fc8e0", size: 1 },
  lake:            { family: F.serifNature, weight: 400, italic: true,  caps: false, tracking: 0.04, color: "#9ad0e8", size: 1 },
  // ── Land: serif, upright ──
  mountain:        { family: F.serifNature, weight: 400, italic: false, caps: false, tracking: 0.02, color: "#d8a878", size: 1 },
  desert:          { family: F.serifNature, weight: 700, italic: false, caps: true,  tracking: 0.22, color: "#d8b878", size: 1 },
  forest:          { family: F.serifNature, weight: 700, italic: false, caps: true,  tracking: 0.22, color: "#8fc088", size: 1 },
  tundra:          { family: F.serifNature, weight: 700, italic: false, caps: true,  tracking: 0.22, color: "#a8c0cc", size: 1 },
  // ── People: sans, widely tracked caps (a people is not a place you can stand in) ──
  cultureRegion:   { family: F.sansHuman,  weight: 700, italic: false, caps: true,  tracking: 0.30, color: "#caa6e0", size: 1 },
  peopleTerritory: { family: F.sansHuman,  weight: 600, italic: false, caps: true,  tracking: 0.26, color: "#d8c2ea", size: 1 },
  tradeBasin:      { family: F.serifNature, weight: 600, italic: true,  caps: false, tracking: 0.06, color: "#8fd0d8", size: 1 },
  // ── Political: a sovereignty label — bold, gold, tracked wider than a mere
  // ── settlement, since a state is a claim over land, not a place on it.
  state:           { family: F.sansHuman,  weight: 800, italic: false, caps: true,  tracking: 0.24, color: "#e0c878", size: 1.05 },
};

/** Coordinated theme presets — sparse overrides on the defaults, exactly like
 *  `COLOR_PRESETS` in the settings store. */
export const LABEL_THEMES: Record<string, Partial<Record<LabelKey, Partial<LabelStyle>>>> = {
  // Nature serif/italic, human works sans. The default; no overrides needed.
  "Mixed Contrast": {},
  // One serif throughout — differentiated by case, tracking and slant alone.
  "Classic Atlas": {
    province: { family: F.serifNature }, settlement: { family: F.serifNature },
    cultureRegion: { family: F.serifNature }, peopleTerritory: { family: F.serifNature },
  },
  // Copperplate for everything administrative/areal over a Garamond body.
  "Engraved Antique": {
    province: { family: F.engraved, tracking: 0.24 },
    settlement: { family: F.engraved, caps: true, tracking: 0.10, size: 0.92 },
    river: { family: F.garamond }, lake: { family: F.garamond },
    mountain: { family: F.engraved, caps: true, tracking: 0.12 },
    desert: { family: F.engraved }, forest: { family: F.engraved }, tundra: { family: F.engraved },
    cultureRegion: { family: F.engraved, tracking: 0.36 },
    peopleTerritory: { family: F.engraved, tracking: 0.30 },
    tradeBasin: { family: F.garamond },
    state: { family: F.engraved, tracking: 0.30 },
  },
  // Humanist sans throughout — the Ordnance-Survey register, cleanest zoomed out.
  "Modern Cartographic": {
    river: { family: F.sansHuman }, lake: { family: F.sansHuman },
    mountain: { family: F.sansHuman }, desert: { family: F.sansHuman },
    forest: { family: F.sansHuman }, tundra: { family: F.sansHuman },
    tradeBasin: { family: F.sansHuman },
  },
};

/** Live registry the renderer reads every frame. */
export const labelStyles: Record<LabelKey, LabelStyle> =
  Object.fromEntries(
    (Object.keys(LABEL_STYLE_DEFAULTS) as LabelKey[]).map((k) => [k, { ...LABEL_STYLE_DEFAULTS[k] }]),
  ) as Record<LabelKey, LabelStyle>;

/** A colour with an alpha applied. `OverlayManager.rgba` only understands `hsl()`
 *  and returns a hex untouched (so an alpha would be silently dropped) — the label
 *  colours are all hex, so append the 2-digit alpha channel the way the rest of this
 *  file does (`${col}1c`, `${col}66`). */
export function labelAlpha(col: string, a: number): string {
  if (col.startsWith("hsl(")) return col.replace("hsl(", "hsla(").replace(")", `,${a.toFixed(3)})`);
  if (/^#[0-9a-f]{6}$/i.test(col)) {
    return col + Math.round(Math.max(0, Math.min(1, a)) * 255).toString(16).padStart(2, "0");
  }
  return col;
}

/** Apply a partial typography override (from the Settings store). */
export function setLabelStyles(partial: Partial<Record<LabelKey, Partial<LabelStyle>>>) {
  for (const k of Object.keys(partial) as LabelKey[]) {
    if (labelStyles[k]) Object.assign(labelStyles[k], partial[k]);
  }
}

/** Merge a theme's sparse overrides onto the defaults into a full style set. */
export function resolveLabelTheme(
  theme: Partial<Record<LabelKey, Partial<LabelStyle>>>,
): Record<LabelKey, LabelStyle> {
  return Object.fromEntries(
    (Object.keys(LABEL_STYLE_DEFAULTS) as LabelKey[])
      .map((k) => [k, { ...LABEL_STYLE_DEFAULTS[k], ...(theme[k] ?? {}) }]),
  ) as Record<LabelKey, LabelStyle>;
}

export class OverlayManager {
  private rivers: RiverData[] = [];
  private lakes: LakeData[] = [];
  private settlements: Settlement[] = [];
  private colonies: ColonyMarker[] = [];
  /** Atlas 2.0 · per-hub yearly trade throughput for the Trade Heat overlay. */
  private heatPoints: { x: number; y: number; v: number }[] = [];
  /** Atlas 2.0 · named trade basins (member positions hulled + labelled). */
  private basins: { name: string; pts: [number, number][]; cx: number; cy: number }[] = [];
  private provinceRaster: { data: ArrayLike<number>; w: number; h: number; gridW: number; gridH: number } | null = null;
  /** Prebuilt full-res fill (composited ONCE → no cell grid) + border segments, so the
   *  per-frame cost is just one blit + one stroke regardless of world size. */
  private provinceCanvas: HTMLCanvasElement | null = null;
  private provinceBorderPath: Path2D | null = null;
  /** Fill style: "distinct" = each province its own colour (default); "single" = every
   *  province filled with `provinceSingleColor`, leaving only the (custom-coloured)
   *  borders to read the partition. Driven by the Settings panel. */
  private provinceFillMode: "distinct" | "single" = "distinct";
  private provinceSingleColor = "#3a5a7c";
  private provinceBorderColor = "rgba(8, 14, 20, 0.7)";
  /** Province ids present in the current raster, so a live style change can recolour
   *  the fill without re-fetching the partition. */
  private provinceIdList: number[] = [];
  /** The province picked on the map: its outline + seat, rebuilt only when the id
   *  changes so per-frame cost stays O(1) however large the world is. */
  private selectedProvince: number | null = null;
  private selectedProvincePath: Path2D | null = null;
  private selectedProvinceSeat: { x: number; y: number } | null = null;
  /** Provinces MARKED for a batch merge/split (shift-click). Their combined outline is
   *  drawn in a distinct colour so the "affect only these" set is visible on the map. */
  private markedProvinces: Set<number> = new Set();
  private markedProvincePath: Path2D | null = null;
  /** Province name + label anchor (world cells) + inscribed radius for on-map labels. */
  private provinceLabels: { x: number; y: number; r: number; name: string }[] = [];
  /** Opacity of the province colour FILL only — borders, names and the selection
   *  outline are always drawn at full strength so the political map stays legible
   *  however far the fill is faded back. */
  private provinceOpacity = 0.5;
  /** Province id → seat cell, for marking the selected province's seat. */
  private provinceSeats: Map<number, { x: number; y: number }> = new Map();
  /** Atlas 2.0 · refugee roads (age01 = 0 fresh … 1 faded out). */
  private migrations: { fx: number; fy: number; tx: number; ty: number; age: number }[] = [];
  /** Settlement (cell coords) currently under the cursor — drawn as a shiny ring. */
  private hoverPoint: { x: number; y: number } | null = null;
  setHoverPoint(p: { x: number; y: number } | null) { this.hoverPoint = p; }
  /** #1/#23 · per-hub share of the ISOLATED culture (halo + inner fill overlay). */
  private cultureShares: { x: number; y: number; share: number }[] = [];
  private cultureColor: [number, number, number] = [200, 120, 220];
  setCultureShares(pts: { x: number; y: number; share: number }[], color: [number, number, number]) {
    this.cultureShares = pts; this.cultureColor = color;
  }
  /** Colony/satellite ↔ metropolis link to shine (a=metropolis, b=colony). */
  private colonyLink: { ax: number; ay: number; bx: number; by: number } | null = null;
  setColonyLink(l: { ax: number; ay: number; bx: number; by: number } | null) { this.colonyLink = l; }
  private windData: { samples: VectorSample[]; gridW: number; gridH: number } | null = null;
  private currentLines: Streamline[] = [];
  private tradeRoutes: TradeRoute[] = [];
  /** #23 · the single highlighted itinerary route (world cells), or empty. */
  private travelRoute: [number, number][] = [];
  /** Ridge-drawing tool: transient drawn/in-progress ridge lines to sketch on the map. */
  private ridgeSketch: RidgeLine[] = [];
  /** 🌊 Hydrology · indices (into `rivers`) of the selected system's subtree to
   *  glow on the map; empty = no selection (all rivers drawn normally). */
  private riverHighlight: Set<number> = new Set();
  /** 🌊 Hydrology · per-river-index glow colour (branch / order scheme); missing
   *  entries fall back to the default cyan. */
  private riverHighlightColors: Record<number, string> = {};
  /** 🌊 Hydrology · index (into `lakes`) of the selected lake to glow; -1 = none.
   *  When set, every other lake dims so the chosen basin stands out. */
  private lakeHighlight = -1;
  setLakeHighlight(idx: number | null) { this.lakeHighlight = idx ?? -1; }
  /** #37 · per-hub local price premium for the selected good (1 = par with the
   *  world base value; <1 cheap/abundant, >1 dear/scarce). */
  private goodScarcity: { x: number; y: number; premium: number }[] = [];
  /** #26 · named geographic features (rivers/mountains/lakes/regions). */
  private toponyms: { kind: string; name: string; x: number; y: number }[] = [];
  /** 🌊 Reach-break markers on trunk rivers (upper→middle→delta transitions). */
  private riverBreaks: { x: number; y: number; tx: number; ty: number; label: string }[] = [];
  private coinUse: CoinUseCity[] = [];
  private coinOverlayHub: number | null = null;
  /** Bank seats to mark on the map (set empty to hide). */
  private bankIcons: { x: number; y: number; name: string; defunct: boolean; color: string }[] = [];
  /** Phase 6 · plague-struck cities + contagion routes (source→city, directional). */
  private plagueCities: { x: number; y: number; active: boolean; deaths: number; origin: boolean }[] = [];
  private plagueEdges: { ax: number; ay: number; bx: number; by: number }[] = [];
  /** Phase 6 · guild cities to mark with their good's emoji (+ a brand label for
   *  exceptional crafts). */
  private guildCities: { x: number; y: number; emoji: string; label: string }[] = [];
  /** Phase 6 · living notable figures + landmarks, as emoji map markers. */
  private figureMarks: { x: number; y: number; emoji: string }[] = [];
  private landmarkMarks: { x: number; y: number; emoji: string }[] = [];
  /** Phase 7 · dynasty ties between house seat cities (ally = gold, feud = red). */
  private dynastyLinks: { ax: number; ay: number; bx: number; by: number; ally: boolean }[] = [];
  private fisheryBanks: FisheryBank[] = [];
  private sharkZones: SharkZone[] = [];
  private shipwormZones: SharkZone[] = [];
  private stormZones: SharkZone[] = [];
  private monsoonZones: SharkZone[] = [];
  private reefZones: SharkZone[] = [];
  private goodRegions: GoodRegion[] = [];
  private cultureRegions: CultureRegion[] = [];
  private stateRegions: StateRegion[] = [];
  /** States are rendered EXACTLY like provinces: a prebuilt raster-resolution fill
   *  canvas (each province cell tinted its state's colour) + a border path traced
   *  along raster cell edges. Rebuilt only when the states or the raster change. */
  private stateCanvas: HTMLCanvasElement | null = null;
  private stateBorderPath: Path2D | null = null;
  private statesDirty = false;
  /** Transient highlight pin (searched settlement) in world coords. */
  private searchPin: { wx: number; wy: number } | null = null;
  /** War highlight: attacker (red) + defender (blue) seats, lit from the War panel. */
  private warHighlight: { ax: number; ay: number; bx: number; by: number } | null = null;
  /** Per-good display metadata (icon/color) from the active editable spec; falls
   *  back to the static GOOD_DEFS when absent. */
  private goodMeta: Map<string, { icon: string; color: string }> | null = null;
  private tradeTrunks: TradeTrunk[] = [];
  private dynamicTrunks: TradeTrunk[] = [];
  private tradeCorridorList: TradeCorridor[] = [];
  private expeditions: ExpeditionView[] = [];
  private expeditionFails: ExpeditionFail[] = [];
  private merchantRoutes: MerchantRoute[] = [];
  private futuresLanes: FuturesLane[] = [];
  private selectedFuturesLane: FuturesLane | null = null;
  private futuresFocus: { city?: string; holder?: string; good?: string } | null = null;
  private politicalCenters: PoliticalCenter[] = [];
  private specCenters: SpecCenter[] = [];
  private houses: HouseBrief[] = [];
  private allHouses: HouseBrief[] = [];
  private selectedHouseIdx: number | null = null;
  /** Seat→city polylines for the focused house, snapped onto the EXISTING trade
   *  routes (`routeAlongTradeRoutes`). Empty inner paths are skipped. */
  private houseNetwork: [number, number][][] = [];
  // ── Trade-route graph (lazily built from `this.tradeRoutes`) so house-network and
  //    futures lanes can ride the roads ALREADY drawn on the map instead of straight
  //    or independently-routed lines. Rebuilt only when the routes array changes. ──
  private tgRef: TradeRoute[] | null = null;     // identity of the built graph's source
  private tgNodes: [number, number][] = [];      // settlement junctions (route endpoints)
  private tgAdj: { to: number; pts: [number, number][]; len: number }[][] = [];
  private chokepoints: EconChokepoint[] = [];
  private corridors: EconCorridor[] = [];
  private econRegions: EconRegion[] = [];
  private supplyChain: EconChain | null = null;
  private supplyChainImport = false; // true = inbound import (blue) vs export (red)
  /** Adjustable trade-hub marker display (size multiplier + highlight intensity). */
  private hubSize = 1;
  private hubIntensity = 1;
  /** Per-good reach: chains carrying the selected good + the hubs it reaches. */
  private reachChains: EconChain[] = [];
  private reachHubs: [number, number][] = [];
  private latLinesData: { gridW: number; gridH: number; equatorOffset: number; latScale: number; lineRatio: number } | null = null;
  /** Circulation bands (ITCZ line + subtropical-high / polar-front belts). */
  private climateBands: ClimateBands | null = null;

  private visibility: Record<string, boolean> = {
    rivers: true, lakes: true, settlements: true,
    markers: false, wind: false, currents: false, latLines: false,
    itcz: false, windBelts: false,
    tradeRoutes: false, fisheryBanks: false,
    sharkZones: false, shipwormZones: false, stormZones: false, monsoonZones: false, reefZones: false, tradeFlows: false,
    politicalInfluence: false, chokepoints: false, tradeCorridors: false, campaignCorridors: false, expeditions: false,
    speculation: false, coinDominance: false,
    houseControl: false, merchantRoutes: false, futures: false,
    hubNames: false, settlementNames: false, tradeRegions: false, cultures: false,
    riverBreaks: true,
  };

  private currentScale = 1;
  private worldW = 0;
  /** Cached region-mask boundary edges, keyed by the cell array of each region/
   *  zone (replaced wholesale on each data fetch, so the WeakMap auto-evicts).
   *  Each entry is a flat [x1,y1,x2,y2, …] list — built once, not per frame. */
  private edgeCache = new WeakMap<object, number[]>();
  /** Cached subtype-split edges, keyed by each region's `subtypes` array. */
  private subtypeEdgeCache = new WeakMap<object, number[]>();
  /** Cached meandered river paths, keyed by each river's raw `points` array
   *  (rebuilt only when the rivers data is replaced). */
  private meanderCache = new WeakMap<object, [number, number][]>();
  /** Cached nearest-river lookup for atlas river labels, keyed by "x,y" of the
   *  toponym; invalidated whenever rivers or toponyms are replaced. */
  private riverLabelCache = new Map<string, { river: RiverData; bi: number } | null>();
  /** Placed label bounding boxes for the current frame (world/cell coords), used
   *  to skip overlapping settlement + toponym labels ("Rusapolyelgorod" merges).
   *  Cleared at the top of every `render`. */
  private placedLabels: { x0: number; y0: number; x1: number; y1: number }[] = [];

  drawRivers(rivers: RiverData[]) { this.rivers = rivers; this.riverLabelCache.clear(); }
  drawLakes(lakes: LakeData[]) { this.lakes = lakes; }
  drawSettlements(settlements: Settlement[]) { this.settlements = settlements; }
  drawColonies(colonies: ColonyMarker[]) { this.colonies = colonies; }
  /** Atlas 2.0 · set the Trade Heat points (hub position + yearly throughput). */
  drawTradeHeat(pts: { x: number; y: number; v: number }[]) { this.heatPoints = pts; }
  /** Atlas 2.0 · set the named trade basins. */
  drawTradeBasins(b: { name: string; pts: [number, number][]; cx: number; cy: number }[]) { this.basins = b; }

  /** Province partition overlay: a downsampled per-cell province-id raster + a
   *  per-province fill colour (keyed by culture) for the political/goods map. */
  updateProvinces(
    raster: { data: ArrayLike<number>; w: number; h: number; gridW: number; gridH: number } | null,
    provinces: {
      id: number; culture: string; name?: string; seat_x?: number; seat_y?: number;
      label_x?: number; label_y?: number; label_r?: number; cells?: number;
    }[],
  ) {
    this.provinceRaster = raster;
    this.provinceIdList = provinces.map((p) => p.id);
    const labels: { x: number; y: number; r: number; name: string }[] = [];
    for (const p of provinces) {
      if (p.name) {
        // Anchor on the pole of inaccessibility when the world carries one — it is
        // always INSIDE the province, unlike a centroid, and unlike the seat (a city,
        // often near an edge). Older worlds fall back to the seat with a radius
        // estimated from the area, so their names still place sensibly.
        const hasAnchor = p.label_x !== undefined && p.label_y !== undefined;
        const x = hasAnchor ? p.label_x! : p.seat_x;
        const y = hasAnchor ? p.label_y! : p.seat_y;
        if (x === undefined || y === undefined) continue;
        const r = p.label_r ?? (p.cells ? Math.sqrt(p.cells / Math.PI) * 0.6 : 4);
        labels.push({ x, y, r: Math.max(0.5, r), name: p.name });
      }
    }
    this.provinceLabels = labels;
    this.provinceSeats = new Map(
      provinces.filter((p) => p.seat_x !== undefined && p.seat_y !== undefined)
        .map((p) => [p.id, { x: p.seat_x!, y: p.seat_y! }]),
    );
    void cultureColor; // retained for the culture political map elsewhere
    this.rebuildProvinceColors();
    // The raster may have been replaced (regenerate / world open) — re-cut the
    // selected outline against it rather than leaving a stale path on screen.
    if (this.selectedProvince !== null) this.buildSelectedProvince();
    if (this.markedProvinces.size > 0) this.buildMarkedProvinces();
    // States are rendered on this raster, so a new raster invalidates them too.
    this.statesDirty = true;
  }

  /** Rebuild the per-province fill colours from the current fill mode, then re-raster.
   *  Cheap enough to call live from the Settings panel (no partition re-fetch). */
  private rebuildProvinceColors() {
    if (!this.provinceRaster) return;
    const rgb: [number, number, number][] = [];
    const single = hexToRgb(this.provinceSingleColor);
    for (const id of this.provinceIdList) {
      rgb[id] = this.provinceFillMode === "single" ? single : provinceColorRGB(id);
    }
    this.buildProvinceRender(rgb);
  }

  /** Province appearance from the Settings panel: a single flat fill (borders carry the
   *  partition) vs each province its own colour, plus a custom border colour. */
  setProvinceStyle(s: { fillMode?: "distinct" | "single"; singleColor?: string; borderColor?: string }) {
    if (s.fillMode !== undefined) this.provinceFillMode = s.fillMode;
    if (s.singleColor !== undefined) this.provinceSingleColor = s.singleColor;
    if (s.borderColor !== undefined) this.provinceBorderColor = s.borderColor;
    this.rebuildProvinceColors();
  }

  /** Opacity of the province colour fill, 0..1. Borders/names/selection are unaffected. */
  setProvinceOpacity(v: number) { this.provinceOpacity = Math.max(0, Math.min(1, v)); }

  /** Highlight ONE province (the map click / list selection). `null` clears it. */
  setSelectedProvince(id: number | null) {
    if (this.selectedProvince === id) return;
    this.selectedProvince = id;
    this.buildSelectedProvince();
  }

  /** Which province owns a world cell, or null on sea / no partition. The raster is
   *  full-resolution, so this is a single array read — cheap enough for hit-testing
   *  on every click with no IPC round-trip. */
  provinceAt(wx: number, wy: number): number | null {
    const r = this.provinceRaster;
    if (!r) return null;
    const rx = Math.floor((wx / r.gridW) * r.w);
    const ry = Math.floor((wy / r.gridH) * r.h);
    if (rx < 0 || ry < 0 || rx >= r.w || ry >= r.h) return null;
    const id = r.data[ry * r.w + rx];
    return id === 0xffffffff ? null : id;
  }

  /** Cut the selected province's outline out of the raster once, in world cells. */
  private buildSelectedProvince() {
    this.selectedProvincePath = null;
    this.selectedProvinceSeat = null;
    const r = this.provinceRaster;
    const id = this.selectedProvince;
    if (!r || id === null) return;
    const { data, w, h, gridW, gridH } = r;
    const sx = gridW / w, sy = gridH / h;
    const path = new Path2D();
    // An edge wherever the province meets ANYTHING else — including sea, so the
    // highlight traces the full coastline of an island province too.
    for (let ry = 0; ry < h; ry++) {
      for (let rx = 0; rx < w; rx++) {
        if (data[ry * w + rx] !== id) continue;
        const x0 = rx * sx, x1 = (rx + 1) * sx, y0 = ry * sy, y1 = (ry + 1) * sy;
        if (rx === 0 || data[ry * w + rx - 1] !== id) { path.moveTo(x0, y0); path.lineTo(x0, y1); }
        if (rx + 1 >= w || data[ry * w + rx + 1] !== id) { path.moveTo(x1, y0); path.lineTo(x1, y1); }
        if (ry === 0 || data[(ry - 1) * w + rx] !== id) { path.moveTo(x0, y0); path.lineTo(x1, y0); }
        if (ry + 1 >= h || data[(ry + 1) * w + rx] !== id) { path.moveTo(x0, y1); path.lineTo(x1, y1); }
      }
    }
    this.selectedProvincePath = path;
    this.selectedProvinceSeat = this.provinceSeats.get(id) ?? null;
  }

  /** Set the provinces MARKED for a batch merge/split; rebuilds their combined outline. */
  setMarkedProvinces(ids: number[]) {
    const next = new Set(ids);
    if (next.size === this.markedProvinces.size &&
        [...next].every((i) => this.markedProvinces.has(i))) return;
    this.markedProvinces = next;
    this.buildMarkedProvinces();
  }

  /** Cut the combined outline of every marked province out of the raster, in world cells. */
  private buildMarkedProvinces() {
    this.markedProvincePath = null;
    const r = this.provinceRaster;
    if (!r || this.markedProvinces.size === 0) return;
    const { data, w, h, gridW, gridH } = r;
    const sx = gridW / w, sy = gridH / h;
    const marked = this.markedProvinces;
    const path = new Path2D();
    for (let ry = 0; ry < h; ry++) {
      for (let rx = 0; rx < w; rx++) {
        const id = data[ry * w + rx];
        if (!marked.has(id)) continue;
        const x0 = rx * sx, x1 = (rx + 1) * sx, y0 = ry * sy, y1 = (ry + 1) * sy;
        // A border wherever a marked cell meets a cell of a DIFFERENT (or unmarked) id,
        // so several marked provinces read as their own outlined regions.
        if (rx === 0 || data[ry * w + rx - 1] !== id) { path.moveTo(x0, y0); path.lineTo(x0, y1); }
        if (rx + 1 >= w || data[ry * w + rx + 1] !== id) { path.moveTo(x1, y0); path.lineTo(x1, y1); }
        if (ry === 0 || data[(ry - 1) * w + rx] !== id) { path.moveTo(x0, y0); path.lineTo(x1, y0); }
        if (ry + 1 >= h || data[(ry + 1) * w + rx] !== id) { path.moveTo(x0, y1); path.lineTo(x1, y1); }
      }
    }
    this.markedProvincePath = path;
  }

  /** Rasterize province fills into an offscreen canvas ONCE (each cell one opaque
   *  pixel → composited exactly once, so blitting it semi-transparent shows no cell
   *  grid) and precompute the border segments as a Path2D in world-cell coordinates. */
  private buildProvinceRender(rgb: [number, number, number][]) {
    this.provinceCanvas = null;
    this.provinceBorderPath = null;
    const r = this.provinceRaster;
    if (!r) return;
    const { data, w, h, gridW, gridH } = r;
    const NO = 0xffffffff; // NO_PROVINCE sentinel (u32; the raster is Uint32Array)
    const cv = document.createElement("canvas");
    cv.width = w; cv.height = h;
    const ictx = cv.getContext("2d");
    if (!ictx) return;
    const img = ictx.createImageData(w, h);
    const px = img.data;
    for (let i = 0; i < w * h; i++) {
      const id = data[i];
      const c = id === NO ? undefined : rgb[id];
      if (!c) { px[i * 4 + 3] = 0; continue; }
      px[i * 4] = c[0]; px[i * 4 + 1] = c[1]; px[i * 4 + 2] = c[2]; px[i * 4 + 3] = 255;
    }
    ictx.putImageData(img, 0, 0);
    this.provinceCanvas = cv;
    // Borders: a segment wherever two LAND provinces meet (skip sea edges — the coast
    // already reads from the base terrain). Coordinates are world cells (sx=sy=1 at
    // full res). Built once; stroked each frame.
    const sx = gridW / w, sy = gridH / h;
    const path = new Path2D();
    for (let ry = 0; ry < h; ry++) {
      for (let rx = 0; rx < w; rx++) {
        const id = data[ry * w + rx];
        if (id === NO) continue;
        if (rx + 1 < w) {
          const n = data[ry * w + rx + 1];
          if (n !== NO && n !== id) { const x = (rx + 1) * sx; path.moveTo(x, ry * sy); path.lineTo(x, (ry + 1) * sy); }
        }
        if (ry + 1 < h) {
          const n = data[(ry + 1) * w + rx];
          if (n !== NO && n !== id) { const y = (ry + 1) * sy; path.moveTo(rx * sx, y); path.lineTo((rx + 1) * sx, y); }
        }
      }
    }
    this.provinceBorderPath = path;
  }
  /** Atlas 2.0 · set the refugee roads (age01 0 = fresh, 1 = fully faded). */
  drawMigrations(m: { fx: number; fy: number; tx: number; ty: number; age: number }[]) { this.migrations = m; }

  // ── Route-bound migration overlay (dots · ribbon · focus) ──
  // `routed` is the polyline SNAPPED onto the trade-route network (computed lazily
  // from `path`'s hub hops); we draw that so flows follow the roads instead of
  // slashing a straight line to the city.
  private migrationRoutes: { path: [number, number][]; culture: string; volume: number; to: number; age: number; routed?: [number, number][] }[] = [];
  private migrationMode: "dots" | "ribbon" | "focus" = "ribbon";
  private migrationFocusHub: number | null = null;
  private migMaxVol = 1;
  /** Set the reworked migration flows (polylines along trade routes + culture + volume). */
  setMigrationRoutes(r: { path: [number, number][]; culture: string; volume: number; to: number; age: number }[]) {
    this.migrationRoutes = r;
    this.migMaxVol = r.reduce((m, x) => Math.max(m, x.volume), 1);
  }

  /** The migration polyline snapped to the trade roads: route each hub→hub hop over
   *  the trade-route graph (same geometry as the trunks) and concatenate. Falls back
   *  to the straight hub segment for any hop the graph can't route. Memoized on the
   *  route object so the Dijkstra runs once per fetch, not every frame. */
  private routedMigration(r: { path: [number, number][]; routed?: [number, number][] }): [number, number][] {
    if (r.routed) return r.routed;
    const out: [number, number][] = [];
    for (let i = 1; i < r.path.length; i++) {
      const a = r.path[i - 1], b = r.path[i];
      const seg = this.routeAlongTradeRoutes(a, b) ?? [a, b];
      // Drop the duplicated junction shared with the previous segment.
      for (let k = out.length ? 1 : 0; k < seg.length; k++) out.push(seg[k]);
    }
    r.routed = out.length >= 2 ? out : r.path;
    return r.routed;
  }
  setMigrationMode(mode: "dots" | "ribbon" | "focus") { this.migrationMode = mode; }
  setMigrationFocus(hub: number | null) { this.migrationFocusHub = hub; }
  /** Deterministic vivid colour per culture name (stable across the app). */
  private cultureHue(name: string): string {
    let h = 0x811c9dc5;
    for (let i = 0; i < name.length; i++) { h ^= name.charCodeAt(i); h = Math.imul(h, 0x01000193); }
    const hue = (h >>> 0) % 360;
    return `hsl(${hue},70%,62%)`;
  }

  /** Named trade basins: a soft dashed hull around each cluster's member towns +
   *  a serif region label at its heart. Colours rotate a fixed palette so basins
   *  stay tellable-apart; seam-spanning basins are skipped (no slash). */
  private renderTradeBasins(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const W = this.worldW;
    this.basins.forEach((b, i) => {
      const col = BASIN_COLORS[i % BASIN_COLORS.length];
      const xs = b.pts.map((p) => p[0]);
      if (W > 0 && Math.max(...xs) - Math.min(...xs) > W / 2) return;
      const hull = convexHull(b.pts);
      if (hull.length < 3) return;
      const grow = 4; // soft margin, pushed out from the centroid
      ctx.beginPath();
      hull.forEach(([x, y], k) => {
        const dx = x - b.cx, dy = y - b.cy;
        const d = Math.hypot(dx, dy) || 1;
        const px = x + (dx / d) * grow + 0.5, py = y + (dy / d) * grow + 0.5;
        if (k === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
      });
      ctx.closePath();
      ctx.fillStyle = `${col}1c`;
      ctx.strokeStyle = `${col}66`;
      ctx.lineWidth = Math.max(0.6, 1.2 * inv);
      ctx.setLineDash([4 * inv, 3 * inv]);
      ctx.fill();
      ctx.stroke();
      ctx.setLineDash([]);
      const fs = Math.max(6, 11 * inv);
      // The basin's own hue still wins over the class colour — a basin is identified
      // by its tint elsewhere on the map, so only the FACE comes from the registry.
      this.drawLabel(ctx, "tradeBasin", b.name, b.cx + 0.5, b.cy - grow - 2 * inv, fs,
        "center", { color: col, halo: "rgba(0,0,0,0.7)", haloWidth: Math.max(1.5, 2.5 * inv) });
      ctx.textAlign = "left";
    });
  }

  /** Refugee roads: parchment dashed arcs from a dying town to its havens,
   *  fading out over ~4 years, with an arrowhead at the destination. */
  private renderMigrations(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const W = this.worldW;
    for (const m of this.migrations) {
      if (W > 0 && Math.abs(m.tx - m.fx) > W / 2) continue; // no seam slash
      const a = Math.max(0, 1 - m.age) * 0.75;
      if (a <= 0.03) continue;
      const mx = (m.fx + m.tx) / 2, my = (m.fy + m.ty) / 2;
      const dx = m.tx - m.fx, dy = m.ty - m.fy;
      const len = Math.hypot(dx, dy) || 1;
      const cxp = mx - (dy / len) * len * 0.12, cyp = my + (dx / len) * len * 0.12;
      ctx.strokeStyle = `rgba(232,217,176,${a.toFixed(3)})`;
      ctx.lineWidth = Math.max(0.5, 1.1 * inv);
      ctx.setLineDash([3 * inv, 2.4 * inv]);
      ctx.beginPath();
      ctx.moveTo(m.fx + 0.5, m.fy + 0.5);
      ctx.quadraticCurveTo(cxp + 0.5, cyp + 0.5, m.tx + 0.5, m.ty + 0.5);
      ctx.stroke();
      ctx.setLineDash([]);
      const ax = m.tx - cxp, ay = m.ty - cyp;
      const al = Math.hypot(ax, ay) || 1;
      const ux = ax / al, uy = ay / al;
      const s = Math.max(1.2, 2.2 * inv);
      ctx.fillStyle = `rgba(232,217,176,${a.toFixed(3)})`;
      ctx.beginPath();
      ctx.moveTo(m.tx + 0.5, m.ty + 0.5);
      ctx.lineTo(m.tx + 0.5 - ux * s - uy * s * 0.5, m.ty + 0.5 - uy * s + ux * s * 0.5);
      ctx.lineTo(m.tx + 0.5 - ux * s + uy * s * 0.5, m.ty + 0.5 - uy * s - ux * s * 0.5);
      ctx.closePath();
      ctx.fill();
    }
  }

  /** Route-bound migration: every flow is drawn STRICTLY along its trade-route polyline
   *  (through the intermediate hubs the sim routed it over), coloured by the migrants'
   *  culture. Three modes: `ribbon` (width ∝ volume) · `dots` (spaced markers riding the
   *  path) · `focus` (only flows arriving at the focused hub, brightly). */
  private renderMigrationRoutes(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const W = this.worldW;
    const focus = this.migrationFocusHub;
    const spanOk = (p: [number, number][]) => {
      // Skip a flow that would slash across the cylindrical seam.
      for (let i = 1; i < p.length; i++) if (W > 0 && Math.abs(p[i][0] - p[i - 1][0]) > W / 2) return false;
      return true;
    };
    for (const r of this.migrationRoutes) {
      if (r.path.length < 2) continue;
      if (this.migrationMode === "focus" && focus != null && r.to !== focus) continue;
      const path = this.routedMigration(r);
      if (path.length < 2 || !spanOk(path)) continue;
      const focused = this.migrationMode === "focus" && focus != null && r.to === focus;
      const fade = Math.max(0.12, 1 - r.age / 6); // ~6y lifetime
      const col = this.cultureHue(r.culture);
      const volN = Math.min(1, r.volume / this.migMaxVol);

      if (this.migrationMode === "dots") {
        // Markers spaced along the routed polyline.
        ctx.strokeStyle = this.rgba(col, fade * 0.28);
        ctx.lineWidth = Math.max(0.4, 0.8 * inv);
        this.tracePath(ctx, path);
        ctx.stroke();
        const step = 4;
        for (let i = 1; i < path.length; i++) {
          const [x0, y0] = path[i - 1], [x1, y1] = path[i];
          const segs = Math.max(1, Math.round(Math.hypot(x1 - x0, y1 - y0) / step));
          for (let s = 0; s < segs; s++) {
            const t = s / segs;
            ctx.fillStyle = this.rgba(col, fade);
            ctx.beginPath();
            ctx.arc(x0 + (x1 - x0) * t + 0.5, y0 + (y1 - y0) * t + 0.5, Math.max(0.7, (0.9 + volN) * inv), 0, Math.PI * 2);
            ctx.fill();
          }
        }
      } else {
        // Ribbon (and focus) — a stroked path whose width tracks volume, drawn as a
        // DASH-DOT line ( -•-•-• ) so migration reads distinctly from solid trade lanes,
        // with an arrowhead into the destination city.
        ctx.strokeStyle = this.rgba(col, focused ? Math.min(1, fade + 0.25) : fade * 0.85);
        ctx.lineWidth = Math.max(0.6, (focused ? 2.2 : 1.0 + volN * 2.4) * inv);
        ctx.lineJoin = "round"; ctx.lineCap = "round";
        // dash · gap · dot(~0, rendered as a round cap) · gap → the "-•-•-•" pattern.
        const dash = 2.4 * inv, gap = 1.7 * inv;
        ctx.setLineDash([dash, gap, 0.01, gap]);
        this.tracePath(ctx, path);
        ctx.stroke();
        ctx.setLineDash([]); // reset so it doesn't leak into other overlays
        // Arrowhead at the destination.
        const n = path.length;
        const [px, py] = path[n - 2], [qx, qy] = path[n - 1];
        const al = Math.hypot(qx - px, qy - py) || 1;
        const ux = (qx - px) / al, uy = (qy - py) / al;
        const s = Math.max(1.4, 2.6 * inv);
        ctx.fillStyle = this.rgba(col, focused ? 1 : fade);
        ctx.beginPath();
        ctx.moveTo(qx + 0.5, qy + 0.5);
        ctx.lineTo(qx + 0.5 - ux * s - uy * s * 0.5, qy + 0.5 - uy * s + ux * s * 0.5);
        ctx.lineTo(qx + 0.5 - ux * s + uy * s * 0.5, qy + 0.5 - uy * s - ux * s * 0.5);
        ctx.closePath();
        ctx.fill();
      }
    }
  }

  private tracePath(ctx: CanvasRenderingContext2D, p: [number, number][]) {
    ctx.beginPath();
    ctx.moveTo(p[0][0] + 0.5, p[0][1] + 0.5);
    for (let i = 1; i < p.length; i++) ctx.lineTo(p[i][0] + 0.5, p[i][1] + 0.5);
  }

  /** hsl(...) / #hex → rgba-ish stroke with alpha via globalAlpha-free helper. */
  private rgba(col: string, a: number): string {
    if (col.startsWith("hsl(")) return col.replace("hsl(", "hsla(").replace(")", `,${a.toFixed(3)})`);
    return col;
  }

  /** Trade Heat: additive radial glows sized/coloured by each hub's share of the
   *  busiest hub's yearly throughput. sqrt ramp keeps mid-size markets visible;
   *  "lighter" compositing makes overlapping basins genuinely GLOW — the regions
   *  where trade concentrates read instantly. */
  private renderTradeHeat(ctx: CanvasRenderingContext2D) {
    const max = this.heatPoints.reduce((m, p) => Math.max(m, p.v), 0);
    if (max <= 0) return;
    const ramp = (t: number): string => {
      // teal (79,208,192) → gold (216,178,74) → crimson (214,80,58)
      const lerp = (a: number, b: number, k: number) => Math.round(a + (b - a) * k);
      const [r, g, b] = t < 0.5
        ? [lerp(79, 216, t * 2), lerp(208, 178, t * 2), lerp(192, 74, t * 2)]
        : [lerp(216, 214, t * 2 - 1), lerp(178, 80, t * 2 - 1), lerp(74, 58, t * 2 - 1)];
      return `${r},${g},${b}`;
    };
    ctx.save();
    ctx.globalCompositeOperation = "lighter";
    for (const p of this.heatPoints) {
      const t = Math.sqrt(p.v / max);
      if (t < 0.05) continue;
      const r = 3 + 20 * t; // world cells — scales naturally with zoom
      const cx = p.x + 0.5, cy = p.y + 0.5;
      const grad = ctx.createRadialGradient(cx, cy, 0, cx, cy, r);
      const col = ramp(t);
      grad.addColorStop(0, `rgba(${col},${(0.10 + 0.32 * t).toFixed(3)})`);
      grad.addColorStop(0.55, `rgba(${col},${(0.15 * t).toFixed(3)})`);
      grad.addColorStop(1, `rgba(${col},0)`);
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.arc(cx, cy, r, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  }

  drawWindArrows(data: VectorSample[], gridW: number, gridH: number) {
    this.windData = { samples: data, gridW, gridH };
  }

  drawCurrentStreamlines(lines: Streamline[]) {
    this.currentLines = lines;
  }

  /** Set (or clear with []) the highlighted point-to-point itinerary route. */
  drawTravelRoute(points: [number, number][]) {
    this.travelRoute = points;
  }

  /** Set (or clear with []) the hand-drawn ridge lines to sketch on the map. */
  setRidgeSketch(lines: RidgeLine[]) {
    this.ridgeSketch = lines;
  }

  /** 🌊 Set (or clear with null/[]) which rivers glow as the selected system,
   *  with an optional per-river-index colour map (branch / order scheme). */
  setRiverHighlight(ids: number[] | null, colors?: Record<number, string> | null) {
    this.riverHighlight = new Set(ids ?? []);
    this.riverHighlightColors = colors ?? {};
  }

  /** Set (or clear with []) the per-hub scarcity discs for the selected good. */
  drawGoodScarcity(cities: { x: number; y: number; premium: number }[]) {
    this.goodScarcity = cities;
  }

  /** Set (or clear with []) the named geographic features to label. */
  drawToponyms(t: { kind: string; name: string; x: number; y: number }[]) {
    this.toponyms = t;
    this.riverLabelCache.clear();
  }

  /** Set (or clear with []) the reach-break markers along trunk rivers. */
  drawRiverBreaks(breaks: { x: number; y: number; tx: number; ty: number; label: string }[]) {
    this.riverBreaks = breaks;
  }

  drawTradeRoutes(routes: TradeRoute[]) {
    this.tradeRoutes = routes;
    // The road network changed → re-snap any already-loaded house web / futures
    // lanes / merchant routes onto the new routes so they keep following what's drawn.
    this.recomputeHouseNetwork();
    this.routeFuturesLanes();
    this.routeMerchantRoutes();
  }

  /** Coin-usage overlay data + the selected coin (mint hub id) to highlight. */
  setCoinUsage(usage: CoinUseCity[], hub: number | null) {
    this.coinUse = usage;
    this.coinOverlayHub = hub;
  }

  /** Bank seats to mark on the map (pass [] to hide). */
  setBankIcons(banks: { x: number; y: number; name: string; defunct: boolean; color: string }[]) {
    this.bankIcons = banks;
  }

  /** Phase 6 · plague overlay: struck cities + contagion routes (pass [],[] to hide). */
  setEpidemics(
    cities: { x: number; y: number; active: boolean; deaths: number; origin: boolean }[],
    edges: { ax: number; ay: number; bx: number; by: number }[],
  ) {
    this.plagueCities = cities;
    this.plagueEdges = edges;
  }

  /** Phase 6 · guild-city overlay (pass [] to hide). */
  setGuilds(cities: { x: number; y: number; emoji: string; label: string }[]) {
    this.guildCities = cities;
  }

  /** Phase 6 · living-figure map markers (pass [] to hide). */
  setFigureMarks(marks: { x: number; y: number; emoji: string }[]) { this.figureMarks = marks; }
  /** Phase 6 · landmark map markers (pass [] to hide). */
  setLandmarkMarks(marks: { x: number; y: number; emoji: string }[]) { this.landmarkMarks = marks; }
  /** Phase 7 · dynasty ties between house seat cities (pass [] to hide). */
  setDynastyLinks(links: { ax: number; ay: number; bx: number; by: number; ally: boolean }[]) { this.dynastyLinks = links; }

  /** Hit-test a world point against the bank icons (for click-to-open). Returns the
   *  bank's list index, or -1. `tol` is in world cells. */
  bankIconAt(wx: number, wy: number, tol: number): number {
    let best = -1, bestD = tol * tol;
    for (let i = 0; i < this.bankIcons.length; i++) {
      const b = this.bankIcons[i];
      if (b.defunct) continue;
      const dx = wx - (b.x + 0.5), dy = wy - (b.y + 0.5);
      const d = dx * dx + dy * dy;
      if (d < bestD) { bestD = d; best = i; }
    }
    return best;
  }

  /** Build (once per routes array) an undirected graph whose nodes are settlement
   *  junctions (trade-route endpoints) and whose edges are the route polylines, so a
   *  path between two cities is literally a chain of EXISTING routes. */
  private ensureTradeGraph() {
    if (this.tgRef === this.tradeRoutes) return;
    this.tgRef = this.tradeRoutes;
    const nodes: [number, number][] = [];
    const adj: { to: number; pts: [number, number][]; len: number }[][] = [];
    const idOf = new Map<string, number>();
    const W = this.worldW;
    const key = (p: [number, number]) => `${Math.round(p[0])},${Math.round(p[1])}`;
    const getNode = (p: [number, number]) => {
      const k = key(p);
      let id = idOf.get(k);
      if (id === undefined) { id = nodes.length; nodes.push(p); idOf.set(k, id); adj.push([]); }
      return id;
    };
    const plen = (pts: [number, number][]) => {
      let L = 0;
      for (let i = 1; i < pts.length; i++) {
        let dx = pts[i][0] - pts[i - 1][0];
        if (W && Math.abs(dx) > W / 2) dx -= Math.sign(dx) * W;
        L += Math.hypot(dx, pts[i][1] - pts[i - 1][1]);
      }
      return L;
    };
    for (const r of this.tradeRoutes) {
      const pts = r.points;
      if (pts.length < 2) continue;
      const a = getNode(pts[0]);
      const b = getNode(pts[pts.length - 1]);
      if (a === b) continue;
      const len = plen(pts);
      adj[a].push({ to: b, pts, len });
      adj[b].push({ to: a, pts: [...pts].slice().reverse(), len });
    }
    this.tgNodes = nodes;
    this.tgAdj = adj;
  }

  /** Shortest path from `a` to `b` ALONG the existing trade-route graph (Dijkstra by
   *  road length), returned as one concatenated world-cell polyline. Null if either
   *  end can't be attached to the network or no road path exists. */
  private routeAlongTradeRoutes(a: [number, number], b: [number, number]): [number, number][] | null {
    this.ensureTradeGraph();
    const n = this.tgNodes.length;
    if (n === 0) return null;
    const W = this.worldW;
    const nearest = (p: [number, number]) => {
      let best = -1, bd = Infinity;
      for (let i = 0; i < n; i++) {
        let dx = this.tgNodes[i][0] - p[0];
        if (W && Math.abs(dx) > W / 2) dx -= Math.sign(dx) * W;
        const dy = this.tgNodes[i][1] - p[1];
        const d = dx * dx + dy * dy;
        if (d < bd) { bd = d; best = i; }
      }
      return best;
    };
    const s = nearest(a), t = nearest(b);
    if (s < 0 || t < 0 || s === t) return null;
    // Dijkstra with a binary min-heap (graph is sparse: ~3 edges/node).
    const dist = new Float64Array(n).fill(Infinity);
    const prev = new Int32Array(n).fill(-1);
    const prevPts: ([number, number][] | null)[] = new Array(n).fill(null);
    dist[s] = 0;
    const heap: number[] = [s];        // node ids, ordered by dist
    const hpush = (node: number) => {
      heap.push(node);
      let i = heap.length - 1;
      while (i > 0) {
        const par = (i - 1) >> 1;
        if (dist[heap[par]] <= dist[heap[i]]) break;
        [heap[par], heap[i]] = [heap[i], heap[par]]; i = par;
      }
    };
    const hpop = () => {
      const top = heap[0], last = heap.pop()!;
      if (heap.length) {
        heap[0] = last;
        let i = 0;
        for (;;) {
          const l = 2 * i + 1, r = 2 * i + 2; let m = i;
          if (l < heap.length && dist[heap[l]] < dist[heap[m]]) m = l;
          if (r < heap.length && dist[heap[r]] < dist[heap[m]]) m = r;
          if (m === i) break;
          [heap[m], heap[i]] = [heap[i], heap[m]]; i = m;
        }
      }
      return top;
    };
    const done = new Uint8Array(n);
    while (heap.length) {
      const u = hpop();
      if (done[u]) continue;
      done[u] = 1;
      // Run the FULL Dijkstra (no early break): if `t` turns out unreachable we need
      // every node's distance to pick the best reachable junction to ride toward.
      for (const e of this.tgAdj[u]) {
        const nd = dist[u] + e.len;
        if (nd < dist[e.to]) { dist[e.to] = nd; prev[e.to] = u; prevPts[e.to] = e.pts; hpush(e.to); }
      }
    }
    // The line must follow the corridor network end-to-end. If the partner's junction
    // isn't reachable along existing routes, return null so the caller SKIPS it —
    // we never bridge with a straight line.
    if (!isFinite(dist[t])) return null;
    const segs: [number, number][][] = [];
    let cur = t;
    while (cur !== s && prev[cur] >= 0) { segs.push(prevPts[cur]!); cur = prev[cur]; }
    segs.reverse();
    const out: [number, number][] = [a];
    for (const seg of segs) for (const p of seg) out.push(p);
    out.push(b);
    return out;
  }

  /** Re-snap the focused house's seat→city web onto the current trade routes. */
  private recomputeHouseNetwork() {
    const sel = this.selectedHouseIdx != null
      ? this.allHouses.find((h) => h.idx === this.selectedHouseIdx) ?? null
      : null;
    if (!sel || !sel.seat) { this.houseNetwork = []; return; }
    const cities: [number, number][] = [
      ...(sel.controls ?? []), ...(sel.partners ?? []),
      ...((sel.offices ?? []).map((o) => o[1]).filter(Boolean) as [number, number][]),
    ];
    const net: [number, number][][] = [];
    for (const c of cities) {
      if (c[0] === sel.seat[0] && c[1] === sel.seat[1]) continue;
      const path = this.routeAlongTradeRoutes(sel.seat, c);
      if (path && path.length >= 2) net.push(path); // corridor only — never a straight line
    }
    this.houseNetwork = net;
  }

  /** Snap every futures lane onto the existing trade routes (source→buyer). */
  private routeFuturesLanes() {
    for (const r of this.futuresLanes) {
      const path = this.routeAlongTradeRoutes(r.a, r.b);
      r.path = path && path.length >= 2 ? path : undefined;
    }
  }

  /** Snap every merchant route onto the existing trade routes (a→b). Cached on each
   *  route's `path` so we run Dijkstra only when the route set or the road network
   *  changes — never per frame. A route with no corridor path is left `path`-less
   *  and SKIPPED at draw time (we never bridge with a straight slash). */
  private routeMerchantRoutes() {
    for (const r of this.merchantRoutes) {
      const path = this.routeAlongTradeRoutes(r.a, r.b);
      r.path = path && path.length >= 2 ? path : undefined;
    }
  }

  drawFisheryBanks(banks: FisheryBank[]) {
    this.fisheryBanks = banks;
  }

  drawSharkZones(zones: SharkZone[]) {
    this.sharkZones = zones;
  }

  drawShipwormZones(zones: SharkZone[]) {
    this.shipwormZones = zones;
  }

  drawStormZones(zones: SharkZone[]) {
    this.stormZones = zones;
  }

  /** Circulation bands (ITCZ line + subtropical-high / polar-front belts). */
  setClimateBands(bands: ClimateBands | null) {
    this.climateBands = bands;
  }

  drawMonsoonZones(zones: SharkZone[]) {
    this.monsoonZones = zones;
  }

  drawReefZones(zones: SharkZone[]) {
    this.reefZones = zones;
  }

  drawGoodRegions(regions: GoodRegion[]) {
    this.goodRegions = regions;
  }

  drawCultureRegions(regions: CultureRegion[]) {
    this.cultureRegions = regions;
  }

  /** CITY_PROVINCE_WAR_PLAN.md §3.3 · a tier 1-2 city's writ, drawn as a territory.
   *  Rendered on the province raster so its border is the province border exactly. */
  drawStates(states: StateRegion[]) {
    this.stateRegions = states;
    this.statesDirty = true;
  }

  /** Build the state fill canvas + border path from the province raster, tinting
   *  exactly the cells of the provinces each state administers. Same technique as
   *  `buildProvinceRender`/`buildSelectedProvince`, so state borders coincide with
   *  province borders by construction (never an approximating cell cloud). */
  private buildStateRender() {
    this.statesDirty = false;
    this.stateCanvas = null;
    this.stateBorderPath = null;
    const r = this.provinceRaster;
    if (!r || this.stateRegions.length === 0) return;
    const { data, w, h, gridW, gridH } = r;
    const NO = 0xffffffff;
    // pid → state colour (packed RGB, 0 = not in any state). One entry per province.
    const stateColor = new Map<number, [number, number, number]>();
    // A per-pid "which state" tag for border detection (0 = none; else index+1).
    const stateTag = new Map<number, number>();
    this.stateRegions.forEach((s, si) => {
      for (const pid of s.province_ids) {
        stateColor.set(pid, s.color);
        stateTag.set(pid, si + 1);
      }
    });
    if (stateColor.size === 0) return;

    const cv = document.createElement("canvas");
    cv.width = w; cv.height = h;
    const ictx = cv.getContext("2d");
    if (!ictx) return;
    const img = ictx.createImageData(w, h);
    const px = img.data;
    for (let i = 0; i < w * h; i++) {
      const pid = data[i];
      const col = pid === NO ? undefined : stateColor.get(pid);
      const o = i * 4;
      if (col) {
        px[o] = col[0]; px[o + 1] = col[1]; px[o + 2] = col[2]; px[o + 3] = 255;
      } else {
        px[o + 3] = 0; // transparent — land with no state / sea
      }
    }
    ictx.putImageData(img, 0, 0);
    this.stateCanvas = cv;

    // Border: an edge wherever a cell's STATE differs from its neighbour's (a
    // different state, or none). Traced in world-cell coords, like the province path.
    const sx = gridW / w, sy = gridH / h;
    const tagAt = (rx: number, ry: number): number => {
      if (rx < 0 || ry < 0 || rx >= w || ry >= h) return 0;
      const pid = data[ry * w + rx];
      return pid === NO ? 0 : (stateTag.get(pid) ?? 0);
    };
    const path = new Path2D();
    for (let ry = 0; ry < h; ry++) {
      for (let rx = 0; rx < w; rx++) {
        const t = tagAt(rx, ry);
        if (t === 0) continue;
        const x0 = rx * sx, x1 = (rx + 1) * sx, y0 = ry * sy, y1 = (ry + 1) * sy;
        if (tagAt(rx - 1, ry) !== t) { path.moveTo(x0, y0); path.lineTo(x0, y1); }
        if (tagAt(rx + 1, ry) !== t) { path.moveTo(x1, y0); path.lineTo(x1, y1); }
        if (tagAt(rx, ry - 1) !== t) { path.moveTo(x0, y0); path.lineTo(x1, y0); }
        if (tagAt(rx, ry + 1) !== t) { path.moveTo(x0, y1); path.lineTo(x1, y1); }
      }
    }
    this.stateBorderPath = path;
  }

  /** Drop a transient highlight pin at a world cell (searched settlement). */
  setSearchPin(wx: number, wy: number) {
    this.searchPin = { wx, wy };
  }
  clearSearchPin() {
    this.searchPin = null;
  }

  /** Light the two cities of a war: `a` = attacker (red), `b` = defender (blue). */
  setWarHighlight(ax: number, ay: number, bx: number, by: number) {
    this.warHighlight = { ax, ay, bx, by };
  }
  clearWarHighlight() {
    this.warHighlight = null;
  }

  setGoodMeta(meta: Map<string, { icon: string; color: string }>) {
    this.goodMeta = meta;
  }

  drawTradeTrunks(trunks: TradeTrunk[], gridW: number) {
    this.tradeTrunks = trunks;
    this.worldW = gridW;
  }

  /** DLC 3.5 · the live yearly-averaged dynamic trade-flow trunks. */
  drawDynamicFlow(trunks: TradeTrunk[], gridW: number) {
    this.dynamicTrunks = trunks;
    if (gridW > 0) this.worldW = gridW;
  }

  drawTradeCorridors(corridors: TradeCorridor[], gridW: number) {
    this.tradeCorridorList = corridors;
    if (gridW > 0) this.worldW = gridW;
  }

  drawExpeditions(active: ExpeditionView[], failed: ExpeditionFail[], gridW: number) {
    this.expeditions = active;
    this.expeditionFails = failed;
    if (gridW > 0) this.worldW = gridW;
  }

  drawMerchantRoutes(routes: MerchantRoute[], gridW: number) {
    this.merchantRoutes = routes;
    if (gridW > 0) this.worldW = gridW;
    // Snap each route onto the existing trade routes (roads/sea-lanes).
    this.routeMerchantRoutes();
  }

  /** Transient highlight of one settlement's trade flows (Trade ▸ Flows subtab):
   *  glowing arrows between the city and its partners. dir 0 = inbound (arrow → city),
   *  1 = outbound (arrow → partner). Drawn whenever set; cleared with []. */
  flowHighlight: { ax: number; ay: number; bx: number; by: number; dir: number; w: number }[] = [];
  setFlowHighlight(segs: { ax: number; ay: number; bx: number; by: number; dir: number; w: number }[], gridW: number) {
    this.flowHighlight = segs;
    if (gridW > 0) this.worldW = gridW;
  }

  /** Nearest active merchant route to a world point, within `thresh` cells (for
   *  click-to-inspect). Returns null if none close. */
  pickMerchantRoute(wx: number, wy: number, thresh: number): MerchantRoute | null {
    let best: MerchantRoute | null = null;
    let bestD = thresh * thresh;
    for (const r of this.merchantRoutes) {
      const ax = r.a[0] + 0.5, ay = r.a[1] + 0.5, bx = r.b[0] + 0.5, by = r.b[1] + 0.5;
      if (this.worldW > 0 && Math.abs(ax - bx) > this.worldW / 2) continue;
      const dx = bx - ax, dy = by - ay;
      const len2 = dx * dx + dy * dy;
      const t = len2 > 1e-6 ? Math.max(0, Math.min(1, ((wx - ax) * dx + (wy - ay) * dy) / len2)) : 0;
      const px = ax + t * dx, py = ay + t * dy;
      const d = (wx - px) * (wx - px) + (wy - py) * (wy - py);
      if (d < bestD) { bestD = d; best = r; }
    }
    return best;
  }

  drawFutures(lanes: FuturesLane[], gridW: number, selected: FuturesLane | null,
              focus: { city?: string; holder?: string; good?: string } | null) {
    this.futuresLanes = lanes;
    this.selectedFuturesLane = selected;
    this.futuresFocus = focus;
    if (gridW > 0) this.worldW = gridW;
    // Snap each lane onto the existing trade routes (source→buyer).
    this.routeFuturesLanes();
  }

  /** Nearest futures lane to a world point, within `thresh` cells (click-to-inspect). */
  pickFuturesLane(wx: number, wy: number, thresh: number): FuturesLane | null {
    let best: FuturesLane | null = null;
    let bestD = thresh * thresh;
    for (const r of this.futuresLanes) {
      const ax = r.a[0] + 0.5, ay = r.a[1] + 0.5, bx = r.b[0] + 0.5, by = r.b[1] + 0.5;
      if (this.worldW > 0 && Math.abs(ax - bx) > this.worldW / 2) continue;
      const dx = bx - ax, dy = by - ay;
      const len2 = dx * dx + dy * dy;
      const t = len2 > 1e-6 ? Math.max(0, Math.min(1, ((wx - ax) * dx + (wy - ay) * dy) / len2)) : 0;
      const px = ax + t * dx, py = ay + t * dy;
      const d = (wx - px) * (wx - px) + (wy - py) * (wy - py);
      if (d < bestD) { bestD = d; best = r; }
    }
    return best;
  }

  drawPolitical(centers: PoliticalCenter[]) {
    this.politicalCenters = centers;
  }

  /** DLC 3 · per-polis speculation-risk discs (green→amber→red by tier). */
  drawSpeculation(centers: SpecCenter[]) {
    this.specCenters = centers;
  }

  /** Merchant-family control: houses that control >=1 settlement (>=50% of its
   *  trade) — its seat or a remote outpost. Pass the active houses + wrap width. */
  drawHouseControl(houses: HouseBrief[], gridW: number, selectedIdx?: number | null) {
    this.houses = houses.filter((h) => !h.defunct && h.controls && h.controls.length > 0);
    // Keep ALL houses (even ones controlling nothing) so a selected house can still
    // show its sphere/routes/offices.
    this.allHouses = houses.filter((h) => !h.defunct);
    this.selectedHouseIdx = selectedIdx ?? null;
    if (gridW > 0) this.worldW = gridW;
    // Snap the focused house's web onto the existing trade routes.
    this.recomputeHouseNetwork();
  }

  drawChokepoints(chokepoints: EconChokepoint[]) {
    this.chokepoints = chokepoints;
  }

  drawCorridors(corridors: EconCorridor[], gridW: number) {
    this.corridors = corridors;
    this.worldW = gridW;
  }

  /** Adjustable hub marker display: size multiplier and highlight intensity. */
  setHubDisplay(size: number, intensity: number) {
    this.hubSize = size;
    this.hubIntensity = intensity;
  }

  drawEconRegions(regions: EconRegion[]) {
    this.econRegions = regions;
  }

  /** Set the per-good reach network: the chains carrying one good and the hub
   *  positions it reaches (drawn highlighted). Pass empty to clear. */
  setReachNetwork(chains: EconChain[], hubs: [number, number][]) {
    this.reachChains = chains;
    this.reachHubs = hubs;
  }

  /** Highlight one good's supply chain (origin → hub stops → consumer) with the
   *  price at each stop. Pass null to clear. */
  setSupplyChain(chain: EconChain | null, isImport = false) {
    this.supplyChain = chain;
    this.supplyChainImport = isImport;
  }

  drawLatLines(gridW: number, gridH: number, equatorOffset = 0.5, latScale = 1, lineRatio = 1) {
    this.latLinesData = { gridW, gridH, equatorOffset, latScale, lineRatio };
  }

  setVisible(type: string, visible: boolean) {
    this.visibility[type] = visible;
  }

  updateScale(scale: number) {
    this.currentScale = scale;
  }

  /** Meandered render path for a river, cached per raw `points` array (meanders
   *  are scale-independent, so this is computed once per rivers-data load). */
  private riverPath(river: RiverData, id: number, order: number): [number, number][] {
    const cached = this.meanderCache.get(river.points);
    if (cached) return cached;
    // Meanders are generated in the BACKEND (build_meander_path): physically
    // clamped to the valley AND walled off from neighbouring rivers so render paths
    // never cross. The frontend only draws that path (Catmull-Rom smoothed). Old
    // saves that predate the `render` field fall back to the TRUE cell path — which,
    // being the real drainage tree, is itself non-crossing — rather than the old
    // cosmetic client-side meander that could weave across neighbours.
    void id; void order;
    const path: [number, number][] =
      river.render && river.render.length >= 2 ? river.render : river.points;
    this.meanderCache.set(river.points, path);
    return path;
  }

  /** Does a candidate label box overlap any already-placed label this frame? */
  private labelCollides(x0: number, y0: number, x1: number, y1: number): boolean {
    for (const r of this.placedLabels) {
      if (x0 < r.x1 && x1 > r.x0 && y0 < r.y1 && y1 > r.y0) return true;
    }
    return false;
  }

  /** Reserve a label box (centered horizontally on `cx`, bottom-anchored at
   *  `baseY`) if it doesn't collide with an already-placed one. Returns true and
   *  records the box when placed; false when it should be skipped. `w`/`h` are the
   *  text metrics in world (cell) units. A small pad keeps neighbours from kissing. */
  // ── Map label typography helpers (see the `labelStyles` registry above) ──────
  // Every place-name render site goes through these, so a class's face, case,
  // tracking and colour live in ONE place instead of at ~23 scattered call sites.
  //
  // `px` is a size in the CURRENT canvas units — the callers already divide by
  // `currentScale` where the context carries a world transform, so these helpers
  // stay transform-agnostic.

  /** The class's text as it should read (applies ALL-CAPS). */
  private labelText(key: LabelKey, s: string): string {
    return labelStyles[key].caps ? s.toUpperCase() : s;
  }

  /** CSS font string for a class at a given size. */
  private labelFont(key: LabelKey, px: number): string {
    const st = labelStyles[key];
    return `${st.italic ? "italic " : ""}${st.weight} ${px * st.size}px ${st.family}`;
  }

  /** Width of a class's label INCLUDING tracking. The province fit-test depends on
   *  this being exact — a tracked capital string is far wider than the raw text. */
  private measureLabel(ctx: CanvasRenderingContext2D, key: LabelKey, raw: string, px: number): number {
    const st = labelStyles[key];
    const text = this.labelText(key, raw);
    ctx.font = this.labelFont(key, px);
    const base = ctx.measureText(text).width;
    if (st.tracking === 0 || text.length < 2) return base;
    return base + st.tracking * px * st.size * (text.length - 1);
  }

  /** Draw a class's label with its halo, honouring tracking. Returns the width drawn.
   *
   *  Tracking is applied by drawing CHARACTER BY CHARACTER rather than through
   *  `ctx.letterSpacing`: that property is Chromium-only in practice, and Tauri runs
   *  WebKit2GTK on Linux and WKWebView on macOS, so relying on it would silently drop
   *  tracking on two of the three platforms. Drawing by hand also gives an exact
   *  advance, which `measureLabel` needs. */
  private drawLabel(
    ctx: CanvasRenderingContext2D, key: LabelKey, raw: string,
    x: number, y: number, px: number,
    align: CanvasTextAlign = "left",
    opts?: { color?: string; halo?: string; haloWidth?: number },
  ): number {
    const st = labelStyles[key];
    const text = this.labelText(key, raw);
    if (!text) return 0;
    const size = px * st.size;
    ctx.font = this.labelFont(key, px);
    ctx.lineJoin = "round";
    const halo = opts?.halo ?? "rgba(6,12,18,0.85)";
    const haloW = opts?.haloWidth ?? Math.max(0.4, size * 0.2);
    const fill = opts?.color ?? st.color;

    const track = st.tracking * size;
    const width = this.measureLabel(ctx, key, raw, px);
    // No tracking → one fillText is both cheaper and better kerned.
    if (track === 0 || text.length < 2) {
      ctx.textAlign = align;
      ctx.lineWidth = haloW; ctx.strokeStyle = halo; ctx.strokeText(text, x, y);
      ctx.fillStyle = fill; ctx.fillText(text, x, y);
      ctx.textAlign = "left";
      return width;
    }
    // Tracked: lay out from a left origin derived from the requested alignment.
    let cx = align === "center" ? x - width / 2 : align === "right" ? x - width : x;
    ctx.textAlign = "left";
    ctx.lineWidth = haloW; ctx.strokeStyle = halo;
    for (const ch of text) {
      ctx.strokeText(ch, cx, y);
      cx += ctx.measureText(ch).width + track;
    }
    cx = align === "center" ? x - width / 2 : align === "right" ? x - width : x;
    ctx.fillStyle = fill;
    for (const ch of text) {
      ctx.fillText(ch, cx, y);
      cx += ctx.measureText(ch).width + track;
    }
    return width;
  }

  private reserveLabel(cx: number, baseY: number, w: number, h: number): boolean {
    const pad = h * 0.18;
    const x0 = cx - w / 2 - pad, x1 = cx + w / 2 + pad;
    const y0 = baseY - h - pad, y1 = baseY + pad;
    if (this.labelCollides(x0, y0, x1, y1)) return false;
    this.placedLabels.push({ x0, y0, x1, y1 });
    return true;
  }

  /** Draw the province partition: a single blit of the prebuilt full-res fill canvas
   *  (composited once → NO cell grid, follows the coastline exactly) + the prebuilt
   *  border path + seat name labels. Per-frame cost is O(1) in world size. */
  private renderProvinces(ctx: CanvasRenderingContext2D) {
    const r = this.provinceRaster;
    if (!r || !this.provinceCanvas) return;
    const { w, h, gridW, gridH } = r;
    // Fills — one blit at the user's opacity (each pixel already composited exactly
    // once, so fading it never reveals a cell grid). Only the FILL is faded; the
    // borders, names and selection outline below stay at full strength, so winding the
    // slider down leaves a clean political outline over the terrain.
    ctx.imageSmoothingEnabled = false;
    if (this.provinceOpacity > 0) {
      ctx.globalAlpha = this.provinceOpacity;
      ctx.drawImage(this.provinceCanvas, 0, 0, w, h, 0, 0, gridW, gridH);
      ctx.globalAlpha = 1;
    }
    // Borders — thin dark line between adjacent provinces (1 screen px at any zoom).
    if (this.provinceBorderPath) {
      ctx.strokeStyle = this.provinceBorderColor;
      ctx.lineWidth = 1 / this.currentScale;
      ctx.stroke(this.provinceBorderPath);
    }

    // Provinces marked for a batch merge/split: a distinct cyan outline (dark halo
    // underneath), drawn UNDER the picked-province highlight so selection still reads.
    if (this.markedProvincePath) {
      ctx.lineJoin = "round";
      ctx.strokeStyle = "rgba(6, 12, 20, 0.8)";
      ctx.lineWidth = 4 / this.currentScale;
      ctx.stroke(this.markedProvincePath);
      ctx.strokeStyle = "rgba(90, 220, 240, 0.95)";
      ctx.lineWidth = 1.8 / this.currentScale;
      ctx.stroke(this.markedProvincePath);
    }

    // The picked province: a bright outline (dark halo underneath so it reads over
    // any fill colour) + a ★ on its seat.
    if (this.selectedProvincePath) {
      ctx.lineJoin = "round";
      ctx.strokeStyle = "rgba(6, 12, 20, 0.85)";
      ctx.lineWidth = 4 / this.currentScale;
      ctx.stroke(this.selectedProvincePath);
      ctx.strokeStyle = "rgba(255, 226, 138, 0.95)";
      ctx.lineWidth = 1.8 / this.currentScale;
      ctx.stroke(this.selectedProvincePath);
      const seat = this.selectedProvinceSeat;
      if (seat) {
        const fs = Math.max(9, 15 / this.currentScale);
        ctx.font = `${fs}px system-ui, sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.lineWidth = Math.max(0.8, 3 / this.currentScale);
        ctx.strokeStyle = "rgba(6, 10, 16, 0.9)";
        ctx.strokeText("★", seat.x + 0.5, seat.y + 0.5);
        ctx.fillStyle = "rgba(255, 226, 138, 0.98)";
        ctx.fillText("★", seat.x + 0.5, seat.y + 0.5);
      }
    }

    // Province names, centred on each province's inscribed circle and sized to it.
    //
    // Three deliberate departures from how this used to work:
    //  · NO zoom gate. The old `currentScale > 0.7` meant that at the default world
    //    view not one province was named — the layer's whole point, missing.
    //  · NO shared collision cull. Each name is sized to fit inside its OWN province's
    //    inscribed circle, and those circles are disjoint by construction, so names
    //    cannot meaningfully overlap. The cull was silently dropping names that had
    //    every right to be drawn.
    //  · Font scales with the province, not with the zoom, so a great province reads
    //    like one and a sliver does not shout. A readable floor keeps small provinces
    //    legible; the only reason to skip a name now is that it genuinely does not fit.
    if (this.provinceLabels.length > 0) {
      ctx.textBaseline = "middle";   // with textAlign center → centred on both axes
      const st = labelStyles.province;
      for (const lb of this.provinceLabels) {
        const diamPx = 2 * lb.r * this.currentScale;
        // Proportional to the room available, clamped to a readable band.
        const fsPx = Math.min(34, Math.max(9, diamPx * 0.42));
        const fs = fsPx / this.currentScale;   // canvas is under a world transform
        const limit = diamPx * 1.6;
        // The province style is tracked CAPITALS, which run far wider than the raw
        // name — so rather than hiding a label the moment it overflows, shed the
        // width-costly treatments one at a time first. Without this, adopting the
        // typography would silently re-hide names that fit perfectly well before.
        // The degraded treatments are applied by temporarily relaxing the shared
        // style; `finally` guarantees it is put back even if a draw throws, so a
        // single bad label can never leave every province mis-styled.
        const saved = { caps: st.caps, tracking: st.tracking };
        try {
          let fits = false;
          for (const step of [0, 1, 2] as const) {
            if (step === 1) st.tracking = 0;                      // drop the tracking
            if (step === 2) { st.caps = false; st.tracking = 0; } // drop the capitals too
            if (this.measureLabel(ctx, "province", lb.name, fs) * this.currentScale <= limit) {
              fits = true; break;
            }
          }
          if (fits) {
            this.drawLabel(ctx, "province", lb.name, lb.x + 0.5, lb.y + 0.5, fs, "center", {
              halo: "rgba(6, 10, 16, 0.85)",
              haloWidth: Math.max(0.6, (fsPx * 0.2) / this.currentScale),
            });
          }
        } finally {
          st.caps = saved.caps; st.tracking = saved.tracking;
        }
      }
      ctx.textAlign = "left";
    }
  }

  render(ctx: CanvasRenderingContext2D) {
    // Fresh label-collision map each frame (settlement + toponym passes share it).
    this.placedLabels = [];
    // Provinces underlie everything (a base political/economic layer).
    if (this.visibility.provinces && this.provinceRaster) {
      this.renderProvinces(ctx);
    }
    // Trade-region territories first (under everything else) so markers/routes
    // stay legible on top.
    if (this.visibility.tradeRegions && this.econRegions.length > 0) {
      this.renderEconRegions(ctx);
    }

    if (this.visibility.lakes && this.lakes.length > 0) {
      const lhl = this.lakeHighlight;
      const hasLakeHL = lhl >= 0 && lhl < this.lakes.length;
      this.lakes.forEach((lake, li) => {
        const isSel = hasLakeHL && li === lhl;
        // Oxbow backwater · salt-lake brine tint · else open blue water. When a
        // lake is selected in the Hydrology panel, dim the others so it stands out.
        ctx.globalAlpha = hasLakeHL ? (isSel ? 1 : 0.28) : 1;
        ctx.fillStyle = lakeFill(lake);
        for (const [x, y] of lake.cells) ctx.fillRect(x, y, 1, 1);
        // SHEEN: a faint light-blue glint across the upper third of the basin so
        // open water reads as glossy/reflective rather than a flat blue slab.
        // Skip oxbows (weedy backwaters) — only open freshwater lakes glint.
        if (lake.kind !== 1 && lake.cells.length >= 4) {
          let minY = Infinity, maxY = -Infinity;
          for (const [, y] of lake.cells) { if (y < minY) minY = y; if (y > maxY) maxY = y; }
          const sheenCut = minY + (maxY - minY) * 0.34;
          ctx.fillStyle = "rgba(225, 245, 255, 0.28)";
          for (const [x, y] of lake.cells) if (y <= sheenCut) ctx.fillRect(x, y, 1, 1);
        }
        // Bright wash over the selected basin so it reads as picked (cheap — one
        // extra fill pass, no per-cell shadow). Kept lighter than before so a
        // picked lake still reads as vivid water, not a washed-out pale patch.
        if (isSel) {
          ctx.fillStyle = "rgba(210, 236, 255, 0.34)";
          for (const [x, y] of lake.cells) ctx.fillRect(x, y, 1, 1);
        }
      });
      ctx.globalAlpha = 1;
    }

    if (this.visibility.rivers && this.rivers.length > 0) {
      ctx.globalAlpha = 0.85;
      // Width and COLOUR shade track the river's discharge (set physically in
      // rivers.rs from precipitation × drainage area × climate): a small headwater
      // stream is thin and pale, a great trunk river wide and deep blue. Width is
      // zoom-compensated so even small streams stay visible.
      const inv = 1 / Math.sqrt(this.currentScale);
      ctx.lineJoin = "round";
      ctx.lineCap = "round";
      // Hydrology selection: when a river system is picked, its subtree glows and
      // every other river dims so the chosen network stands out on the map.
      const hl = this.riverHighlight;
      const hasHL = hl.size > 0;
      this.rivers.forEach((river, i) => {
        if (river.points.length < 2) return;
        const isHL = hasHL && hl.has(i);
        // Width scales with the river's Strahler order (headwater creek → great
        // trunk): a creek ≈ 0.55, a high-order trunk ≈ 1.3, zoom-compensated. Kept
        // deliberately thin — earlier bands still read as fat ribbons on close zoom.
        const ord = river.order ?? (river.major ? 4 : 1);
        const baseW = 0.4 + Math.min(ord, 6) * 0.15;
        const riverW = Math.max(0.4, Math.min(1.5, baseW) * inv);
        ctx.globalAlpha = hasHL ? (isHL ? 0.95 : 0.22) : 0.85;
        ctx.strokeStyle = riverShade(river.major);
        ctx.lineWidth = riverW;
        // Braided anabranches first (faint, thin), so the main stem draws over them
        // and they read as side-channels splitting around sandbar islands.
        if (river.braids && river.braids.length > 0) {
          ctx.save();
          ctx.globalAlpha = (hasHL ? (isHL ? 0.6 : 0.14) : 0.5);
          ctx.lineWidth = Math.max(0.6, riverW * 0.55);
          for (const strand of river.braids) {
            if (strand.length >= 2) strokeSmoothPath(ctx, strand);
          }
          ctx.restore();
        }
        // Meander + Catmull-Rom smoothing so the drainage lines read as natural
        // winding channels rather than the straight diagonal grid-lines the
        // steepest-descent flow produces on flats.
        strokeSmoothPath(ctx, this.riverPath(river, i, ord));
        // Delta: braided distributary fan + marsh stipple over the shallow shelf.
        if (river.mouth_kind === 1 && river.delta && river.delta.length > 0) {
          const [mx, my] = river.points[river.points.length - 1];
          ctx.fillStyle = "rgba(70,170,200,0.45)";
          for (const [dx, dy] of river.delta) {
            ctx.fillRect(dx, dy, 1, 1);
          }
          ctx.strokeStyle = "rgba(90,180,210,0.7)";
          ctx.lineWidth = Math.max(0.4, river.width * 0.3 * inv);
          for (const [dx, dy] of river.delta) {
            ctx.beginPath();
            ctx.moveTo(mx + 0.5, my + 0.5);
            ctx.lineTo(dx + 0.5, dy + 0.5);
            ctx.stroke();
          }
        }
      });
      // Glow pass over the highlighted subtree (drawn last → on top). Each
      // tributary can glow its own colour (branch / order scheme from the panel).
      if (hasHL) {
        ctx.save();
        ctx.globalAlpha = 0.95;
        ctx.shadowBlur = 8 * inv;
        this.rivers.forEach((river, i) => {
          if (!hl.has(i) || river.points.length < 2) return;
          const col = this.riverHighlightColors[i] ?? "#cdeeff";
          ctx.strokeStyle = col;
          ctx.shadowColor = col;
          const ord = river.order ?? (river.major ? 4 : 1);
          ctx.lineWidth = Math.max(1.1, Math.min(3.4, 1.2 + Math.min(ord, 6) * 0.34) * inv);
          strokeSmoothPath(ctx, this.riverPath(river, i, ord));
        });
        ctx.restore();
      }
      ctx.globalAlpha = 1;
    }

    if (this.visibility.currents && this.currentLines.length > 0) {
      for (const line of this.currentLines) {
        this.renderStreamline(ctx, line);
      }
    }

    if (this.visibility.wind && this.windData && this.windData.samples.length > 0) {
      for (const v of this.windData.samples) {
        if (v.vx === 0 && v.vy === 0) continue;
        this.renderArrow(ctx, v.x, v.y, v.vx, v.vy, WIND_COLOR, 0.5);
      }
    }

    if (this.visibility.latLines && this.latLinesData) {
      const { gridW, gridH, equatorOffset, latScale, lineRatio } = this.latLinesData;
      ctx.globalAlpha = 0.5;
      ctx.strokeStyle = LAT_LINE_COLOR;
      ctx.lineWidth = 0.5;
      // Lines drawn at the WIND-BELT boundaries (0°/±30°/±60°) so they mark
      // where the prevailing winds actually change — trade winds (0–30°),
      // westerlies (30–60°), polar easterlies (60–90°) — rather than the
      // tropics/polar circles (23.5°/66.5°), which don't coincide with the
      // belts. The ±90° poles are included so the user can see where they fall
      // (off-canvas when the latitude scale is expanded = cropped).
      const lines = [
        { lat: 0, label: "Equator (ITCZ)" },
        { lat: 30, label: "Trades / Westerlies" },
        { lat: -30, label: "Trades / Westerlies" },
        { lat: 60, label: "Westerlies / Polar" },
        { lat: -60, label: "Westerlies / Polar" },
        { lat: 90, label: "N Pole" },
        { lat: -90, label: "S Pole" },
      ];
      for (const { lat, label } of lines) {
        // Line position from the spacing ratio (1 = even … higher = poles fan
        // out). The map raster is never touched — only these reference lines move.
        const y = latLineY(lat, gridH, equatorOffset, latScale, lineRatio);
        // Crop: skip lines pushed off the canvas (a high ratio fans the polar
        // lines past the edge — correct for a pole-stretched chart).
        if (y < 0 || y > gridH) continue;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(gridW, y);
        ctx.stroke();

        const fontSize = Math.max(9, 9 / this.currentScale);
        ctx.font = `${fontSize}px sans-serif`;
        ctx.fillStyle = LAT_LINE_COLOR;
        ctx.fillText(label, 4, y + fontSize + 1);
      }
      ctx.globalAlpha = 1;
    }

    // ── Climate bands: circulation belts (subtropical high / polar front) and the
    // ITCZ rain line. All positioned in the SAME latitude framing as the lat lines,
    // and driven by the rotation-based Circulation model, so they shift when the
    // Planet panel changes the rotation/greenhouse. ──
    if ((this.visibility.windBelts || this.visibility.itcz) && this.climateBands && this.latLinesData) {
      const { gridW, gridH, equatorOffset, latScale, lineRatio } = this.latLinesData;
      const cb = this.climateBands;
      const fontSize = Math.max(9, 9 / this.currentScale);
      const lw = Math.max(0.5, 1.0 / this.currentScale);
      ctx.font = `${fontSize}px sans-serif`;

      // Horizontal belt line at a fixed latitude with a right-aligned label.
      const belt = (lat: number, color: string, label: string, dash: number[]) => {
        const y = latLineY(lat, gridH, equatorOffset, latScale, lineRatio);
        if (y < 0 || y > gridH) return;
        ctx.strokeStyle = color;
        ctx.lineWidth = lw;
        ctx.setLineDash(dash.map((d) => d / this.currentScale));
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(gridW, y);
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.fillStyle = color;
        ctx.fillText(label, 4, y - 2);
      };

      if (this.visibility.windBelts) {
        ctx.globalAlpha = 0.75;
        const he = cb.hadley_edge, pf = cb.polar_front;
        // Subtropical highs (the dry / desert belts) — warm amber.
        belt(he, SUBTROPICAL_COLOR, `Subtropical High ${he.toFixed(0)}° (dry)`, [6, 4]);
        belt(-he, SUBTROPICAL_COLOR, `Subtropical High ${he.toFixed(0)}° (dry)`, [6, 4]);
        // Polar fronts (the storm tracks) — cool blue.
        belt(pf, POLAR_FRONT_COLOR, `Polar Front ${pf.toFixed(0)}° (storms)`, [6, 4]);
        belt(-pf, POLAR_FRONT_COLOR, `Polar Front ${pf.toFixed(0)}° (storms)`, [6, 4]);
        ctx.globalAlpha = 1;
      }

      // ITCZ — the convergence / heavy-rain line, drawn at BOTH seasonal extremes
      // with the migration belt shaded between them. Each line is per-column, so it
      // bows poleward over the continents and equatorward over the oceans; the band
      // between them is the land that changes circulation regime between January
      // and July, which is the definition of a monsoon climate.
      if (this.visibility.itcz && cb.itcz.length > 0) {
        const step = Math.max(1, Math.floor(cb.width / 720));
        const yAt = (latDeg: number) =>
          latLineY(latDeg, gridH, equatorOffset, latScale, lineRatio);
        const hasSeasons =
          cb.itcz_july?.length === cb.width && cb.itcz_january?.length === cb.width;
        // Fall back to the single annual line on a world saved before the seasonal
        // ITCZ existed, so an old save still draws something correct.
        const julLats = hasSeasons ? cb.itcz_july : cb.itcz;
        const janLats = hasSeasons ? cb.itcz_january : cb.itcz;

        const trace = (lats: number[]) => {
          ctx.beginPath();
          let started = false;
          for (let x = 0; x < cb.width; x += step) {
            const y = yAt(lats[x]);
            if (!started) { ctx.moveTo(x + 0.5, y); started = true; }
            else { ctx.lineTo(x + 0.5, y); }
          }
          ctx.stroke();
        };

        if (hasSeasons) {
          // The migration band: a low-opacity fill between the two lines, hatched
          // with diagonal dashes so it reads as "this belt sweeps" rather than as a
          // solid climate region of its own.
          ctx.save();
          ctx.beginPath();
          let started = false;
          for (let x = 0; x < cb.width; x += step) {
            const y = yAt(julLats[x]);
            if (!started) { ctx.moveTo(x + 0.5, y); started = true; }
            else { ctx.lineTo(x + 0.5, y); }
          }
          for (let x = cb.width - 1 - ((cb.width - 1) % step); x >= 0; x -= step) {
            ctx.lineTo(x + 0.5, yAt(janLats[x]));
          }
          ctx.closePath();
          ctx.globalAlpha = 0.10;
          ctx.fillStyle = ITCZ_COLOR;
          ctx.fill();
          // Diagonal hatch inside the band only.
          ctx.clip();
          ctx.globalAlpha = 0.22;
          ctx.strokeStyle = ITCZ_COLOR;
          ctx.lineWidth = Math.max(0.4, 0.9 / this.currentScale);
          const hatch = Math.max(6, 14 / Math.sqrt(this.currentScale));
          const yTop = yAt(90), yBot = yAt(-90);
          const span = Math.abs(yBot - yTop) + cb.width;
          ctx.beginPath();
          for (let d = -span; d < cb.width + span; d += hatch) {
            ctx.moveTo(d, Math.min(yTop, yBot));
            ctx.lineTo(d + Math.abs(yBot - yTop), Math.max(yTop, yBot));
          }
          ctx.stroke();
          ctx.restore();
        }

        // July line — solid; January — dashed, so the pair reads without colour.
        ctx.globalAlpha = 0.92;
        ctx.strokeStyle = ITCZ_COLOR;
        ctx.lineWidth = Math.max(0.8, 1.8 / this.currentScale);
        ctx.setLineDash([]);
        trace(julLats);
        if (hasSeasons) {
          ctx.setLineDash([
            Math.max(3, 7 / Math.sqrt(this.currentScale)),
            Math.max(2, 5 / Math.sqrt(this.currentScale)),
          ]);
          trace(janLats);
          ctx.setLineDash([]);
        }

        ctx.globalAlpha = 1;
        ctx.fillStyle = ITCZ_COLOR;
        if (hasSeasons) {
          // Plain fillText, like the other band annotations: these are overlay
          // legends, not place names, so they stay out of the label registry
          // (§8.11) exactly as road names and river-break markers do.
          ctx.fillText("ITCZ July (summer rains)", 4, yAt(julLats[0]) - 2);
          ctx.fillText("ITCZ January", 4, yAt(janLats[0]) - 2);
        } else {
          ctx.fillText("ITCZ (convergence / rains)", 4, yAt(cb.itcz[0]) - 2);
        }
      }
    }

    // Fishery grand banks: large translucent teal discs over rich grounds.
    // Drawn before settlements/routes so dots and lines stay legible on top.
    if (this.visibility.fisheryBanks && this.fisheryBanks.length > 0) {
      for (const bank of this.fisheryBanks) {
        const alpha = 0.10 + 0.22 * Math.min(1, bank.score);
        ctx.beginPath();
        ctx.arc(bank.x + 0.5, bank.y + 0.5, bank.radius, 0, Math.PI * 2);
        ctx.fillStyle = FISHERY_BANK;
        ctx.globalAlpha = alpha;
        ctx.fill();
        ctx.globalAlpha = Math.min(0.8, alpha + 0.35);
        ctx.strokeStyle = FISHERY_BANK;
        ctx.lineWidth = Math.max(0.5, 1.2 / Math.sqrt(this.currentScale));
        ctx.setLineDash([Math.max(2, 5 / Math.sqrt(this.currentScale)), Math.max(2, 4 / Math.sqrt(this.currentScale))]);
        ctx.stroke();
        ctx.setLineDash([]);
      }
      ctx.globalAlpha = 1;
    }

    // Peoples / culture territories: each hearth's land tinted in its colour with
    // the people name at the centroid. Drawn first so belts/hazards sit on top.
    if (this.visibility.cultures && this.cultureRegions.length > 0) {
      for (const r of this.cultureRegions) {
        const [cr, cg, cb] = r.color;
        this.renderRegionMask(ctx, r.cells, r.cell_size, `rgb(${cr},${cg},${cb})`, "", r.x, r.y, 0.22);
      }
      // People names at each territory's centroid (drawn after fills so labels
      // aren't covered by a neighbouring region's tint).
      const fs = Math.max(7, 13 / this.currentScale);
      ctx.textBaseline = "middle";
      for (const r of this.cultureRegions) {
        this.drawLabel(ctx, "peopleTerritory", r.label, r.x, r.y, fs, "center",
          { halo: "rgba(0,0,0,0.75)", haloWidth: Math.max(0.6, 2.4 / this.currentScale) });
      }
      ctx.textAlign = "start";
      ctx.textBaseline = "alphabetic";
    }

    // States (§3.3): a tier 1-2 city's own writ, tinted distinctly from any house's
    // heraldic colour so a state's territory never reads as a merchant house's
    // sphere. Drawn after peoples/before belts — a political claim over land the
    // way `cultureRegions` reads an ethnic one.
    if (this.visibility.states && this.stateRegions.length > 0) {
      if (this.statesDirty) this.buildStateRender();
      if (this.stateCanvas && this.provinceRaster) {
        // Exact fill: blit the province-raster-resolution tint (each pixel
        // composited once, so a semi-transparent blit shows no cell grid), then a
        // heavier border stroke ALONG the province edges — the state outline IS the
        // province outline.
        const { w, h, gridW, gridH } = this.provinceRaster;
        ctx.imageSmoothingEnabled = false;
        ctx.globalAlpha = 0.30;
        ctx.drawImage(this.stateCanvas, 0, 0, w, h, 0, 0, gridW, gridH);
        ctx.globalAlpha = 1;
        if (this.stateBorderPath) {
          ctx.strokeStyle = "rgba(10, 14, 22, 0.7)";
          ctx.lineWidth = 2.2 / this.currentScale;
          ctx.stroke(this.stateBorderPath);
        }
      } else {
        // Fallback (province raster not loaded yet): the old cell-cloud tint, so a
        // state still shows rather than vanishing until the raster arrives.
        for (const r of this.stateRegions) {
          const [sr, sg, sb] = r.color;
          this.renderRegionMask(ctx, r.cells, r.cell_size, `rgb(${sr},${sg},${sb})`, "", r.x, r.y, 0.20);
        }
      }
      const fs2 = Math.max(7, 13 / this.currentScale);
      ctx.textBaseline = "middle";
      for (const r of this.stateRegions) {
        this.drawLabel(ctx, "state", r.name, r.x, r.y, fs2, "center",
          { halo: "rgba(0,0,0,0.75)", haloWidth: Math.max(0.6, 2.4 / this.currentScale) });
      }
      ctx.textAlign = "start";
      ctx.textBaseline = "alphabetic";
    }

    // Trade-good belts: the actual physics-driven cells, filled + outlined, with
    // the good's emoji at the centroid (per-good toggle).
    if (this.goodRegions.length > 0) {
      for (const r of this.goodRegions) {
        if (!this.visibility[goodOverlayKey(r.good)]) continue;
        const def = GOOD_BY_NAME.get(r.good);
        const m = this.goodMeta?.get(r.good);
        const color = m?.color ?? def?.color ?? "#cccccc";
        const emoji = m?.icon ?? def?.emoji ?? "";
        // Multi-type goods (grain / paper) tint by per-cell subtype.
        const sub = goodSubtypes(r.good);
        const subtypes = sub && r.subtypes.length === r.cells.length ? r.subtypes : undefined;
        this.renderRegionMask(ctx, r.cells, r.cell_size, color, emoji, r.x, r.y, 0.16 + 0.18 * Math.min(1, r.score), r.sublabel, r.values, subtypes, sub ?? undefined, r.good);
      }
    }

    // Shark-infested water: the highest-risk habitat cells + a shark glyph.
    if (this.visibility.sharkZones && this.sharkZones.length > 0) {
      for (const z of this.sharkZones) {
        this.renderRegionMask(ctx, z.cells, z.cell_size, SHARK_COLOR, "\u{1F988}", z.x, z.y, 0.16 + 0.22 * Math.min(1, z.score));
      }
    }

    // Shipworm hull-hazard water: warm brackish coasts marked + a worm glyph.
    if (this.visibility.shipwormZones && this.shipwormZones.length > 0) {
      for (const z of this.shipwormZones) {
        this.renderRegionMask(ctx, z.cells, z.cell_size, SHIPWORM_COLOR, "\u{1FAB1}", z.x, z.y, 0.16 + 0.22 * Math.min(1, z.score));
      }
    }

    // Storm/cyclone belts: open-ocean danger water marked + a cyclone glyph.
    if (this.visibility.stormZones && this.stormZones.length > 0) {
      for (const z of this.stormZones) {
        this.renderRegionMask(ctx, z.cells, z.cell_size, STORM_COLOR, "\u{1F300}", z.x, z.y, 0.16 + 0.22 * Math.min(1, z.score));
      }
    }

    // Monsoon-climate land: the seasonal wet-season flood belt marked + a rain
    // glyph (a natural-disaster sibling of the cyclone zones, on land not sea).
    if (this.visibility.monsoonZones && this.monsoonZones.length > 0) {
      for (const z of this.monsoonZones) {
        this.renderRegionMask(ctx, z.cells, z.cell_size, MONSOON_COLOR, "\u{1F327}", z.x, z.y, 0.14 + 0.20 * Math.min(1, z.score));
      }
    }

    // Reef/shoal wreck hazards: warm shallow coastal water marked + a rock glyph.
    if (this.visibility.reefZones && this.reefZones.length > 0) {
      for (const z of this.reefZones) {
        this.renderRegionMask(ctx, z.cells, z.cell_size, REEF_COLOR, "\u{1FAA8}", z.x, z.y, 0.16 + 0.22 * Math.min(1, z.score));
      }
    }

    // Trade flows: bundled commodity trunks routed over the trade network
    // (width ∝ total volume on each corridor).
    if (this.visibility.tradeFlows && this.tradeTrunks.length > 0) {
      this.renderTradeTrunks(ctx);
    }
    if (this.visibility.dynamicFlow && this.dynamicTrunks.length > 0) {
      this.renderDynamicFlow(ctx);
    }
    if (this.visibility.campaignCorridors && this.tradeCorridorList.length > 0) {
      this.renderTradeCorridors(ctx);
    }
    if (this.visibility.expeditions && (this.expeditions.length > 0 || this.expeditionFails.length > 0)) {
      this.renderExpeditions(ctx);
    }

    if (this.visibility.tradeRoutes && this.tradeRoutes.length > 0) {
      for (const route of this.tradeRoutes) {
        this.renderTradeRoute(ctx, route);
      }
    }

    // #23 · the chosen itinerary route — a bright magenta thread with endpoint
    // pins, drawn over the trade network so the journey stands out.
    if (this.visibility.travelRoute && this.travelRoute.length >= 2) {
      this.renderTravelRoute(ctx);
    }

    // Ridge-drawing tool: sketch the user's drawn ridge lines (width ∝ footprint,
    // opacity ∝ peak height). Always shown while lines exist (no visibility gate).
    if (this.ridgeSketch.length > 0) {
      this.renderRidgeSketch(ctx);
    }

    // #37 · per-good scarcity: graduated discs at each hub, green where the good
    // is cheap/abundant through to red where it is dear/scarce.
    if (this.visibility.goodScarcity && this.goodScarcity.length > 0) {
      this.renderGoodScarcity(ctx);
    }

    // #26 · geographic toponyms: culture-styled labels for rivers/peaks/lakes/regions.
    if (this.visibility.toponyms && this.toponyms.length > 0) {
      this.renderToponyms(ctx);
    }

    // 🌊 Reach breaks: where a trunk river turns upper→middle→delta.
    if (this.visibility.riverBreaks !== false && this.riverBreaks.length > 0) {
      this.renderRiverBreaks(ctx);
    }

    // Merchant layer: live family/guild routes coloured by the owning house.
    if (this.visibility.merchantRoutes && this.merchantRoutes.length > 0) {
      this.renderMerchantRoutes(ctx);
    }

    // Futures layer: contractual supply lanes (source → buyer), dashed + directed.
    if (this.visibility.futures && this.futuresLanes.length > 0) {
      this.renderFutures(ctx);
    }

    // Political influence: translucent discs sized by trade power.
    if (this.visibility.politicalInfluence && this.politicalCenters.length > 0) {
      // Trade posts (outposts) shown on the trade-hub layer as small black dots,
      // a distinct class below the blue hubs / red emporia / golden capital.
      const dotR = Math.max(0.6, 1.3 / Math.sqrt(this.currentScale));
      ctx.fillStyle = "#0a0a0a";
      for (const s of this.settlements) {
        if (s.size !== "outpost") continue;
        ctx.beginPath();
        ctx.arc(s.x + 0.5, s.y + 0.5, dotR, 0, Math.PI * 2);
        ctx.fill();
      }
      for (const c of this.politicalCenters) this.renderPoliticalCenter(ctx, c);
    }

    // DLC 3 · speculation risk: translucent discs sized by bubble risk, coloured
    // green→amber→red by tier (rendered above the trade-hub markers).
    if (this.visibility.speculation && this.specCenters.length > 0) {
      for (const c of this.specCenters) this.renderSpecCenter(ctx, c);
    }

    // Merchant-family control: settlements a house dominates (>=50% of local
    // trade) and the trade routes it runs are tinted that house's unique colour;
    // every other settlement is a small grey dot and every other route is grey.
    if (this.visibility.houseControl && this.settlements.length > 0) {
      this.renderHouseControlLayer(ctx);
    }

    // Monetary-dominance map: tint EVERY settlement by the coin it settles in.
    if (this.visibility.coinDominance && this.coinUse.length > 0) {
      this.renderCoinDominance(ctx);
    }
    // Coin-usage drill-down: a selected coin's territory (primary / held / reserve).
    if (this.coinOverlayHub != null && this.coinUse.length > 0) {
      this.renderCoinUsage(ctx);
    }

    // Bank seats — a gold disc + 🏦 so banks are easy to find on the map.
    if (this.bankIcons.length > 0) {
      this.renderBankIcons(ctx);
    }

    // Phase 6 · plague-struck cities (red glow) + contagion routes.
    if (this.visibility.plagueZones && this.plagueCities.length > 0) {
      this.renderPlagueZones(ctx);
    }
    // Phase 6 · guild cities marked with their good's emoji.
    if (this.visibility.guildCities && this.guildCities.length > 0) {
      this.renderGuildCities(ctx);
    }
    // Phase 6 · living notable figures.
    if (this.visibility.figureMarks && this.figureMarks.length > 0) {
      this.renderEmojiMarks(ctx, this.figureMarks, "#9070c0");
    }
    // Phase 6 · landmarks & sacred sites.
    if (this.visibility.landmarks && this.landmarkMarks.length > 0) {
      this.renderEmojiMarks(ctx, this.landmarkMarks, "#40b090");
    }
    // Phase 7 · dynasty ties (alliances gold, feuds red) between seat cities.
    if (this.visibility.dynastyLinks && this.dynastyLinks.length > 0) {
      this.renderDynastyLinks(ctx);
    }

    // Trade ▸ Flows highlight (always on top when set by the settlement panel).
    if (this.flowHighlight.length > 0) {
      this.renderFlowHighlight(ctx);
    }

    // Directional trade corridors: one net-direction arrow per hub→hub corridor
    // (so direction only flips at hubs), width ∝ total value carried.
    if (this.visibility.tradeCorridors && this.corridors.length > 0) {
      this.renderCorridors(ctx);
    }

    // Strategic chokepoints: high-volume trade gateways (straits / passes).
    if (this.visibility.chokepoints && this.chokepoints.length > 0) {
      for (const cp of this.chokepoints) this.renderChokepoint(ctx, cp);
    }

    // Selected supply-chain road (origin → hub stops → here) with per-hop price.
    if (this.supplyChain && this.supplyChain.stops.length > 0) {
      this.renderSupplyChain(ctx, this.supplyChain);
    }

    // Per-good reach network: every route carrying the selected good + a ring on
    // each hub it reaches.
    if (this.reachChains.length > 0) {
      this.renderReachNetwork(ctx);
    }

    // Atlas 2.0 · NAMED TRADE BASINS — dashed hulls + region labels, drawn first
    // so heat, routes and markers sit on top.
    if (this.visibility.tradeBasins && this.basins.length > 0) {
      this.renderTradeBasins(ctx);
    }

    // Atlas 2.0 · TRADE HEAT — where trade concentrates. Soft additive glows per
    // hub, radius + colour ∝ last year's throughput (teal → gold → crimson).
    // Drawn UNDER the settlement markers so the dots stay crisp on the glow.
    if (this.visibility.tradeHeat && this.heatPoints.length > 0) {
      this.renderTradeHeat(ctx);
    }

    // Atlas 2.0 · MIGRATION — route-bound flows (dots/ribbon/focus) when present,
    // else the legacy fading refugee-road arrows.
    if (this.visibility.migrations !== false) {
      if (this.migrationRoutes.length > 0) this.renderMigrationRoutes(ctx);
      else if (this.migrations.length > 0) this.renderMigrations(ctx);
    }

    if (this.visibility.settlements && this.settlements.length > 0) {
      for (const s of this.settlements) {
        // A DEAD (abandoned/collapsed) city is a † ruin: a dark cross on a faint
        // parchment halo, still on the map so the loss stays visible forever.
        if (s.dead) {
          const dinv = 1 / Math.sqrt(this.currentScale);
          const r = Math.max(1.0, 1.8 * dinv);
          const cx = s.x + 0.5, cy = s.y + 0.5;
          // Halo first so the cross reads on dark terrain too.
          ctx.strokeStyle = "rgba(232,217,176,0.55)";
          ctx.lineWidth = Math.max(1.1, 2.2 * dinv);
          ctx.beginPath();
          ctx.moveTo(cx, cy - r * 1.15); ctx.lineTo(cx, cy + r);
          ctx.moveTo(cx - r * 0.75, cy - r * 0.45); ctx.lineTo(cx + r * 0.75, cy - r * 0.45);
          ctx.stroke();
          // The † itself.
          ctx.strokeStyle = "#0d0d0d";
          ctx.lineWidth = Math.max(0.5, 1.0 * dinv);
          ctx.beginPath();
          ctx.moveTo(cx, cy - r * 1.15); ctx.lineTo(cx, cy + r);
          ctx.moveTo(cx - r * 0.75, cy - r * 0.45); ctx.lineTo(cx + r * 0.75, cy - r * 0.45);
          ctx.stroke();
          continue;
        }
        // Dot scales continuously with population (log) on top of the tier base,
        // so the emergent carrying-capacity / trade hierarchy reads on the map.
        // Widened range (≈0.7–2.2×): on the humble campaign scale a 10k city must
        // visibly dwarf a 500-pop village (the old 0.6+log/5 curve compressed
        // everything into ~1.1–1.7×). Combined with the tier base this gives a
        // strong, legible size hierarchy that tracks growth/decline live.
        const popf = Math.min(2.2, Math.max(0.7, 0.45 + Math.log10(Math.max(s.population, 50)) * 0.32));

        // Marker system by POPULATION + type (user rule):
        //   outpost           → small GREY square
        //   village (<1.5k)   → small WHITE square
        //   town/city ≤20k    → WHITE circle (grows with pop)
        //   20k–100k          → ORANGE circle (larger)
        //   ≥100k             → ORANGE circle + ★ star inside
        // Squares are kept small; circles carry the growth hierarchy. Dead cities
        // render as a † cross above.
        const pop = s.population;
        const isOutpost = s.size === "outpost";
        const isVillage = !isOutpost && pop < 1_500;
        const isSquare = isOutpost || isVillage;
        const cx = s.x + 0.5, cy = s.y + 0.5;
        ctx.globalAlpha = 0.95;
        ctx.strokeStyle = "rgba(0,0,0,0.65)";
        let radius: number;
        if (isSquare) {
          // Small squares — outposts a touch smaller than villages.
          radius = (isOutpost ? 0.7 : 0.9) * popf;
          ctx.fillStyle = isOutpost ? "#9aa7b4" : "#f0f0f0"; // grey outpost · white village
          ctx.lineWidth = Math.max(0.2, radius * 0.16);
          const hw = radius * 0.72; // smaller squares than before
          ctx.beginPath();
          ctx.rect(cx - hw, cy - hw, hw * 2, hw * 2);
          ctx.fill();
          ctx.stroke();
        } else {
          const orange = pop >= 20_000;
          // Circles kept small (user: "way smaller") — population still drives the
          // size hierarchy via popf, but the base is tightened so even a metropolis
          // reads as a compact dot rather than a blob.
          radius = (orange ? 1.05 : 0.85) * popf;
          ctx.fillStyle = orange ? "#ff8a3c" : "#f0f0f0"; // orange ≥20k · white town/city
          ctx.lineWidth = Math.max(0.25, radius * 0.14);
          ctx.beginPath();
          ctx.arc(cx, cy, radius, 0, Math.PI * 2);
          ctx.fill();
          ctx.stroke();
          if (pop >= 100_000) {
            // Metropolis: a dark ★ set inside the orange disc.
            ctx.fillStyle = "rgba(26,18,6,0.9)";
            ctx.beginPath();
            for (let i = 0; i < 10; i++) {
              const rr = i % 2 === 0 ? radius * 0.62 : radius * 0.26;
              const a = -Math.PI / 2 + (i * Math.PI) / 5;
              const px = cx + Math.cos(a) * rr, py = cy + Math.sin(a) * rr;
              if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
            }
            ctx.closePath();
            ctx.fill();
          }
        }
        // Dynamically-earned commercial rank (campaign, re-ranked twice a year): a
        // TRADE HUB wears a small blue diamond, an ENTREPÔT a red triangle — a distinct
        // shape set above the population dot, so "how big" and "how commercial" read
        // together. Rises and falls with the trade that actually flows through.
        const hc = s.hubClass ?? 0;
        if (hc >= 1) {
          const dinv = 1 / Math.sqrt(this.currentScale);
          const mr = Math.max(radius * 0.85, 1.3 * dinv);
          const mx = cx, my = cy - radius - mr * 1.15;
          ctx.lineWidth = Math.max(0.2, 0.35 * dinv);
          ctx.strokeStyle = "rgba(0,0,0,0.7)";
          if (hc >= 2) {
            ctx.fillStyle = "#e63030"; // entrepôt — red triangle
            ctx.beginPath();
            ctx.moveTo(mx, my - mr);
            ctx.lineTo(mx - mr * 0.9, my + mr * 0.7);
            ctx.lineTo(mx + mr * 0.9, my + mr * 0.7);
            ctx.closePath();
            ctx.fill();
            ctx.stroke();
          } else {
            ctx.fillStyle = "#3a86d6"; // trade hub — blue diamond
            ctx.beginPath();
            ctx.moveTo(mx, my - mr);
            ctx.lineTo(mx - mr * 0.8, my);
            ctx.lineTo(mx, my + mr);
            ctx.lineTo(mx + mr * 0.8, my);
            ctx.closePath();
            ctx.fill();
            ctx.stroke();
          }
        }
        // Atlas 2.0 · a settlement founded THIS campaign wears a gold founding
        // star for its first years — new towns pop out at a glance.
        if (s.isNew) {
          const dinv = 1 / Math.sqrt(this.currentScale);
          const sr = Math.max(radius * 0.9, 1.4 * dinv);
          const sx = cx + radius + sr * 0.6, sy = cy - radius - sr * 0.4;
          ctx.fillStyle = "#ffd75e";
          ctx.strokeStyle = "rgba(0,0,0,0.6)";
          ctx.lineWidth = Math.max(0.2, 0.35 * dinv);
          ctx.beginPath();
          for (let i = 0; i < 8; i++) {
            const rr = i % 2 === 0 ? sr : sr * 0.38;
            const a = -Math.PI / 2 + (i * Math.PI) / 4;
            const px = sx + Math.cos(a) * rr, py = sy + Math.sin(a) * rr;
            if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
          }
          ctx.closePath();
          ctx.fill();
          ctx.stroke();
        }
        ctx.globalAlpha = 1;
      }
    }

    // #1/#23 · Culture-share overlay: for the isolated people, each settlement gets
    // a coloured HALO ring (thickness by share tier) + an inner FILL disc whose area
    // grows with the share — 75%+ solid, 45-74 half, 20-44 quarter, 5-19 ring only.
    if (this.cultureShares.length > 0) {
      const inv = 1 / Math.sqrt(this.currentScale);
      const [cr, cg, cb] = this.cultureColor;
      const colour = `rgb(${cr},${cg},${cb})`;
      const r = Math.max(2.0, 2.8 * inv);
      for (const p of this.cultureShares) {
        const s = p.share;
        const cx = p.x + 0.5, cy = p.y + 0.5;
        // A WHITE circle background with an inner circle in the people's COLOUR, sized
        // by share: 75%+ ≈ solid · 45-74 majority · 20-44 significant · 5-19 minority dot.
        ctx.fillStyle = "rgba(250,250,252,0.92)";
        ctx.strokeStyle = "rgba(0,0,0,0.45)";
        ctx.lineWidth = Math.max(0.2, 0.4 * inv);
        ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
        const frac = s >= 0.75 ? 0.98 : s >= 0.45 ? 0.68 : s >= 0.20 ? 0.42 : 0.22;
        ctx.fillStyle = colour;
        ctx.beginPath(); ctx.arc(cx, cy, r * frac, 0, Math.PI * 2); ctx.fill();
      }
    }

    // Colony/satellite ↔ metropolis link: a glowing dashed tie with a ring at each
    // end (the metropolis larger), set when a row is clicked in the Colonial panel.
    if (this.colonyLink && this.colonyLink.ax >= 0 && this.colonyLink.bx >= 0) {
      const inv = 1 / Math.sqrt(this.currentScale);
      const { ax, ay, bx, by } = this.colonyLink;
      ctx.save();
      ctx.strokeStyle = "rgba(255,222,120,0.9)";
      ctx.lineWidth = Math.max(0.6, 1.5 * inv);
      ctx.setLineDash([Math.max(1.5, 3 * inv), Math.max(1, 2 * inv)]);
      ctx.beginPath(); ctx.moveTo(ax + 0.5, ay + 0.5); ctx.lineTo(bx + 0.5, by + 0.5); ctx.stroke();
      ctx.setLineDash([]);
      for (const [x, y, rr] of [[ax, ay, 3.4] as const, [bx, by, 2.6] as const]) {
        ctx.strokeStyle = "rgba(255,222,120,0.95)";
        ctx.lineWidth = Math.max(0.5, 1.0 * inv);
        ctx.beginPath(); ctx.arc(x + 0.5, y + 0.5, Math.max(1.6, rr * inv), 0, Math.PI * 2); ctx.stroke();
      }
      ctx.restore();
    }

    // Hover "shine": a bright pulsing double ring around the settlement under the
    // cursor, so it's obvious which one a click will select.
    if (this.hoverPoint) {
      const inv = 1 / Math.sqrt(this.currentScale);
      const cx = this.hoverPoint.x + 0.5, cy = this.hoverPoint.y + 0.5;
      const r = Math.max(2.4, 3.6 * inv);
      ctx.save();
      ctx.strokeStyle = "rgba(255,236,150,0.95)";
      ctx.lineWidth = Math.max(0.5, 0.9 * inv);
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.stroke();
      ctx.strokeStyle = "rgba(255,236,150,0.30)";
      ctx.lineWidth = Math.max(1.0, 2.2 * inv);
      ctx.beginPath(); ctx.arc(cx, cy, r * 1.55, 0, Math.PI * 2); ctx.stroke();
      ctx.restore();
    }

    // Colonies & house trade outposts (their own markers + routed supply lanes).
    if (this.visibility.colonies && this.colonies.length > 0) {
      this.renderColonies(ctx);
    }

    // Name labels (opt-in overlays). Drawn last so they sit on top of markers.
    if (this.visibility.settlementNames && this.settlements.length > 0) {
      this.renderSettlementNames(ctx);
    }
    if (this.visibility.hubNames && this.politicalCenters.length > 0) {
      this.renderHubNames(ctx);
    }

    // Search highlight pin: a bright double ring + dot on the searched settlement,
    // drawn on top of everything (cleared by MapCanvas after a few seconds).
    if (this.searchPin) {
      const inv = 1 / Math.sqrt(this.currentScale);
      const cx = this.searchPin.wx + 0.5, cy = this.searchPin.wy + 0.5;
      ctx.save();
      ctx.strokeStyle = "#ffd86a";
      ctx.lineWidth = Math.max(0.7, 2 * inv);
      ctx.beginPath(); ctx.arc(cx, cy, 7 * inv, 0, Math.PI * 2); ctx.stroke();
      ctx.globalAlpha = 0.55;
      ctx.beginPath(); ctx.arc(cx, cy, 11 * inv, 0, Math.PI * 2); ctx.stroke();
      ctx.globalAlpha = 1;
      ctx.fillStyle = "#ffd86a";
      ctx.beginPath(); ctx.arc(cx, cy, Math.max(1, 1.6 * inv), 0, Math.PI * 2); ctx.fill();
      ctx.restore();
    }

    // War highlight: the two belligerent cities, ATTACKER red and DEFENDER blue,
    // joined by a clashing dashed line — "where the war is happening" (#6b).
    if (this.warHighlight) {
      const inv = 1 / Math.sqrt(this.currentScale);
      const { ax, ay, bx, by } = this.warHighlight;
      const acx = ax + 0.5, acy = ay + 0.5, bcx = bx + 0.5, bcy = by + 0.5;
      ctx.save();
      // The clash line between them.
      ctx.strokeStyle = "rgba(230,210,160,0.7)";
      ctx.lineWidth = Math.max(0.6, 1.6 * inv);
      ctx.setLineDash([Math.max(2, 4 * inv), Math.max(2, 3 * inv)]);
      ctx.beginPath(); ctx.moveTo(acx, acy); ctx.lineTo(bcx, bcy); ctx.stroke();
      ctx.setLineDash([]);
      const ring = (x: number, y: number, color: string) => {
        ctx.strokeStyle = color;
        ctx.lineWidth = Math.max(0.8, 2.4 * inv);
        ctx.beginPath(); ctx.arc(x, y, 8 * inv, 0, Math.PI * 2); ctx.stroke();
        ctx.globalAlpha = 0.5;
        ctx.beginPath(); ctx.arc(x, y, 12.5 * inv, 0, Math.PI * 2); ctx.stroke();
        ctx.globalAlpha = 1;
        ctx.fillStyle = color;
        ctx.beginPath(); ctx.arc(x, y, Math.max(1.2, 2 * inv), 0, Math.PI * 2); ctx.fill();
      };
      ring(acx, acy, "#ff5a4d");  // attacker
      ring(bcx, bcy, "#4d9bff");  // defender
      ctx.restore();
    }
  }

  /**
   * Draw one warm ocean current as a single continuous arrow: a smooth thick
   * polyline tracing the current from start to end, with periodic arrowheads
   * showing flow direction. This replaces the old per-cell arrow field.
   */
  private renderStreamline(ctx: CanvasRenderingContext2D, line: Streamline) {
    const pts = line.points;
    if (pts.length < 2) return;

    // Drop segments that span the wrap seam (cylindrical X) so a current near
    // the date line doesn't draw a horizontal line across the whole map.
    const color = line.ctype === 1 ? WARM_CURRENT
      : line.ctype === 2 ? COLD_CURRENT
      : NEUTRAL_CURRENT;
    // Neutral drift/equatorial lines drawn a touch thinner than boundary currents.
    const widthScale = line.ctype === 0 ? 1.8 : 2.4;
    const lineWidth = Math.max(0.7, widthScale / Math.sqrt(this.currentScale));

    ctx.globalAlpha = 0.85;
    ctx.strokeStyle = color;
    ctx.lineWidth = lineWidth;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    const seamGap = 20; // world cells; segments longer than this are wrap seams
    ctx.beginPath();
    let started = false;
    for (let i = 0; i < pts.length; i++) {
      const [x, y] = pts[i];
      if (i > 0) {
        const dx = Math.abs(x - pts[i - 1][0]);
        if (dx > seamGap) { started = false; } // break the path at the seam
      }
      if (!started) { ctx.moveTo(x + 0.5, y + 0.5); started = true; }
      else ctx.lineTo(x + 0.5, y + 0.5);
    }
    ctx.stroke();

    // Periodic arrowheads along the line (every ~spacing points).
    const headEvery = Math.max(6, Math.round(pts.length / 4));
    const headLen = Math.max(2, 7 / Math.sqrt(this.currentScale));
    ctx.fillStyle = color;
    for (let i = headEvery; i < pts.length; i += headEvery) {
      const [x, y] = pts[i];
      const [px, py] = pts[i - 1];
      let dx = x - px, dy = y - py;
      if (Math.abs(dx) > seamGap) continue; // skip seam
      const m = Math.sqrt(dx * dx + dy * dy);
      if (m < 0.001) continue;
      dx /= m; dy /= m;
      const ax = x + 0.5, ay = y + 0.5;
      const perpX = -dy, perpY = dx;
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      ctx.lineTo(ax - dx * headLen + perpX * headLen * 0.55, ay - dy * headLen + perpY * headLen * 0.55);
      ctx.lineTo(ax - dx * headLen - perpX * headLen * 0.55, ay - dy * headLen - perpY * headLen * 0.55);
      ctx.closePath();
      ctx.fill();
    }

    ctx.globalAlpha = 1;
  }

  /**
   * Draw one trade route as a dashed polyline. Maritime routes are pale cyan,
   * overland routes tan. The path is broken at the cylindrical wrap seam so a
   * route near the date line doesn't streak across the whole map.
   */
  private renderTradeRoute(ctx: CanvasRenderingContext2D, route: TradeRoute) {
    const pts = route.points;
    if (pts.length < 2) return;

    const color = route.kind === 1 ? lineColors.tradeSea : route.kind === 2 ? lineColors.tradeRiver : lineColors.tradeLand;
    // Minor connector roads (a lesser town's single link) are drawn thinner and
    // fainter than the major inter-hub routes, so every settlement is on the
    // network without the small roads overpowering the trunks.
    const lineWidth = Math.max(0.4, (route.minor ? 0.8 : 1.6) / Math.sqrt(this.currentScale));
    const dash = Math.max(1.5, 4 / Math.sqrt(this.currentScale));
    // Distinguish routes by line STYLE, not just colour (flat map redesign):
    // sea = long rhumb-line dashes · river = even dash · land = fine dotted caravan track.
    const pattern = route.kind === 1 ? [dash * 2.2, dash]
      : route.kind === 2 ? [dash, dash]
      : [Math.max(0.8, dash * 0.5), dash * 1.5];

    ctx.globalAlpha = route.minor ? 0.5 : 0.8;
    ctx.strokeStyle = color;
    ctx.lineWidth = lineWidth;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.setLineDash(pattern);

    const seamGap = 20;
    ctx.beginPath();
    let started = false;
    for (let i = 0; i < pts.length; i++) {
      const [x, y] = pts[i];
      if (i > 0 && Math.abs(x - pts[i - 1][0]) > seamGap) started = false;
      if (!started) { ctx.moveTo(x + 0.5, y + 0.5); started = true; }
      else ctx.lineTo(x + 0.5, y + 0.5);
    }
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.globalAlpha = 1;
  }

  /** #23 · the highlighted itinerary route: a bright thread plus origin/destination
   *  pins. Drawn in world cells; the antimeridian seam is split like trade routes. */
  private renderTravelRoute(ctx: CanvasRenderingContext2D) {
    const pts = this.travelRoute;
    const w = Math.max(0.9, 2.4 / Math.sqrt(this.currentScale));
    ctx.strokeStyle = "#e85bd0";
    ctx.lineWidth = w;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.globalAlpha = 0.95;
    const seamGap = 20;
    ctx.beginPath();
    let started = false;
    for (let i = 0; i < pts.length; i++) {
      const [x, y] = pts[i];
      if (i > 0 && Math.abs(x - pts[i - 1][0]) > seamGap) started = false;
      if (!started) { ctx.moveTo(x + 0.5, y + 0.5); started = true; }
      else ctx.lineTo(x + 0.5, y + 0.5);
    }
    ctx.stroke();
    // Endpoint pins (origin green, destination red).
    const r = Math.max(1.2, 3 / Math.sqrt(this.currentScale));
    const pin = (p: [number, number], fill: string) => {
      ctx.beginPath();
      ctx.arc(p[0] + 0.5, p[1] + 0.5, r, 0, Math.PI * 2);
      ctx.fillStyle = fill;
      ctx.fill();
      ctx.lineWidth = Math.max(0.4, r * 0.35);
      ctx.strokeStyle = "#0b1420";
      ctx.stroke();
    };
    pin(pts[0], "#46d07a");
    pin(pts[pts.length - 1], "#e85b5b");
    ctx.globalAlpha = 1;
  }

  /** Ridge-drawing tool: sketch each drawn ridge line in world-cell space. The
   *  stroke is `width*2` cells wide (the range's footprint) and its opacity tracks
   *  the peak height; erase lines read cool/blue. Antimeridian seam split like the
   *  trade/itinerary routes. */
  private renderRidgeSketch(ctx: CanvasRenderingContext2D) {
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    const seamGap = 20;
    for (const line of this.ridgeSketch) {
      const pts = line.points;
      if (pts.length < 1) continue;
      const alpha = 0.2 + 0.7 * Math.max(0, Math.min(1, line.height));
      // Footprint band (translucent) then a crisp spine thread on top.
      const band = Math.max(1, line.width * 2);
      const draw = (lw: number, a: number) => {
        ctx.lineWidth = lw;
        ctx.globalAlpha = a;
        ctx.beginPath();
        let started = false;
        for (let i = 0; i < pts.length; i++) {
          const [x, y] = pts[i];
          if (i > 0 && Math.abs(x - pts[i - 1][0]) > seamGap) started = false;
          if (!started) { ctx.moveTo(x + 0.5, y + 0.5); started = true; }
          else ctx.lineTo(x + 0.5, y + 0.5);
        }
        // A single-point line still shows a dot.
        if (pts.length === 1) { ctx.moveTo(pts[0][0] + 0.5, pts[0][1] + 0.5); ctx.lineTo(pts[0][0] + 0.5, pts[0][1] + 0.5); }
        ctx.stroke();
      };
      ctx.strokeStyle = line.erase ? "#5aa0d8" : "#c98a4a";
      draw(band, alpha * 0.35);
      const spine = Math.max(0.8, 1.6 / Math.sqrt(this.currentScale));
      ctx.strokeStyle = line.erase ? "#bfe0f5" : "#f0c890";
      draw(spine, Math.min(1, alpha + 0.15));
    }
    ctx.globalAlpha = 1;
  }

  /** #37 · scarcity discs. Premium (local price ÷ world base value) maps green
   *  (≤0.8, cheap) → grey (≈1, par) → red (≥1.5, dear); disc size grows with how
   *  far from par the price sits, so the priciest/cheapest markets read loudest. */
  private renderGoodScarcity(ctx: CanvasRenderingContext2D) {
    const base = Math.max(1.4, 3.2 / Math.sqrt(this.currentScale));
    for (const c of this.goodScarcity) {
      const p = c.premium;
      // Colour ramp around par (1.0).
      let col: string;
      if (p <= 1) {
        const t = Math.max(0, Math.min(1, (1 - p) / 0.5)); // 0 at par → 1 at half price
        col = `rgb(${Math.round(120 - 50 * t)},${Math.round(150 + 60 * t)},${Math.round(120 - 40 * t)})`;
      } else {
        const t = Math.max(0, Math.min(1, (p - 1) / 0.8)); // 0 at par → 1 at +80%
        col = `rgb(${Math.round(150 + 90 * t)},${Math.round(140 - 90 * t)},${Math.round(120 - 80 * t)})`;
      }
      const r = base * (1 + Math.min(1.2, Math.abs(p - 1) * 1.4));
      ctx.beginPath();
      ctx.arc(c.x + 0.5, c.y + 0.5, r, 0, Math.PI * 2);
      ctx.globalAlpha = 0.72;
      ctx.fillStyle = col;
      ctx.fill();
      ctx.globalAlpha = 0.9;
      ctx.lineWidth = Math.max(0.3, r * 0.18);
      ctx.strokeStyle = "#0b1420";
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
  }

  /** #26 · draw toponym labels. Regions (culture hearths + the large desert/
   *  forest/tundra biome subregions) read as faint uppercase tracking; rivers/
   *  peaks/lakes get a small kind-coloured dot + italic-ish name. Sizes are
   *  zoom-compensated and kept legible with a dark halo. */
  private renderToponyms(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    // Feature kind → its entry in the label typography registry, so face, case,
    // tracking and colour all come from one editable place (see `labelStyles`).
    const KEY: Record<string, LabelKey> = {
      river: "river", lake: "lake", mountain: "mountain", region: "cultureRegion",
      desert: "desert", forest: "forest", tundra: "tundra",
    };
    const COLORS: Record<string, string> = Object.fromEntries(
      Object.entries(KEY).map(([k, key]) => [k, labelStyles[key].color]),
    );
    // Region-style (faint uppercase, centroid, no collision check, no dot):
    // the culture hearth AND the large biome subregions — all area labels, not
    // point features.
    const isRegionStyle = (kind: string) =>
      kind === "region" || kind === "desert" || kind === "forest" || kind === "tundra";
    ctx.textBaseline = "middle";
    ctx.lineJoin = "round";
    // Region-style first (they always draw — faint area labels), then the point
    // features, which are dropped when they'd overlap an already-drawn label so a
    // cluster of nearby rivers/peaks doesn't stack names on one spot.
    const ordered = [...this.toponyms].sort((a, b) =>
      (isRegionStyle(a.kind) ? 0 : 1) - (isRegionStyle(b.kind) ? 0 : 1));
    // Per-feature-type visibility (split toggles under the master `toponyms`).
    const kindVisible: Record<string, boolean> = {
      river: this.visibility.toponymsRiver !== false,
      lake: this.visibility.toponymsLake !== false,
      mountain: this.visibility.toponymsMountain !== false,
      region: this.visibility.toponymsRegion !== false,
      desert: this.visibility.toponymsDesert !== false,
      forest: this.visibility.toponymsForest !== false,
      tundra: this.visibility.toponymsTundra !== false,
    };
    for (const t of ordered) {
      if (kindVisible[t.kind] === false) continue;
      const region = isRegionStyle(t.kind);
      const key = KEY[t.kind] ?? "mountain";
      const fs = Math.max(6, Math.min(16, (region ? 13 : 9) * inv));
      const col = COLORS[t.kind] ?? "#cfe2f6";
      // Rivers read like an atlas: the name is set in italic and CURVES along the
      // channel, angled to the flow. Falls back to a point label if the reach is
      // too short/kinked or the label would collide.
      if (t.kind === "river" && this.drawRiverLabel(ctx, t, fs, col, inv)) continue;
      const dotR = Math.max(0.5, 1.4 * inv);
      const tx = t.x + 0.5 + (region ? 0 : dotR + 1.5 * inv);
      // Point features de-collide (regions bypass — they're a different, faint layer).
      if (!region) {
        const wLbl = this.measureLabel(ctx, key, t.name, fs);
        const x0 = tx - fs * 0.2, x1 = tx + wLbl + fs * 0.2;
        const y0 = t.y + 0.5 - fs * 0.7, y1 = t.y + 0.5 + fs * 0.7;
        if (this.labelCollides(x0, y0, x1, y1)) continue;
        this.placedLabels.push({ x0, y0, x1, y1 });
        // Non-region features get a small locator dot.
        ctx.beginPath();
        ctx.arc(t.x + 0.5, t.y + 0.5, dotR, 0, Math.PI * 2);
        ctx.fillStyle = col;
        ctx.fill();
      }
      this.drawLabel(ctx, key, t.name, tx, t.y + 0.5, fs, region ? "center" : "left", {
        // Area labels stay deliberately faint — they sit under everything else.
        color: region ? labelAlpha(labelStyles[key].color, 0.85) : undefined,
        haloWidth: Math.max(0.6, 2.2 * inv),
      });
    }
    ctx.textAlign = "left";
  }

  /** 🌊 Draw the reach-break markers: a small amber tick across the channel + a
   *  "Upper › Middle @X km" label where a trunk river changes reach. De-collides
   *  against already-placed labels (toponyms run first). */
  private renderRiverBreaks(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const tick = Math.max(3, 7 * inv);
    const fs = Math.max(6, 8.5 * inv);
    ctx.save();
    ctx.font = `600 ${fs}px -apple-system, Segoe UI, sans-serif`;
    ctx.textBaseline = "middle";
    for (const b of this.riverBreaks) {
      const len = Math.hypot(b.tx, b.ty) || 1;
      const nx = -b.ty / len, ny = b.tx / len;   // perpendicular to flow
      const cx = b.x + 0.5, cy = b.y + 0.5;
      // Tick across the channel (dark halo under amber).
      ctx.beginPath();
      ctx.moveTo(cx - nx * tick, cy - ny * tick);
      ctx.lineTo(cx + nx * tick, cy + ny * tick);
      ctx.strokeStyle = "rgba(8,14,20,0.9)";
      ctx.lineWidth = Math.max(0.9, 2.6 * inv);
      ctx.stroke();
      ctx.strokeStyle = "#ffd27f";
      ctx.lineWidth = Math.max(0.5, 1.4 * inv);
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(cx, cy, Math.max(0.9, 1.7 * inv), 0, Math.PI * 2);
      ctx.fillStyle = "#ffd27f";
      ctx.fill();
      // Label offset to the side, de-collided.
      const tx = cx + nx * (tick + 2.5 * inv), ty = cy + ny * (tick + 2.5 * inv);
      const w = ctx.measureText(b.label).width;
      const x0 = tx - fs * 0.3, x1 = tx + w + fs * 0.3, y0 = ty - fs * 0.8, y1 = ty + fs * 0.8;
      if (this.labelCollides(x0, y0, x1, y1)) continue;
      this.placedLabels.push({ x0, y0, x1, y1 });
      ctx.textAlign = "left";
      ctx.lineWidth = Math.max(0.6, 2.0 * inv);
      ctx.strokeStyle = "rgba(8,14,20,0.85)";
      ctx.strokeText(b.label, tx, ty);
      ctx.fillStyle = "#ffe0a0";
      ctx.fillText(b.label, tx, ty);
    }
    ctx.textAlign = "left";
    ctx.restore();
  }

  /** Atlas-style CURVED river label: lays the name glyph-by-glyph along the
   *  river channel it sits on, each letter rotated to the local flow direction
   *  and the whole string flipped when the reach runs right-to-left so it always
   *  reads upright. Returns false (→ caller draws a plain point label) when no
   *  river is close enough, the reach is too short/kinked for the name, or it
   *  crosses the wrap seam. Returns true when handled (drawn OR skipped-on-collision). */
  private drawRiverLabel(ctx: CanvasRenderingContext2D, t: { name: string; x: number; y: number }, fs: number, col: string, inv: number): boolean {
    const hit = this.nearestRiverPoint(t.x, t.y);
    if (!hit) return false;
    const pts = hit.river.points;
    const n = pts.length;
    if (n < 3) return false;

    // Face, case and tracking come from the `river` label style; the curve logic
    // below is unchanged (it already lays out character by character).
    ctx.font = this.labelFont("river", fs);
    const chars = [...this.labelText("river", t.name)];
    const rst = labelStyles.river;
    const sp = fs * rst.size * Math.max(0.05, rst.tracking);  // letter spacing (atlas feel)
    const widths = chars.map((c) => ctx.measureText(c).width);
    const W = widths.reduce((a, b) => a + b, 0) + sp * Math.max(0, chars.length - 1);

    // Cumulative arc length along the channel, bailing on a wrap-seam jump.
    const half = this.worldW > 0 ? this.worldW / 2 : Infinity;
    const cum = new Array<number>(n);
    cum[0] = 0;
    for (let i = 1; i < n; i++) {
      const dx = pts[i][0] - pts[i - 1][0], dy = pts[i][1] - pts[i - 1][1];
      if (Math.abs(dx) > half) return false;    // crosses the seam → point label
      cum[i] = cum[i - 1] + Math.hypot(dx, dy);
    }
    const sMid = cum[hit.bi];
    const sStart = sMid - W / 2, sEnd = sMid + W / 2;
    if (sStart < 0 || sEnd > cum[n - 1]) return false;   // reach too short for the name

    const a0 = this.arcSample(pts, cum, sStart);
    const a1 = this.arcSample(pts, cum, sEnd);
    const reversed = a1.x < a0.x;               // reads right-to-left → flip upright

    // Approximate collision box for the whole label; de-collide like point labels.
    const minx = Math.min(a0.x, a1.x) - fs, maxx = Math.max(a0.x, a1.x) + fs;
    const miny = Math.min(a0.y, a1.y) - fs, maxy = Math.max(a0.y, a1.y) + fs;
    if (this.labelCollides(minx, miny, maxx, maxy)) return true; // handled: skip, don't stamp a flat one
    this.placedLabels.push({ x0: minx, y0: miny, x1: maxx, y1: maxy });

    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.lineWidth = Math.max(0.6, 2.0 * inv);
    ctx.strokeStyle = "rgba(6,12,18,0.85)";
    ctx.fillStyle = col;
    let acc = 0;
    for (let k = 0; k < chars.length; k++) {
      const w = widths[k];
      const sChar = reversed ? (sEnd - acc - w / 2) : (sStart + acc + w / 2);
      acc += w + sp;
      const s = this.arcSample(pts, cum, sChar);
      const ang = Math.atan2(s.ty, s.tx) + (reversed ? Math.PI : 0);
      ctx.save();
      ctx.translate(s.x + 0.5, s.y + 0.5);
      ctx.rotate(ang);
      ctx.strokeText(chars[k], 0, -fs * 0.12); // nudge off the channel line so it stays visible
      ctx.fillText(chars[k], 0, -fs * 0.12);
      ctx.restore();
    }
    ctx.textAlign = "left";
    return true;
  }

  /** The river (from the drawn set) whose nearest path point is closest to
   *  `(x,y)`, with that point's index — or null if none is within ~3 cells.
   *  Cached per (toponym position) since rivers don't move between data loads. */
  private nearestRiverPoint(x: number, y: number): { river: RiverData; bi: number } | null {
    const key = `${x},${y}`;
    const cached = this.riverLabelCache.get(key);
    if (cached !== undefined) return cached;
    let best: { river: RiverData; bi: number } | null = null;
    let bd = 9; // (3 cells)² — must sit right on the channel to attach
    for (const river of this.rivers) {
      const pts = river.points;
      for (let i = 0; i < pts.length; i++) {
        const dx = pts[i][0] - x, dy = pts[i][1] - y;
        const d = dx * dx + dy * dy;
        if (d < bd) { bd = d; best = { river, bi: i }; }
      }
    }
    this.riverLabelCache.set(key, best);
    return best;
  }

  /** Position + unit tangent at arc length `s` along a polyline with cumulative
   *  lengths `cum` (binary-search the segment, then linear-interpolate). */
  private arcSample(pts: [number, number][], cum: number[], s: number): { x: number; y: number; tx: number; ty: number } {
    const n = pts.length;
    let lo = 0, hi = n - 1;
    while (lo < hi) { const mid = (lo + hi) >> 1; if (cum[mid] < s) lo = mid + 1; else hi = mid; }
    const i = Math.max(1, lo);
    const seg = cum[i] - cum[i - 1] || 1;
    const f = Math.min(1, Math.max(0, (s - cum[i - 1]) / seg));
    const ax = pts[i - 1][0], ay = pts[i - 1][1], bx = pts[i][0], by = pts[i][1];
    let tx = bx - ax, ty = by - ay;
    const len = Math.hypot(tx, ty) || 1;
    tx /= len; ty /= len;
    return { x: ax + (bx - ax) * f, y: ay + (by - ay) * f, tx, ty };
  }

  /** Boundary edges of a coarse-cell mask (only edges whose neighbour is outside
   *  the set), as a flat [x1,y1,x2,y2,…] list. Computed once per region and cached
   *  on the cell array (WeakMap), so the per-frame string-Set rebuild is gone. */
  private maskEdges(cells: [number, number][], cellSize: number): number[] {
    const cached = this.edgeCache.get(cells);
    if (cached) return cached;
    const set = new Set(cells.map(([cx, cy]) => `${cx},${cy}`));
    const e: number[] = [];
    for (const [cx, cy] of cells) {
      if (!set.has(`${cx},${cy - cellSize}`)) e.push(cx, cy, cx + cellSize, cy);
      if (!set.has(`${cx},${cy + cellSize}`)) e.push(cx, cy + cellSize, cx + cellSize, cy + cellSize);
      if (!set.has(`${cx - cellSize},${cy}`)) e.push(cx, cy, cx, cy + cellSize);
      if (!set.has(`${cx + cellSize},${cy}`)) e.push(cx + cellSize, cy, cx + cellSize, cy + cellSize);
    }
    this.edgeCache.set(cells, e);
    return e;
  }

  /** A filled cell-mask AREA (the real distribution shape) with a boundary
   *  outline and a centered emoji glyph (shark / trade-good belt). When `values`
   *  is given (per-cell 0..255 abundance) each cell is shaded by its richness. When
   *  `subtypes` + `subPalette` are given (grain species / paper sources), each cell
   *  is tinted by its subtype, boundaries between differing subtypes are drawn (the
   *  "split"), and per-subtype icons/labels are placed at each subtype's centroid. */
  private renderRegionMask(
    ctx: CanvasRenderingContext2D,
    cells: [number, number][], cellSize: number, color: string, emoji: string,
    lx: number, ly: number, alpha: number, sublabel: string = "", values?: number[],
    subtypes?: number[], subPalette?: SubtypeDef[], iconName?: string,
  ) {
    if (cells.length === 0) return;
    const hasSub = !!(subtypes && subPalette && subtypes.length === cells.length);
    const hasVals = !!(values && values.length === cells.length);

    // Fill every coarse cell. DLC 4 · TERROIR HEATMAP: per-cell quality (the
    // belt's suitability `value`) drives a continuous ramp — pale & faint where the
    // good grows poorly, rich & saturated where the terroir is finest (think wine
    // regions). Subtype goods (grain/paper) keep their subtype tint but still ramp
    // their richness by quality. Zones without per-cell values fill flat.
    for (let i = 0; i < cells.length; i++) {
      const [cx, cy] = cells[i];
      const baseCol = hasSub ? (subPalette![subtypes![i]]?.color ?? color) : color;
      if (hasVals) {
        const t = Math.max(0, Math.min(1, values![i] / 255));
        ctx.fillStyle = rampToward(baseCol, t);   // pale (low) → full colour (high)
        ctx.globalAlpha = alpha * (0.45 + 1.5 * t); // richer quality = more opaque
      } else {
        ctx.fillStyle = baseCol;
        ctx.globalAlpha = alpha;
      }
      ctx.fillRect(cx, cy, cellSize, cellSize);
    }

    // Boundary outline from the precomputed edge list.
    const edges = this.maskEdges(cells, cellSize);
    ctx.globalAlpha = Math.min(0.85, alpha + 0.4);
    ctx.strokeStyle = color;
    ctx.lineWidth = Math.max(0.4, 1.0 / Math.sqrt(this.currentScale));
    ctx.beginPath();
    for (let i = 0; i < edges.length; i += 4) {
      ctx.moveTo(edges[i], edges[i + 1]);
      ctx.lineTo(edges[i + 2], edges[i + 3]);
    }
    ctx.stroke();

    // Internal subtype splits: stroke edges between cells of differing subtype.
    if (hasSub) {
      const sEdges = this.subtypeEdges(cells, subtypes!, cellSize);
      ctx.globalAlpha = 0.7;
      ctx.strokeStyle = "rgba(20,16,10,0.8)";
      ctx.lineWidth = Math.max(0.4, 1.2 / Math.sqrt(this.currentScale));
      ctx.beginPath();
      for (let i = 0; i < sEdges.length; i += 4) {
        ctx.moveTo(sEdges[i], sEdges[i + 1]);
        ctx.lineTo(sEdges[i + 2], sEdges[i + 3]);
      }
      ctx.stroke();
    }
    ctx.globalAlpha = 1;

    // Labels: per-subtype icons at each subtype's centroid (when the palette
    // carries icons — paper), otherwise the single good emoji at the centroid.
    if (hasSub && subPalette!.some((s) => s.icon)) {
      const fs = Math.max(6, 14 / this.currentScale);
      ctx.font = `${fs}px sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      // Centroid per present subtype.
      const sums = new Map<number, { x: number; y: number; n: number }>();
      for (let i = 0; i < cells.length; i++) {
        const s = subtypes![i];
        const e = sums.get(s) ?? { x: 0, y: 0, n: 0 };
        e.x += cells[i][0]; e.y += cells[i][1]; e.n++;
        sums.set(s, e);
      }
      for (const [s, e] of sums) {
        const ic = subPalette![s]?.icon;
        if (!ic || e.n < 2) continue;
        ctx.fillText(ic, e.x / e.n + cellSize / 2, e.y / e.n + cellSize / 2);
      }
      ctx.textAlign = "start";
      ctx.textBaseline = "alphabetic";
    } else if (iconName) {
      // Trade-good medallion (EU4-style vector roundel) at the region centroid.
      const r = Math.max(4, 12 / this.currentScale);
      drawGoodIcon(ctx, iconName, lx, ly, r, color, { sublabel });
      if (sublabel) {
        const ss = Math.max(5, 9 / this.currentScale);
        ctx.font = `${ss}px serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.lineWidth = Math.max(0.6, 2 / this.currentScale);
        ctx.strokeStyle = "rgba(0,0,0,0.7)";
        ctx.strokeText(sublabel, lx, ly + r * 1.5);
        ctx.fillStyle = "#f0e8d0";
        ctx.fillText(sublabel, lx, ly + r * 1.5);
        ctx.textAlign = "start";
        ctx.textBaseline = "alphabetic";
      }
    } else if (emoji) {
      // Hazard-zone glyph (shark / shipworm / storm / reef) — kept as an emoji.
      const fs = Math.max(6, 16 / this.currentScale);
      ctx.font = `${fs}px sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(emoji, lx, ly);
      ctx.textAlign = "start";
      ctx.textBaseline = "alphabetic";
    }
  }

  /** Edges between adjacent cells whose subtype differs (the species "split"
   *  lines), cached per `subtypes` array. */
  private subtypeEdges(cells: [number, number][], subtypes: number[], cellSize: number): number[] {
    const cached = this.subtypeEdgeCache.get(subtypes);
    if (cached) return cached;
    const sub = new Map<string, number>();
    for (let i = 0; i < cells.length; i++) sub.set(`${cells[i][0]},${cells[i][1]}`, subtypes[i]);
    const e: number[] = [];
    for (let i = 0; i < cells.length; i++) {
      const [cx, cy] = cells[i];
      const s = subtypes[i];
      const right = sub.get(`${cx + cellSize},${cy}`);
      const down = sub.get(`${cx},${cy + cellSize}`);
      if (right !== undefined && right !== s) e.push(cx + cellSize, cy, cx + cellSize, cy + cellSize);
      if (down !== undefined && down !== s) e.push(cx, cy + cellSize, cx + cellSize, cy + cellSize);
    }
    this.subtypeEdgeCache.set(subtypes, e);
    return e;
  }

  /** Bundled commodity trunks: each routed coarse edge drawn with width ∝ the
   *  total goods volume travelling along it, so shared corridors read as trunks. */
  /** The live merchant layer: each active family/guild route as a line coloured by
   *  the owning house (width ∝ volume, dashed overland / solid by sea), with a dot
   *  at each end to read as a round-trip corridor. */
  /** Tint every settlement by its use of the selected coin: the mint city brightest,
   *  heavy users solid gold, light users dim gold, reserve-reach a dashed green ring.
   *  Cities that use a DIFFERENT coin are left as small grey dots. */
  /** All-coins MONETARY-DOMINANCE map: every settlement filled with the colour of
   *  the coin it settles in (its `primary` currency). A city that has flipped to a
   *  FOREIGN coin (a reserve currency that took its market) gets a bright ring, so
   *  reserve-currency incursions read at a glance; each coin's own mint is starred. */
  private renderCoinDominance(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const prim = this.coinUse.filter((u) => u.primary);
    let maxVol = 0;
    for (const u of prim) maxVol = Math.max(maxVol, u.volume);
    const base = Math.max(2, 3.6 * inv);
    ctx.lineCap = "round";

    // Currency ZONES: a translucent territory hull per coin (its primary cities),
    // so the map reads as contiguous monetary regions rather than loose dots.
    const byCoin = new Map<number, { x: number; y: number }[]>();
    const coinColor = new Map<number, string>();
    for (const u of prim) {
      if (!byCoin.has(u.coin)) { byCoin.set(u.coin, []); coinColor.set(u.coin, u.color || "#c9a227"); }
      byCoin.get(u.coin)!.push({ x: u.x + 0.5, y: u.y + 0.5 });
    }
    for (const [coin, pts] of byCoin) {
      if (pts.length < 3) continue;
      const hull = this.convexHull(pts);
      if (hull.length < 3) continue;
      ctx.beginPath();
      ctx.moveTo(hull[0].x, hull[0].y);
      for (let i = 1; i < hull.length; i++) ctx.lineTo(hull[i].x, hull[i].y);
      ctx.closePath();
      ctx.fillStyle = coinColor.get(coin) || "#c9a227";
      ctx.globalAlpha = 0.08; ctx.fill();
      ctx.globalAlpha = 0.28; ctx.lineWidth = Math.max(0.5, 0.9 * inv);
      ctx.setLineDash([Math.max(2, 4 * inv), Math.max(1.5, 3 * inv)]); ctx.stroke(); ctx.setLineDash([]);
    }
    ctx.globalAlpha = 1;
    for (const u of prim) {
      const t = maxVol > 0 ? u.volume / maxVol : 0;
      const r = u.mint ? base * 2.0 : base * (0.9 + 0.9 * t);
      const cx = u.x + 0.5, cy = u.y + 0.5;
      // Fill by the settling coin's colour.
      ctx.fillStyle = u.color || "#c9a227";
      ctx.globalAlpha = 0.85;
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fill();
      if (u.mint) {
        // A mint — the capital of its currency: bright outline.
        ctx.strokeStyle = "#fff3c0"; ctx.globalAlpha = 0.95; ctx.lineWidth = Math.max(0.7, 1.3 * inv);
        ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.stroke();
      } else {
        // A foreign coin settles here → a reserve currency has taken this market.
        ctx.strokeStyle = "#eaf2ff"; ctx.globalAlpha = 0.7; ctx.lineWidth = Math.max(0.5, 0.9 * inv);
        ctx.beginPath(); ctx.arc(cx, cy, r + 1.2 * inv, 0, Math.PI * 2); ctx.stroke();
      }
    }
    ctx.globalAlpha = 1;
  }

  /** Convex hull (monotone chain) of {x,y} points — the coin's "territory" shape. */
  private convexHull(pts: { x: number; y: number }[]): { x: number; y: number }[] {
    if (pts.length < 3) return pts.slice();
    const p = pts.slice().sort((a, b) => a.x - b.x || a.y - b.y);
    const cross = (o: any, a: any, b: any) => (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    const lower: any[] = [];
    for (const q of p) { while (lower.length >= 2 && cross(lower[lower.length - 2], lower[lower.length - 1], q) <= 0) lower.pop(); lower.push(q); }
    const upper: any[] = [];
    for (let i = p.length - 1; i >= 0; i--) { const q = p[i]; while (upper.length >= 2 && cross(upper[upper.length - 2], upper[upper.length - 1], q) <= 0) upper.pop(); upper.push(q); }
    lower.pop(); upper.pop();
    return lower.concat(upper);
  }

  /** Single-coin DRILL-DOWN: the selected coin's reach in THREE tiers — PRIMARY
   *  (its home turf: bold coins inside a translucent territory hull), RESERVE-reach
   *  (held abroad as a reserve: green dashed rings) and HELD-only (a minor basket
   *  holding: faint dots). Shows both how far a coin has spread and where it rules. */
  private renderCoinUsage(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const cities = this.coinUse.filter((u) => u.coin === this.coinOverlayHub);
    let maxVol = 0;
    for (const u of cities) maxVol = Math.max(maxVol, u.volume);
    const base = Math.max(2, 4 * inv);

    // Territory hull around the PRIMARY cities (the coin's core turf).
    const primary = cities.filter((u) => u.primary);
    const coinColor = cities[0]?.color || "#c9a227";
    if (primary.length >= 3) {
      const hull = this.convexHull(primary.map((u) => ({ x: u.x + 0.5, y: u.y + 0.5 })));
      if (hull.length >= 3) {
        ctx.beginPath();
        ctx.moveTo(hull[0].x, hull[0].y);
        for (let i = 1; i < hull.length; i++) ctx.lineTo(hull[i].x, hull[i].y);
        ctx.closePath();
        ctx.fillStyle = coinColor; ctx.globalAlpha = 0.10; ctx.fill();
        ctx.strokeStyle = coinColor; ctx.globalAlpha = 0.4;
        ctx.lineWidth = Math.max(0.6, 1.1 * inv);
        ctx.setLineDash([Math.max(2, 4 * inv), Math.max(1.5, 3 * inv)]); ctx.stroke(); ctx.setLineDash([]);
      }
    }

    ctx.lineCap = "round";
    for (const u of cities) {
      const t = maxVol > 0 ? u.volume / maxVol : 0;
      const cx = u.x + 0.5, cy = u.y + 0.5;
      if (u.primary) {
        // Home turf — a bold struck coin sized by volume.
        const r = u.mint ? base * 2.0 : base * (1.0 + 0.8 * t);
        ctx.fillStyle = u.mint ? "#f6df85" : coinColor;
        ctx.globalAlpha = u.mint ? 1 : 0.7 + 0.3 * t;
        ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fill();
        ctx.strokeStyle = u.mint ? "#fff3c0" : "#12202f"; ctx.globalAlpha = 0.9;
        ctx.lineWidth = Math.max(0.5, (u.mint ? 1.1 : 0.7) * inv);
        ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.stroke();
      } else if (u.reserve_reach) {
        // Held abroad as a reserve — a green dashed ring (not its primary money).
        const r = base * (1.0 + 0.5 * t);
        ctx.strokeStyle = "#37a05a"; ctx.globalAlpha = 0.9;
        ctx.lineWidth = Math.max(0.8, 1.4 * inv);
        ctx.setLineDash([Math.max(1.5, 3 * inv), Math.max(1.5, 2 * inv)]);
        ctx.beginPath(); ctx.arc(cx, cy, r + 2 * inv, 0, Math.PI * 2); ctx.stroke();
        ctx.setLineDash([]);
      } else {
        // Minor basket holding — a faint dot.
        ctx.fillStyle = coinColor; ctx.globalAlpha = 0.28 + 0.25 * t;
        ctx.beginPath(); ctx.arc(cx, cy, Math.max(1, base * 0.7), 0, Math.PI * 2); ctx.fill();
      }
    }
    ctx.globalAlpha = 1;
  }

  private renderBankIcons(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const r = Math.max(2.2, 5 * inv);
    const fs = Math.max(5, 9 * inv);
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (const b of this.bankIcons) {
      if (b.defunct) continue;
      const cx = b.x + 0.5, cy = b.y + 0.5;
      // Gold disc + the owning house's colour ring.
      ctx.globalAlpha = 0.95;
      ctx.fillStyle = "#1a2230";
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fill();
      ctx.strokeStyle = b.color || "#e0c452";
      ctx.lineWidth = Math.max(0.6, 1.4 * inv);
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.stroke();
      ctx.globalAlpha = 1;
      ctx.font = `${fs}px sans-serif`;
      ctx.fillText("🏦", cx, cy + fs * 0.05);
    }
    ctx.globalAlpha = 1;
    ctx.textAlign = "left";
    ctx.textBaseline = "alphabetic";
  }

  /** Phase 6 · plague overlay: contagion routes (faint dashed red, source→city) +
   *  a red glow on each struck city (brighter + a skull while still quarantined). */
  private renderPlagueZones(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const W = this.worldW;
    // Contagion routes first (under the city glyphs).
    ctx.strokeStyle = "rgba(200,70,70,0.5)";
    ctx.lineWidth = Math.max(0.4, 1.0 * inv);
    ctx.setLineDash([Math.max(1, 3 * inv), Math.max(1, 3 * inv)]);
    const ah = Math.max(1.2, 3 * inv); // arrowhead size (world cells)
    for (const e of this.plagueEdges) {
      if (W > 0 && Math.abs(e.bx - e.ax) > W / 2) continue; // skip seam-crossing (no slash)
      const ax = e.ax + 0.5, ay = e.ay + 0.5, bx = e.bx + 0.5, by = e.by + 0.5;
      ctx.beginPath(); ctx.moveTo(ax, ay); ctx.lineTo(bx, by); ctx.stroke();
      // Arrowhead at the destination = direction the pestilence travelled.
      const ang = Math.atan2(by - ay, bx - ax);
      ctx.setLineDash([]);
      ctx.beginPath();
      ctx.moveTo(bx, by);
      ctx.lineTo(bx - ah * Math.cos(ang - 0.4), by - ah * Math.sin(ang - 0.4));
      ctx.moveTo(bx, by);
      ctx.lineTo(bx - ah * Math.cos(ang + 0.4), by - ah * Math.sin(ang + 0.4));
      ctx.stroke();
      ctx.setLineDash([Math.max(1, 3 * inv), Math.max(1, 3 * inv)]);
    }
    ctx.setLineDash([]);
    // City glow (origin gets a star; active cities a skull).
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    const fs = Math.max(5, 9 * inv);
    for (const c of this.plagueCities) {
      const cx = c.x + 0.5, cy = c.y + 0.5;
      const r = Math.max(2.0, (c.origin ? 7 : c.active ? 6 : 4) * inv);
      // Colour by DEATH TOLL so the worst-hit cities stand out (user rule): RED for a
      // touched city, BROWN when it bites hard, BLACK for a devastating cull.
      const dc = c.deaths >= 5000 ? "#141010" : c.deaths >= 800 ? "#7a3a1a" : "#e04040";
      ctx.globalAlpha = c.active ? 0.72 : 0.5;
      ctx.fillStyle = c.origin && c.deaths < 800 ? "#ff6030" : dc;
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fill();
      ctx.globalAlpha = 1;
      ctx.font = `${fs}px sans-serif`;
      if (c.origin) { ctx.fillStyle = "#ffe0b0"; ctx.fillText("★", cx, cy + fs * 0.05); }
      else if (c.active) { ctx.fillStyle = "#f0d0d0"; ctx.fillText("☠", cx, cy + fs * 0.05); }
    }
    ctx.globalAlpha = 1;
    ctx.textAlign = "left";
    ctx.textBaseline = "alphabetic";
  }

  /** Phase 7 · dynasty ties: a line between two house seat cities — solid gold for a
   *  marriage alliance, dashed red for a feud. Seam-crossing links are skipped. */
  private renderDynastyLinks(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const W = this.worldW;
    ctx.lineCap = "round";
    for (const l of this.dynastyLinks) {
      if (W > 0 && Math.abs(l.bx - l.ax) > W / 2) continue; // no slash across the seam
      ctx.strokeStyle = l.ally ? "rgba(220,180,90,0.7)" : "rgba(200,80,80,0.6)";
      ctx.lineWidth = Math.max(0.4, (l.ally ? 1.2 : 0.9) * inv);
      ctx.setLineDash(l.ally ? [] : [Math.max(1, 3 * inv), Math.max(1, 2 * inv)]);
      ctx.beginPath();
      ctx.moveTo(l.ax + 0.5, l.ay + 0.5);
      ctx.lineTo(l.bx + 0.5, l.by + 0.5);
      ctx.stroke();
    }
    ctx.setLineDash([]);
  }

  /** Phase 6/7 · generic emoji map markers (figures, landmarks): a tinted disc +
   *  the emoji, in world coords. */
  private renderEmojiMarks(ctx: CanvasRenderingContext2D, list: { x: number; y: number; emoji: string }[], ring: string) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const r = Math.max(2.0, 4.5 * inv);
    const fs = Math.max(5, 8.5 * inv);
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (const m of list) {
      const cx = m.x + 0.5, cy = m.y + 0.5;
      ctx.globalAlpha = 0.92;
      ctx.fillStyle = "#141018";
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fill();
      ctx.strokeStyle = ring;
      ctx.lineWidth = Math.max(0.6, 1.3 * inv);
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.stroke();
      ctx.globalAlpha = 1;
      ctx.font = `${fs}px sans-serif`;
      ctx.fillText(m.emoji || "•", cx, cy + fs * 0.05);
    }
    ctx.globalAlpha = 1;
    ctx.textAlign = "left";
    ctx.textBaseline = "alphabetic";
  }

  /** Phase 6 · guild cities: a gold disc + the good's emoji. */
  private renderGuildCities(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const r = Math.max(2.2, 5 * inv);
    const fs = Math.max(5, 9 * inv);
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (const g of this.guildCities) {
      const cx = g.x + 0.5, cy = g.y + 0.5;
      ctx.globalAlpha = 0.95;
      ctx.fillStyle = "#221c10";
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fill();
      ctx.strokeStyle = "#c0a040";
      ctx.lineWidth = Math.max(0.6, 1.4 * inv);
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.stroke();
      ctx.globalAlpha = 1;
      ctx.font = `${fs}px sans-serif`;
      ctx.fillText(g.emoji || "🏛", cx, cy + fs * 0.05);
      // Renowned crafts carry their place-brand beneath the marker.
      if (g.label) {
        ctx.font = `${Math.max(4, fs * 0.7)}px sans-serif`;
        ctx.fillStyle = "#e0c878";
        ctx.fillText(g.label, cx, cy + r + fs * 0.7);
      }
    }
    ctx.globalAlpha = 1;
    ctx.textAlign = "left";
    ctx.textBaseline = "alphabetic";
  }

  private renderFlowHighlight(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const W = this.worldW;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    for (const s of this.flowHighlight) {
      const inbound = s.dir === 0;
      const color = inbound ? lineColors.merchantIn : lineColors.merchantOut; // cyan in · gold out
      const w = Math.max(1.2, s.w * inv);
      // Trace the ACTUAL merchant path along the trade-routes layer (roads/sea-lanes).
      // NEVER draw a straight slash: if no corridor path can be found at all (the
      // routes layer isn't computed yet), skip this segment rather than drawing a
      // straight line.
      const path = this.routeAlongTradeRoutes([s.ax, s.ay], [s.bx, s.by]);
      if (!path || path.length < 2) continue;
      const pts: [number, number][] = path;
      const drawPolyline = () => {
        ctx.beginPath();
        let started = false;
        for (let i = 0; i < pts.length; i++) {
          let [px, py] = pts[i];
          if (i > 0 && W > 0 && Math.abs(px - pts[i - 1][0]) > W / 2) {
            // Seam crossing — break the stroke rather than slash across the map.
            ctx.stroke(); ctx.beginPath(); started = false;
          }
          if (!started) { ctx.moveTo(px, py); started = true; } else { ctx.lineTo(px, py); }
        }
        ctx.stroke();
      };
      ctx.strokeStyle = color;
      ctx.globalAlpha = 0.22; ctx.lineWidth = w * 3; drawPolyline();
      ctx.globalAlpha = 0.95; ctx.lineWidth = w; drawPolyline();
      // Directional chevrons ALONG the line (-->-- inbound · --<-- outbound) so the
      // flow direction reads at a glance, not just at the endpoint.
      {
        const spacing = Math.max(12, 30 * inv);
        const ah = Math.max(2.5, 5 * inv);
        ctx.strokeStyle = color; ctx.globalAlpha = 0.9;
        ctx.lineWidth = Math.max(0.8, 1.3 * inv);
        for (let i = 0; i < pts.length - 1; i++) {
          const [x0, y0] = pts[i], [x1, y1] = pts[i + 1];
          if (W > 0 && Math.abs(x1 - x0) > W / 2) continue; // skip the wrap-seam segment
          let dx = x1 - x0, dy = y1 - y0;
          const segLen = Math.hypot(dx, dy);
          if (segLen < 1e-3) continue;
          dx /= segLen; dy /= segLen;
          if (inbound) { dx = -dx; dy = -dy; } // chevrons point toward the receiving city
          const ang = Math.atan2(dy, dx);
          for (let d = spacing * 0.5; d < segLen; d += spacing) {
            const cx = x0 + (x1 - x0) * (d / segLen), cy = y0 + (y1 - y0) * (d / segLen);
            ctx.beginPath();
            ctx.moveTo(cx - ah * Math.cos(ang - 0.55), cy - ah * Math.sin(ang - 0.55));
            ctx.lineTo(cx, cy);
            ctx.lineTo(cx - ah * Math.cos(ang + 0.55), cy - ah * Math.sin(ang + 0.55));
            ctx.stroke();
          }
        }
      }
      // Arrowhead at the receiving end (city for inbound = path start, partner for
      // outbound = path end), aligned to that final leg.
      const [tx, ty, fx, fy] = inbound
        ? [pts[0][0], pts[0][1], pts[1][0], pts[1][1]]
        : [pts[pts.length - 1][0], pts[pts.length - 1][1], pts[pts.length - 2][0], pts[pts.length - 2][1]];
      if (!(W > 0 && Math.abs(tx - fx) > W / 2)) {
        const ang = Math.atan2(ty - fy, tx - fx);
        const ah = Math.max(3, 5 * inv);
        ctx.fillStyle = color; ctx.globalAlpha = 0.95;
        ctx.beginPath();
        ctx.moveTo(tx, ty);
        ctx.lineTo(tx - ah * Math.cos(ang - 0.4), ty - ah * Math.sin(ang - 0.4));
        ctx.lineTo(tx - ah * Math.cos(ang + 0.4), ty - ah * Math.sin(ang + 0.4));
        ctx.closePath(); ctx.fill();
      }
    }
    ctx.globalAlpha = 1;
  }

  private renderMerchantRoutes(ctx: CanvasRenderingContext2D) {
    let maxVol = 0;
    for (const r of this.merchantRoutes) maxVol = Math.max(maxVol, r.volume);
    if (maxVol <= 0) return;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    const W = this.worldW;
    const dash = Math.max(1.5, 3 / Math.sqrt(this.currentScale));
    for (const r of this.merchantRoutes) {
      // Stroke the ACTUAL corridor (roads/sea-lanes), never a straight slash. If no
      // path could be snapped (trade-routes layer not built / endpoints off-network)
      // we skip the route rather than draw a diagonal line across the terrain.
      const pts = r.path;
      if (!pts || pts.length < 2) continue;
      const norm = r.volume / maxVol;
      ctx.globalAlpha = 0.5 + 0.4 * norm;
      ctx.strokeStyle = r.color || "#cccccc";
      ctx.lineWidth = Math.max(0.5, (0.8 + norm * 4.0) / Math.sqrt(this.currentScale));
      ctx.setLineDash(r.sea ? [] : [dash, dash]);
      ctx.beginPath();
      let started = false;
      for (let i = 0; i < pts.length; i++) {
        const px = pts[i][0] + 0.5, py = pts[i][1] + 0.5;
        if (i > 0 && W > 0 && Math.abs(px - (pts[i - 1][0] + 0.5)) > W / 2) {
          ctx.stroke(); ctx.beginPath(); started = false; // break at the wrap seam
        }
        if (!started) { ctx.moveTo(px, py); started = true; } else { ctx.lineTo(px, py); }
      }
      ctx.stroke();
      ctx.setLineDash([]);
      const dotR = Math.max(0.8, 1.6 / Math.sqrt(this.currentScale));
      ctx.globalAlpha = 0.85;
      ctx.fillStyle = r.color || "#cccccc";
      const a = pts[0], b = pts[pts.length - 1];
      ctx.beginPath(); ctx.arc(a[0] + 0.5, a[1] + 0.5, dotR, 0, Math.PI * 2); ctx.fill();
      ctx.beginPath(); ctx.arc(b[0] + 0.5, b[1] + 0.5, dotR, 0, Math.PI * 2); ctx.fill();
    }
    ctx.globalAlpha = 1;
    ctx.setLineDash([]);
  }

  /** Campaign trade CORRIDORS — each drawn as a dashed haul in its owner's colour,
   *  strung with waystation beads shaped by leg terrain (◆ river-port · ■ caravanserai
   *  · ● coastal factory · ▲ pass hospice). The owner colour matches the ward grid,
   *  so a corridor reads as "House X's road" and its beads as that house's posts. */
  private renderTradeCorridors(ctx: CanvasRenderingContext2D) {
    const corridors = this.tradeCorridorList;
    if (corridors.length === 0) return;
    const W = this.worldW;
    const inv = 1 / Math.sqrt(this.currentScale);
    let maxVol = 0;
    for (const c of corridors) maxVol = Math.max(maxVol, c.volume);
    if (maxVol <= 0) maxVol = 1;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    const dash = Math.max(1.5, 3 * inv);
    for (const c of corridors) {
      const pts = c.points;
      if (!pts || pts.length < 2) continue;
      const norm = c.volume / maxVol;
      const col = c.color || "#e0b24a";
      // 1) The routed haul (dashed, width ∝ volume).
      ctx.globalAlpha = 0.5 + 0.4 * norm;
      ctx.strokeStyle = col;
      ctx.lineWidth = Math.max(0.5, (0.8 + norm * 3.2) * inv);
      ctx.setLineDash([dash, dash]);
      ctx.beginPath();
      let started = false;
      for (let i = 0; i < pts.length; i++) {
        const px = pts[i][0] + 0.5, py = pts[i][1] + 0.5;
        if (i > 0 && W > 0 && Math.abs(px - (pts[i - 1][0] + 0.5)) > W / 2) {
          ctx.stroke(); ctx.beginPath(); started = false; // wrap seam
        }
        if (!started) { ctx.moveTo(px, py); started = true; } else { ctx.lineTo(px, py); }
      }
      ctx.stroke();
      ctx.setLineDash([]);
      // 2) Waystation beads — shaped by kind, filled in the owner's colour.
      const r = Math.max(0.9, 2.0 * inv);
      ctx.globalAlpha = 0.95;
      ctx.fillStyle = col;
      ctx.strokeStyle = "#10161f";
      ctx.lineWidth = Math.max(0.3, 0.5 * inv);
      for (const w of c.waystations) {
        const x = w.x + 0.5, y = w.y + 0.5;
        ctx.beginPath();
        if (w.kind === 1) {            // river-port ◆ diamond
          ctx.moveTo(x, y - r); ctx.lineTo(x + r, y); ctx.lineTo(x, y + r); ctx.lineTo(x - r, y); ctx.closePath();
        } else if (w.kind === 2) {     // caravanserai ■ square
          ctx.rect(x - r, y - r, r * 2, r * 2);
        } else if (w.kind === 4) {     // pass hospice ▲ triangle
          ctx.moveTo(x, y - r); ctx.lineTo(x + r, y + r); ctx.lineTo(x - r, y + r); ctx.closePath();
        } else {                       // coastal factory ● circle
          ctx.arc(x, y, r, 0, Math.PI * 2);
        }
        ctx.fill(); ctx.stroke();
      }
      // 3) Endpoint anchors (home + distant city).
      ctx.beginPath(); ctx.arc(pts[0][0] + 0.5, pts[0][1] + 0.5, r * 1.2, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
      const e = pts[pts.length - 1];
      ctx.beginPath(); ctx.arc(e[0] + 0.5, e[1] + 0.5, r * 1.2, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
    }
    ctx.globalAlpha = 1;
    ctx.setLineDash([]);
  }

  /** Expeditions: financed ventures crawling toward distant, unconnected cities.
   *  Each draws its intended track (faint), a caravan/ship marker at the current
   *  position (sized by fleet, coloured by heading — amber outbound, teal
   *  returning), a survival ring, and recent hazard sparks. Recent FAILED ventures
   *  drop a red ✕ at the loss site. This is "attempts on the map" made literal. */
  private renderExpeditions(ctx: CanvasRenderingContext2D) {
    const W = this.worldW;
    const inv = 1 / Math.sqrt(this.currentScale);
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    // 1) Failed ventures — faded red ✕ at the loss site.
    const fr = Math.max(1.1, 2.4 * inv);
    ctx.strokeStyle = "#e2544b";
    ctx.globalAlpha = 0.7;
    ctx.lineWidth = Math.max(0.5, 1.1 * inv);
    for (const f of this.expeditionFails) {
      const x = f.x + 0.5, y = f.y + 0.5;
      ctx.beginPath();
      ctx.moveTo(x - fr, y - fr); ctx.lineTo(x + fr, y + fr);
      ctx.moveTo(x + fr, y - fr); ctx.lineTo(x - fr, y + fr);
      ctx.stroke();
    }
    // 2) Active ventures.
    for (const e of this.expeditions) {
      const sea = e.ships > e.caravans;
      const col = e.outbound ? "#e0b24a" : "#5fc8a8";
      // intended track (origin → dest), faint + dashed.
      const dash = Math.max(1.5, 3 * inv);
      const seam = W > 0 && Math.abs((e.ox + 0.5) - (e.dx + 0.5)) > W / 2;
      if (!seam) {
        ctx.globalAlpha = 0.28;
        ctx.strokeStyle = col;
        ctx.lineWidth = Math.max(0.4, 0.7 * inv);
        ctx.setLineDash([dash, dash]);
        ctx.beginPath();
        ctx.moveTo(e.ox + 0.5, e.oy + 0.5);
        ctx.lineTo(e.dx + 0.5, e.dy + 0.5);
        ctx.stroke();
        ctx.setLineDash([]);
      }
      // recent hazard sparks along the track.
      ctx.globalAlpha = 0.8;
      ctx.fillStyle = "#e2544b";
      const hr = Math.max(0.6, 1.1 * inv);
      for (const h of e.hazards) {
        ctx.beginPath(); ctx.arc(h[0] + 0.5, h[1] + 0.5, hr, 0, Math.PI * 2); ctx.fill();
      }
      // the venture marker at the current position.
      const cx = e.x + 0.5, cy = e.y + 0.5;
      const r = Math.max(1.4, (2.2 + Math.min(6, e.caravans + e.ships) * 0.25) * inv);
      ctx.globalAlpha = 1;
      ctx.fillStyle = col;
      ctx.strokeStyle = "#10161f";
      ctx.lineWidth = Math.max(0.3, 0.6 * inv);
      ctx.beginPath();
      if (sea) {                 // ship ◆ diamond
        ctx.moveTo(cx, cy - r); ctx.lineTo(cx + r, cy); ctx.lineTo(cx, cy + r); ctx.lineTo(cx - r, cy); ctx.closePath();
      } else {                   // caravan ● circle
        ctx.arc(cx, cy, r, 0, Math.PI * 2);
      }
      ctx.fill(); ctx.stroke();
      // survival ring: red arc = fraction already lost.
      const lost = 1 - Math.max(0, Math.min(1, e.survived));
      if (lost > 0.02) {
        ctx.globalAlpha = 0.9;
        ctx.strokeStyle = "#e2544b";
        ctx.lineWidth = Math.max(0.5, 1.0 * inv);
        ctx.beginPath();
        ctx.arc(cx, cy, r + 1.2 * inv, -Math.PI / 2, -Math.PI / 2 + lost * Math.PI * 2);
        ctx.stroke();
      }
    }
    ctx.globalAlpha = 1;
    ctx.setLineDash([]);
  }

  /** The futures layer: each active contract as a DASHED, DIRECTED lane from the
   *  producer/warehouse city (square) to the buyer city (ring). Colour & weight rise
   *  with term (1yr faint → 7yr bold gold); a suspended (quarantined) lane greys out;
   *  the selected lane glows. This is the contractual network, distinct from the
   *  live spot voyages of the merchant layer. */
  private renderFutures(ctx: CanvasRenderingContext2D) {
    // Term → colour (gold gradient) and a base weight factor.
    const termColor = (t: number): string =>
      t >= 7 ? "#ffcf3f" : t >= 5 ? "#f0b54a" : t >= 3 ? "#d8a05a" : "#c8b486";
    let maxQty = 0;
    for (const r of this.futuresLanes) maxQty = Math.max(maxQty, r.qty);
    if (maxQty <= 0) maxQty = 1;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    const dash = Math.max(2, 5 / Math.sqrt(this.currentScale));
    const sel = this.selectedFuturesLane;
    const focus = this.futuresFocus;
    const matchesFocus = (r: FuturesLane): boolean => !focus
      || ((!focus.city || r.a_name === focus.city || r.b_name === focus.city)
        && (!focus.holder || r.holder === focus.holder)
        && (!focus.good || r.good === focus.good));
    // Stroke a lane as its ROUTED polyline (roads/sea), breaking at the cylindrical X
    // seam. A lane with no snapped corridor path is SKIPPED (returns false) rather than
    // bridged with a straight diagonal slash across the terrain — matching the merchant
    // layer, so futures never "connect cities directly" off the road network.
    const strokeLane = (r: FuturesLane): boolean => {
      const pts = r.path;
      if (!pts || pts.length < 2) return false;
      let started = false;
      for (let i = 0; i < pts.length; i++) {
        if (i > 0 && this.worldW > 0 && Math.abs(pts[i][0] - pts[i - 1][0]) > this.worldW / 2) started = false; // seam
        if (!started) { ctx.beginPath(); ctx.moveTo(pts[i][0] + 0.5, pts[i][1] + 0.5); started = true; }
        else ctx.lineTo(pts[i][0] + 0.5, pts[i][1] + 0.5);
      }
      ctx.stroke();
      return true;
    };
    for (const r of this.futuresLanes) {
      const ax = r.a[0] + 0.5, ay = r.a[1] + 0.5, bx = r.b[0] + 0.5, by = r.b[1] + 0.5;
      // No snapped corridor path → skip the lane entirely (no straight diagonal slash,
      // and no orphan source/buyer markers or arrowhead either).
      if (!(r.path && r.path.length >= 2)) continue;
      const isSel = sel != null && sel.a_name === r.a_name && sel.b_name === r.b_name
        && sel.holder === r.holder && sel.good === r.good;
      // A single selection isolates ONE road; otherwise a focus filter (city /
      // warehouse / good) keeps the matching lanes and fades all the others.
      const active = sel != null ? isSel : matchesFocus(r);
      const norm = r.qty / maxQty;
      const col = r.suspended ? "#7a8088" : termColor(r.term);
      if (!active) {
        // Faded context lane: thin, very translucent, no markers/arrow.
        ctx.globalAlpha = 0.1;
        ctx.strokeStyle = col;
        ctx.lineWidth = Math.max(0.4, 0.7 / Math.sqrt(this.currentScale));
        ctx.setLineDash([dash, dash]);
        strokeLane(r);
        ctx.setLineDash([]);
        continue;
      }
      ctx.globalAlpha = r.suspended ? 0.35 : (isSel ? 1.0 : 0.6 + 0.35 * norm);
      // Selected lane gets a soft glow underlay.
      if (isSel) {
        ctx.strokeStyle = "#fff2b0";
        ctx.globalAlpha = 0.35;
        ctx.lineWidth = Math.max(2, (5 + r.term + norm * 6) / Math.sqrt(this.currentScale));
        ctx.setLineDash([]);
        strokeLane(r);
        ctx.globalAlpha = 1.0;
      }
      ctx.strokeStyle = col;
      ctx.lineWidth = Math.max(0.6, (0.8 + r.term * 0.4 + norm * 2.5) / Math.sqrt(this.currentScale));
      ctx.setLineDash([dash, dash]);
      strokeLane(r);
      ctx.setLineDash([]);

      // Direction arrowhead toward the buyer (b). For a routed lane the heading is
      // taken from the last leg so the arrow aligns with the road, not the chord.
      const tail = r.path && r.path.length >= 2 ? r.path[r.path.length - 2] : r.a;
      let dx = bx - (tail[0] + 0.5), dy = by - (tail[1] + 0.5);
      const m = Math.hypot(dx, dy);
      if (m > 0.001) {
        dx /= m; dy /= m;
        const hl = Math.max(2.5, (5 + r.term) / Math.sqrt(this.currentScale));
        const px = -dy, py = dx;
        const tipx = bx - dx * hl * 0.6, tipy = by - dy * hl * 0.6;
        ctx.globalAlpha = r.suspended ? 0.5 : 0.95;
        ctx.fillStyle = col;
        ctx.beginPath();
        ctx.moveTo(tipx + dx * hl, tipy + dy * hl);
        ctx.lineTo(tipx + px * hl * 0.5, tipy + py * hl * 0.5);
        ctx.lineTo(tipx - px * hl * 0.5, tipy - py * hl * 0.5);
        ctx.closePath(); ctx.fill();
      }

      // Source end = filled SQUARE in the holder's colour (the producer/warehouse);
      // buyer end = a RING (the receiving city).
      const r0 = Math.max(0.9, 1.8 / Math.sqrt(this.currentScale));
      ctx.globalAlpha = r.suspended ? 0.5 : 0.95;
      ctx.fillStyle = r.color || "#cccccc";
      ctx.fillRect(ax - r0, ay - r0, r0 * 2, r0 * 2);
      ctx.strokeStyle = r.color || "#cccccc";
      ctx.lineWidth = Math.max(0.5, 1.2 / Math.sqrt(this.currentScale));
      ctx.beginPath(); ctx.arc(bx, by, r0 * 1.3, 0, Math.PI * 2); ctx.stroke();
    }
    ctx.globalAlpha = 1;
    ctx.setLineDash([]);
  }

  private renderTradeTrunks(
    ctx: CanvasRenderingContext2D,
    trunks: TradeTrunk[] = this.tradeTrunks,
    majorColor: string = lineColors.tradeTrunk,
    minorColor: string = lineColors.tradeTrunkMinor,
  ) {
    let maxVol = 0;
    for (const t of trunks) maxVol = Math.max(maxVol, t.volume);
    if (maxVol <= 0) return;

    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    const dash = Math.max(1.5, 3.5 / Math.sqrt(this.currentScale));
    // Two tiers so the main arteries stand out from the feeder routes: major
    // corridors (high volume) are solid, bright and thick; minor ones are thin,
    // muted and dashed. Each trunk carries a direction arrowhead (toward the
    // consuming hub) and the major arteries are labelled (Spice Road / Silk Road).
    const labels: { x: number; y: number; text: string }[] = [];
    for (const t of trunks) {
      const pts = t.points;
      if (pts.length < 2) continue;
      const a = pts[0], b = pts[1];
      // Skip edges spanning the cylindrical wrap seam.
      if (this.worldW > 0 && Math.abs(a[0] - b[0]) > this.worldW / 2) continue;
      const norm = t.volume / maxVol;
      const major = norm >= 0.45;
      ctx.globalAlpha = major ? 0.85 : 0.5;
      ctx.strokeStyle = major ? majorColor : minorColor;
      ctx.lineWidth = Math.max(
        0.5,
        (major ? 1.4 + norm * 5.0 : 0.5 + norm * 1.5) / Math.sqrt(this.currentScale),
      );
      ctx.setLineDash(major ? [] : [dash, dash]);
      const ax = a[0] + 0.5, ay = a[1] + 0.5, bx = b[0] + 0.5, by = b[1] + 0.5;
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      ctx.lineTo(bx, by);
      ctx.stroke();

      // Direction arrowhead at the consumer (b) end.
      let dx = bx - ax, dy = by - ay;
      const m = Math.hypot(dx, dy);
      if (m > 0.001) {
        dx /= m; dy /= m;
        const hl = Math.max(2, (major ? 7 : 4) / Math.sqrt(this.currentScale));
        const px = -dy, py = dx;
        ctx.setLineDash([]);
        ctx.beginPath();
        ctx.moveTo(bx, by);
        ctx.lineTo(bx - dx * hl + px * hl * 0.5, by - dy * hl + py * hl * 0.5);
        ctx.lineTo(bx - dx * hl - px * hl * 0.5, by - dy * hl - py * hl * 0.5);
        ctx.closePath();
        ctx.fillStyle = major ? majorColor : minorColor;
        ctx.fill();
      }
      if (t.road) labels.push({ x: (ax + bx) / 2, y: (ay + by) / 2, text: t.road });
    }
    ctx.setLineDash([]);

    // Road names along the major arteries (drawn last so they sit on top).
    if (labels.length > 0) {
      const fs = Math.max(7, 13 / this.currentScale);
      ctx.font = `${fs}px serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.globalAlpha = 0.95;
      for (const l of labels) {
        ctx.lineWidth = Math.max(1, 3 / this.currentScale);
        ctx.strokeStyle = "rgba(0,0,0,0.75)";
        ctx.strokeText(l.text, l.x, l.y);
        ctx.fillStyle = "#f4e3b0";
        ctx.fillText(l.text, l.x, l.y);
      }
      ctx.textAlign = "start";
      ctx.textBaseline = "alphabetic";
    }
    ctx.globalAlpha = 1;
  }

  /** Dynamic Trade Flow — last year's actual shipped volume, drawn in the smooth
   *  uniform "Trade Corridors" style (solid lines, width + alpha by volume, a single
   *  midpoint direction arrow) but in its own teal so it stays distinct from the
   *  green Corridors overlay. No dashed minor tier and no road-name labels. */
  private renderDynamicFlow(ctx: CanvasRenderingContext2D) {
    const trunks = this.dynamicTrunks;
    if (trunks.length === 0) return;
    const half = (this.worldW || 1e9) / 2;
    const inv = 1 / Math.sqrt(this.currentScale);
    const color = lineColors.dynamicFlow;

    // The backend (`campaign_get_trade_flow`) already bundles every city-pair flow
    // onto the coarse ROUTE edges it traverses, so each trunk is one short edge of
    // the road network. We assemble those edges into ONE combined continental
    // network: a node graph → segments (width ∝ volume, so arteries thicken toward
    // the busy hubs/emporia) → degree-2 chains merged into arteries with a SINGLE
    // arrow each (toward the higher-throughput end = the emporium).
    const key = (p: [number, number]) => `${Math.round(p[0])},${Math.round(p[1])}`;
    const nodePos = new Map<string, [number, number]>();
    const nodeThru = new Map<string, number>(); // sum of incident volume
    type E = { a: string; b: string; vol: number; pa: [number, number]; pb: [number, number] };
    const edges: E[] = [];
    let maxVol = 0;
    for (const t of trunks) {
      if (t.points.length < 2 || t.volume <= 0) continue;
      const pa = t.points[0], pb = t.points[1];
      if (this.worldW > 0 && Math.abs(pa[0] - pb[0]) > half) continue; // wrap seam
      const ka = key(pa), kb = key(pb);
      if (ka === kb) continue;
      nodePos.set(ka, pa); nodePos.set(kb, pb);
      nodeThru.set(ka, (nodeThru.get(ka) || 0) + t.volume);
      nodeThru.set(kb, (nodeThru.get(kb) || 0) + t.volume);
      edges.push({ a: ka, b: kb, vol: t.volume, pa, pb });
      if (t.volume > maxVol) maxVol = t.volume;
    }
    if (edges.length === 0 || maxVol <= 0) return;

    const adj = new Map<string, { e: number; other: string }[]>();
    edges.forEach((e, i) => {
      if (!adj.has(e.a)) adj.set(e.a, []);
      if (!adj.has(e.b)) adj.set(e.b, []);
      adj.get(e.a)!.push({ e: i, other: e.b });
      adj.get(e.b)!.push({ e: i, other: e.a });
    });
    const isJunction = (k: string) => (adj.get(k)?.length ?? 0) !== 2;

    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    // 1) every edge as a segment — width & alpha by volume → the combined network.
    let chokeEdge = 0;
    for (let i = 0; i < edges.length; i++) {
      const e = edges[i];
      if (e.vol > edges[chokeEdge].vol) chokeEdge = i;
      const norm = e.vol / maxVol;
      ctx.globalAlpha = 0.30 + 0.55 * norm;
      ctx.strokeStyle = color;
      ctx.lineWidth = Math.max(0.5, (1.0 + norm * 7.0) * inv);
      ctx.beginPath();
      ctx.moveTo(e.pa[0] + 0.5, e.pa[1] + 0.5);
      ctx.lineTo(e.pb[0] + 0.5, e.pb[1] + 0.5);
      ctx.stroke();
    }

    // 2) merge degree-2 chains into arteries; one arrow each, toward the
    //    higher-throughput endpoint (trade converges on the emporia).
    const used = new Array(edges.length).fill(false);
    const extend = (node: string, fromEdge: number): string[] => {
      const seq: string[] = [];
      let curNode = node, curEdge = fromEdge;
      while (!isJunction(curNode)) {
        const cont = adj.get(curNode)!.find((n) => n.e !== curEdge && !used[n.e]);
        if (!cont) break;
        used[cont.e] = true;
        seq.push(cont.other);
        curEdge = cont.e; curNode = cont.other;
      }
      return seq;
    };
    for (let i = 0; i < edges.length; i++) {
      if (used[i]) continue;
      used[i] = true;
      const e = edges[i];
      const left = extend(e.a, i);   // nodes outward from a
      const right = extend(e.b, i);  // nodes outward from b
      const chain = [...left.reverse(), e.a, e.b, ...right];
      const pts = chain.map((k) => nodePos.get(k)!).filter(Boolean);
      if (pts.length < 2) continue;
      // orient toward the higher-throughput endpoint
      const thruA = nodeThru.get(chain[0]) || 0;
      const thruB = nodeThru.get(chain[chain.length - 1]) || 0;
      const ordered = thruB >= thruA ? pts : [...pts].reverse();
      this.drawFlowArrow(ctx, ordered, e.vol / maxVol, inv, color);
    }

    // 3) emporia glow on the top-throughput nodes + a chokepoint diamond.
    const top = [...nodeThru.entries()].sort((x, y) => y[1] - x[1]).slice(0, 3);
    for (const [k] of top) {
      const p = nodePos.get(k); if (!p) continue;
      const r = Math.max(4, 14 * inv);
      const g = ctx.createRadialGradient(p[0] + 0.5, p[1] + 0.5, 0, p[0] + 0.5, p[1] + 0.5, r);
      g.addColorStop(0, color); g.addColorStop(1, "transparent");
      ctx.globalAlpha = 0.30; ctx.fillStyle = g;
      ctx.beginPath(); ctx.arc(p[0] + 0.5, p[1] + 0.5, r, 0, Math.PI * 2); ctx.fill();
    }
    const ce = edges[chokeEdge];
    const cmx = (ce.pa[0] + ce.pb[0]) / 2 + 0.5, cmy = (ce.pa[1] + ce.pb[1]) / 2 + 0.5;
    const d = Math.max(3, 7 * inv);
    ctx.globalAlpha = 0.95; ctx.strokeStyle = "#ff6a4a"; ctx.lineWidth = Math.max(0.6, 1.6 * inv);
    ctx.beginPath();
    ctx.moveTo(cmx, cmy - d); ctx.lineTo(cmx + d, cmy); ctx.lineTo(cmx, cmy + d); ctx.lineTo(cmx - d, cmy);
    ctx.closePath(); ctx.stroke();

    ctx.lineCap = "butt";
    ctx.lineJoin = "miter";
    ctx.globalAlpha = 1;
  }

  /** One arrowhead at the midpoint of a routed polyline, pointing forward (the
   *  polyline is pre-oriented toward the consumer/emporium). Shared by the dynamic
   *  flow arteries. */
  private drawFlowArrow(
    ctx: CanvasRenderingContext2D, pts: [number, number][], norm: number,
    inv: number, color: string,
  ) {
    if (pts.length < 2) return;
    const segs: number[] = []; let total = 0;
    for (let i = 0; i < pts.length - 1; i++) {
      const dd = Math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1]);
      segs.push(dd); total += dd;
    }
    if (total <= 1e-6) return;
    let acc = 0, mi = 0;
    for (; mi < segs.length - 1; mi++) { if (acc + segs[mi] >= total / 2) break; acc += segs[mi]; }
    const seg = Math.max(segs[mi], 1e-6);
    const tt = (total / 2 - acc) / seg;
    const p0 = pts[mi], p1 = pts[mi + 1];
    const mx = p0[0] + (p1[0] - p0[0]) * tt + 0.5, my = p0[1] + (p1[1] - p0[1]) * tt + 0.5;
    let dx = p1[0] - p0[0], dy = p1[1] - p0[1];
    const m = Math.hypot(dx, dy) || 1; dx /= m; dy /= m;
    const hl = Math.max(3, 11 * inv) * (0.7 + 0.6 * norm);
    const px = -dy, py = dx;
    ctx.globalAlpha = 0.95; ctx.fillStyle = color;
    ctx.beginPath();
    ctx.moveTo(mx + dx * hl, my + dy * hl);
    ctx.lineTo(mx - dx * hl * 0.3 + px * hl * 0.7, my - dy * hl * 0.3 + py * hl * 0.7);
    ctx.lineTo(mx - dx * hl * 0.3 - px * hl * 0.7, my - dy * hl * 0.3 - py * hl * 0.7);
    ctx.closePath(); ctx.fill();
  }

  /** Trade-region territories: each hub's hinterland as a translucent square
   *  cell-mask in a per-hub hue, with the hub name at the centroid. Mirrors the
   *  goods overlay so the user sees each region's area. */
  private renderEconRegions(ctx: CanvasRenderingContext2D) {
    for (const r of this.econRegions) {
      if (r.cells.length === 0) continue;
      const hue = (r.hub * 47) % 360; // spread hub hues
      ctx.fillStyle = `hsl(${hue}, 55%, 55%)`;
      ctx.globalAlpha = 0.22;
      for (const [cx, cy] of r.cells) ctx.fillRect(cx, cy, r.cell_size, r.cell_size);
      // Boundary outline (only edges whose neighbour is outside the set).
      const edges = this.maskEdges(r.cells, r.cell_size);
      ctx.globalAlpha = 0.7;
      ctx.strokeStyle = `hsl(${hue}, 70%, 72%)`;
      ctx.lineWidth = Math.max(0.4, 1.0 / Math.sqrt(this.currentScale));
      ctx.beginPath();
      for (let i = 0; i < edges.length; i += 4) {
        ctx.moveTo(edges[i], edges[i + 1]);
        ctx.lineTo(edges[i + 2], edges[i + 3]);
      }
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
  }

  /** The whole House Control layer: grey baseline (every settlement a small grey
   *  dot, every trade route faint grey), then each dominant house's seat city and
   *  the real trade-route paths it runs recoloured in that house's unique colour. */
  private renderHouseControlLayer(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const worldW = this.worldW > 1 ? this.worldW : 0;
    const GREY_DOT = "#69737f";
    const GREY_ROUTE = "rgba(120,132,146,0.30)";
    // Squared cylindrical distance with X-wrap.
    const d2 = (ax: number, ay: number, bx: number, by: number) => {
      let dx = Math.abs(ax - bx);
      if (worldW) dx = Math.min(dx, worldW - dx);
      const dy = ay - by;
      return dx * dx + dy * dy;
    };
    const TOL = 9; // a route endpoint within 3 cells of a seat/partner counts as it.

    // Controlled-settlement → colour lookup (rounded position key). A house
    // colours every city it controls — its seat AND remote outposts it dominates.
    const key = (x: number, y: number) => `${Math.round(x)},${Math.round(y)}`;
    const seatColor = new Map<string, string>();
    // Per house: the cities it handles (controls + partners + seat), for routes.
    const handled: { color: string; pts: [number, number][] }[] = [];
    for (const h of this.houses) {
      if (!h.color) continue;
      for (const c of h.controls ?? []) seatColor.set(key(c[0], c[1]), h.color);
      const pts: [number, number][] = [...(h.controls ?? []), ...(h.partners ?? [])];
      if (h.seat) pts.push(h.seat);
      handled.push({ color: h.color, pts });
    }

    // A FOCUSED house (clicked in the Houses panel): show only ITS sphere, routes
    // and offices, brightly; everything else dims to context grey.
    const sel = this.selectedHouseIdx != null
      ? this.allHouses.find((h) => h.idx === this.selectedHouseIdx) ?? null
      : null;
    const selPts: [number, number][] = sel
      ? [...(sel.controls ?? []), ...(sel.partners ?? []), ...(sel.seat ? [sel.seat] : [])]
      : [];
    const selColor = sel?.color || "#e8c84a";

    const rgba = (hex: string, a: number) => {
      const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex || "");
      return m ? `rgba(${parseInt(m[1], 16)},${parseInt(m[2], 16)},${parseInt(m[3], 16)},${a})` : `rgba(200,180,80,${a})`;
    };

    // ── Territory = a BLURRY influence area: a soft radial disc around the seat and
    //    each city the house holds, tinted its colour. Overlapping discs blend into an
    //    organic cloud (no hard polygon edge). ──
    if (sel) {
      const baseR = (worldW > 1 ? worldW : 360) * 0.05; // ~town hinterland
      const disc = (x: number, y: number, r: number, a: number) => {
        const grd = ctx.createRadialGradient(x + 0.5, y + 0.5, r * 0.15, x + 0.5, y + 0.5, r);
        grd.addColorStop(0, rgba(selColor, a));
        grd.addColorStop(1, rgba(selColor, 0));
        ctx.fillStyle = grd;
        ctx.beginPath(); ctx.arc(x + 0.5, y + 0.5, r, 0, Math.PI * 2); ctx.fill();
      };
      for (const p of selPts) disc(p[0], p[1], baseR, 0.16);
      if (sel.seat) disc(sel.seat[0], sel.seat[1], baseR * 1.5, 0.20); // seat = strongest
    }
    ctx.globalAlpha = 1;
    void convexHull;

    // ── Trade routes (drawn first, under the dots) ──
    const drawPath = (pts: [number, number][], stroke: string, width: number, alpha: number) => {
      ctx.strokeStyle = stroke;
      ctx.lineWidth = width;
      ctx.globalAlpha = alpha;
      let started = false;
      for (let i = 0; i < pts.length; i++) {
        if (i > 0 && worldW && Math.abs(pts[i][0] - pts[i - 1][0]) > worldW * 0.5) started = false; // seam
        if (!started) { ctx.beginPath(); ctx.moveTo(pts[i][0] + 0.5, pts[i][1] + 0.5); started = true; }
        else ctx.lineTo(pts[i][0] + 0.5, pts[i][1] + 0.5);
      }
      ctx.stroke();
    };
    const matches = (pt: [number, number], set: [number, number][]) =>
      set.some((p) => d2(pt[0], pt[1], p[0], p[1]) <= TOL);
    // Focused house: only ITS routes draw, as BRIGHT DASHED lines; the rest go faint.
    if (sel) {
      for (const route of this.tradeRoutes) {
        const pts = route.points;
        if (pts.length < 2) continue;
        const a = pts[0], b = pts[pts.length - 1];
        if (matches(a, selPts) && matches(b, selPts)) {
          ctx.setLineDash([Math.max(3, 4 * inv), Math.max(2, 3 * inv)]);
          drawPath(pts, selColor, Math.max(1.3, 2.4 * inv), 1);
          ctx.setLineDash([]);
        } else {
          drawPath(pts, GREY_ROUTE, Math.max(0.3, 0.6 * inv), 0.4);
        }
      }
      ctx.globalAlpha = 1;
    } else
    for (const route of this.tradeRoutes) {
      const pts = route.points;
      if (pts.length < 2) continue;
      const a = pts[0], b = pts[pts.length - 1];
      // Colour the route if BOTH its endpoints are cities a single house handles
      // (its trade network) — that's a route it runs.
      let col: string | null = null;
      for (const h of handled) {
        if (matches(a, h.pts) && matches(b, h.pts)) { col = h.color; break; }
      }
      if (col) drawPath(pts, col, Math.max(0.8, 1.6 * inv), 0.95);
      else drawPath(pts, GREY_ROUTE, Math.max(0.4, 0.8 * inv), 1);
    }
    ctx.globalAlpha = 1;

    // ── Settlement dots ──
    const dotR = Math.max(0.7, 1.4 * inv);
    for (const s of this.settlements) {
      const c = seatColor.get(key(s.x, s.y));
      ctx.beginPath();
      ctx.fillStyle = c ?? GREY_DOT;
      ctx.arc(s.x + 0.5, s.y + 0.5, c ? dotR * 1.7 : dotR, 0, Math.PI * 2);
      ctx.fill();
      if (c) {
        ctx.lineWidth = Math.max(0.4, 0.9 * inv);
        ctx.strokeStyle = "rgba(8,16,28,0.9)";
        ctx.stroke();
      }
    }

    // ── Focused house: a glowing RED network web from the seat to every city it
    //    works, snapped onto the EXISTING trade routes (`houseNetwork`, built by
    //    `recomputeHouseNetwork` over the drawn road graph). Each entry already falls
    //    back to a straight seat→city line when no road path exists. Under the pins. ──
    if (sel && sel.seat) {
      const dash = Math.max(2, 5 * inv);
      const RED = "rgba(235,70,70,0.95)";
      const drawRedPath = (pts: [number, number][]) => {
        ctx.strokeStyle = RED;
        ctx.lineWidth = Math.max(0.7, 1.6 * inv);
        ctx.setLineDash([dash, dash]);
        let started = false;
        for (let i = 0; i < pts.length; i++) {
          if (i > 0 && worldW && Math.abs(pts[i][0] - pts[i - 1][0]) > worldW * 0.5) started = false; // seam
          if (!started) { ctx.beginPath(); ctx.moveTo(pts[i][0] + 0.5, pts[i][1] + 0.5); started = true; }
          else ctx.lineTo(pts[i][0] + 0.5, pts[i][1] + 0.5);
        }
        ctx.stroke();
        ctx.setLineDash([]);
        // Bold directional chevron at the path MIDPOINT, pointing seat → city, so the
        // web reads clearly as MAIN city ──▶── city of trade (not just a faint line,
        // and not hidden under the destination pin / influence cloud).
        {
          let total = 0;
          const segs: { a: [number, number]; b: [number, number]; len: number }[] = [];
          for (let i = 1; i < pts.length; i++) {
            const a = pts[i - 1], b = pts[i];
            if (worldW && Math.abs(b[0] - a[0]) > worldW * 0.5) continue; // skip seam legs
            const len = Math.hypot(b[0] - a[0], b[1] - a[1]);
            if (len > 1e-3) { segs.push({ a, b, len }); total += len; }
          }
          let acc = 0; const half = total / 2;
          for (const sg of segs) {
            if (acc + sg.len >= half) {
              const t = (half - acc) / sg.len;
              const mx = sg.a[0] + (sg.b[0] - sg.a[0]) * t + 0.5;
              const my = sg.a[1] + (sg.b[1] - sg.a[1]) * t + 0.5;
              let dx = sg.b[0] - sg.a[0], dy = sg.b[1] - sg.a[1];
              const m = Math.hypot(dx, dy) || 1; dx /= m; dy /= m;
              const hl = Math.max(3.5, 6 * inv); const px = -dy, py = dx;
              ctx.fillStyle = RED;
              ctx.beginPath();
              ctx.moveTo(mx + dx * hl, my + dy * hl);
              ctx.lineTo(mx - dx * hl * 0.35 + px * hl * 0.75, my - dy * hl * 0.35 + py * hl * 0.75);
              ctx.lineTo(mx - dx * hl * 0.35 - px * hl * 0.75, my - dy * hl * 0.35 - py * hl * 0.75);
              ctx.closePath(); ctx.fill();
              break;
            }
            acc += sg.len;
          }
        }
        // Solid arrowhead at the destination CITY (the end of the path), pointing
        // along the final leg, so the line reads as flowing seat → city of trade.
        const n = pts.length;
        const a = pts[n - 2], b = pts[n - 1];
        if (worldW && Math.abs(b[0] - a[0]) > worldW * 0.5) return; // final leg crosses the seam
        let dx = b[0] - a[0], dy = b[1] - a[1];
        const m = Math.hypot(dx, dy);
        if (m < 1e-3) return;
        dx /= m; dy /= m;
        const hl = Math.max(2.5, 4.5 * inv);
        const px = -dy, py = dx;
        const tx = b[0] + 0.5, ty = b[1] + 0.5;
        ctx.fillStyle = RED;
        ctx.beginPath();
        ctx.moveTo(tx, ty);
        ctx.lineTo(tx - dx * hl + px * hl * 0.5, ty - dy * hl + py * hl * 0.5);
        ctx.lineTo(tx - dx * hl - px * hl * 0.5, ty - dy * hl - py * hl * 0.5);
        ctx.closePath();
        ctx.fill();
      };
      // Corridor-only: each path in `houseNetwork` is already snapped onto the trade
      // routes (`recomputeHouseNetwork`). We NEVER bridge with a straight slash — a
      // city whose corridor can't be snapped is simply not drawn (matches the merchant
      // and futures layers). The old straight-web fallback is what drew the wrong
      // diagonal lines the user saw.
      for (const path of this.houseNetwork) { if (path.length >= 2) drawRedPath(path); }
    }

    // ── Focused house markers: offices = small SQUARES, BAILOS = circle+triangle
    //    (a governing HQ), the seat (MAIN city) = a large CIRCLE with a white pip. ──
    if (sel) {
      const bailoSet = new Set(
        (sel.active ?? []).filter((c) => c.role === "bailo")
          .map((c) => `${Math.round(c.x)},${Math.round(c.y)}`));
      const offR = Math.max(1.3, 2.2 * inv);
      for (const office of sel.offices ?? []) {
        const pos = office[1];
        if (!pos) continue;
        if (bailoSet.has(`${Math.round(pos[0])},${Math.round(pos[1])}`)) continue; // a Bailo, not a plain office
        ctx.fillStyle = selColor;
        ctx.strokeStyle = "rgba(8,16,28,0.95)";
        ctx.lineWidth = Math.max(0.5, 1.0 * inv);
        ctx.fillRect(pos[0] + 0.5 - offR, pos[1] + 0.5 - offR, offR * 2, offR * 2);
        ctx.strokeRect(pos[0] + 0.5 - offR, pos[1] + 0.5 - offR, offR * 2, offR * 2);
      }
      // BAILOS — a filled circle with a white triangle inside (the HQ glyph).
      const br = Math.max(2.0, 3.0 * inv);
      for (const c of (sel.active ?? []).filter((c) => c.role === "bailo")) {
        const bx = c.x + 0.5, by = c.y + 0.5;
        ctx.fillStyle = selColor;
        ctx.strokeStyle = "rgba(8,16,28,0.95)";
        ctx.lineWidth = Math.max(0.6, 1.2 * inv);
        ctx.beginPath(); ctx.arc(bx, by, br, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
        const t = br * 0.72;
        ctx.fillStyle = "#ffffff";
        ctx.beginPath();
        ctx.moveTo(bx, by - t);
        ctx.lineTo(bx + t * 0.87, by + t * 0.5);
        ctx.lineTo(bx - t * 0.87, by + t * 0.5);
        ctx.closePath(); ctx.fill();
      }
      // SEAT (main city) — a large CIRCLE with a white pip.
      if (sel.seat) {
        const sr = Math.max(2.4, 3.8 * inv);
        const sx = sel.seat[0] + 0.5, sy = sel.seat[1] + 0.5;
        ctx.fillStyle = selColor;
        ctx.strokeStyle = "rgba(8,16,28,0.95)";
        ctx.lineWidth = Math.max(0.7, 1.4 * inv);
        ctx.beginPath(); ctx.arc(sx, sy, sr, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
        ctx.fillStyle = "#ffffff";
        ctx.beginPath(); ctx.arc(sx, sy, sr * 0.4, 0, Math.PI * 2); ctx.fill();
      }
    }
  }

  /** A trade hub: a blue circle. Large hubs (top power tier, ≥4 of 5) get a white
   *  square inside to mark them as the great entrepôts. No stars, no disc. The
   *  hub's name is drawn separately when the "Hub names" overlay is on. */
  private renderPoliticalCenter(ctx: CanvasRenderingContext2D, c: PoliticalCenter) {
    const x = c.x + 0.5;
    const y = c.y + 0.5;
    const stars = Math.max(0, Math.min(5, Math.round(c.stars)));
    const emporium = !!c.emporium;
    const greatest = c.rank === 0;            // the single largest trade hub
    const large = emporium || greatest || stars >= 4;
    // Marker types: the GREATEST hub is a GOLDEN square (the world's "Nobilium"),
    // emporia are RED squares, everything else a blue circle. The user-set hub size
    // multiplier scales every marker; intensity sets a glow halo.
    const hubColor = greatest ? "#f4c430" : emporium ? "#e63030" : HUB_BLUE;
    // The greatest hub is a golden square; emporia are RED TRIANGLES (user request);
    // ordinary hubs stay blue circles.
    const triangle = emporium && !greatest;
    const square = greatest;
    const r = Math.max(1.0, (greatest ? 4.8 : emporium ? 4.2 : large ? 3.4 : 2.2) * this.hubSize / Math.sqrt(this.currentScale));

    // Highlight halo (intensity-driven): a translucent ring around the marker.
    if (this.hubIntensity > 0.01 || square || triangle) {
      ctx.beginPath();
      ctx.arc(x, y, r * (1.8 + this.hubIntensity), 0, Math.PI * 2);
      ctx.fillStyle = hubColor;
      ctx.globalAlpha = 0.10 + 0.30 * Math.min(1, this.hubIntensity + (square || triangle ? 0.45 : 0));
      ctx.fill();
    }

    ctx.lineWidth = Math.max(0.3, 0.9 / Math.sqrt(this.currentScale));
    ctx.strokeStyle = "rgba(8,20,40,0.85)";
    ctx.globalAlpha = 0.95;
    if (square) {
      // Golden filled square for the greatest hub (the world's "Nobilium").
      const s = r * 1.9;
      ctx.fillStyle = hubColor;
      ctx.fillRect(x - s / 2, y - s / 2, s, s);
      ctx.strokeRect(x - s / 2, y - s / 2, s, s);
    } else if (triangle) {
      // Red upward triangle for emporia (the great pass-through entrepôts).
      const s = r * 2.1;
      const h2 = s * 0.866; // equilateral height
      ctx.beginPath();
      ctx.moveTo(x, y - h2 * 0.62);
      ctx.lineTo(x - s / 2, y + h2 * 0.38);
      ctx.lineTo(x + s / 2, y + h2 * 0.38);
      ctx.closePath();
      ctx.fillStyle = hubColor;
      ctx.fill();
      ctx.stroke();
    } else {
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.fillStyle = hubColor;
      ctx.fill();
      ctx.stroke();
      // Great (but not top/emporium) hubs: a white inscribed square.
      if (large) {
        const s = r * 0.92;
        ctx.fillStyle = "#ffffff";
        ctx.globalAlpha = 1;
        ctx.fillRect(x - s / 2, y - s / 2, s, s);
      }
    }
    ctx.globalAlpha = 1;
  }

  /** DLC 3 · one speculation-risk disc. Radius ∝ risk; colour by tier; a pulsing
   *  ring on HIGH-tier (mania-watch) poleis. */
  private renderSpecCenter(ctx: CanvasRenderingContext2D, c: SpecCenter) {
    const x = c.x + 0.5;
    const y = c.y + 0.5;
    const risk = Math.max(0, Math.min(1, c.risk));
    // green (low) → amber (med) → red (high)
    const color = c.tier === "HIGH" ? "#e6303a" : c.tier === "MED" ? "#e0a020" : "#37a05a";
    const baseR = (3 + 16 * risk) / Math.sqrt(this.currentScale);

    // Soft risk disc.
    ctx.beginPath();
    ctx.arc(x, y, baseR, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.globalAlpha = 0.12 + 0.22 * risk;
    ctx.fill();

    // HIGH-tier mania watch: a brighter outer ring.
    if (c.tier === "HIGH") {
      ctx.beginPath();
      ctx.arc(x, y, baseR * 1.25, 0, Math.PI * 2);
      ctx.lineWidth = Math.max(0.4, 1.2 / Math.sqrt(this.currentScale));
      ctx.strokeStyle = color;
      ctx.globalAlpha = 0.85;
      ctx.stroke();
    }

    // Core dot.
    const dot = Math.max(0.8, 2.0 / Math.sqrt(this.currentScale));
    ctx.beginPath();
    ctx.arc(x, y, dot, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.globalAlpha = 1;
    ctx.fill();
    ctx.lineWidth = Math.max(0.3, 0.8 / Math.sqrt(this.currentScale));
    ctx.strokeStyle = "rgba(8,20,40,0.85)";
    ctx.stroke();
    ctx.globalAlpha = 1;
  }

  /** Hub-name labels (drawn when the "hubNames" overlay is on). Kept separate
   *  from the markers so toggling names doesn't change the dots. */
  private renderHubNames(ctx: CanvasRenderingContext2D) {
    const fs = Math.max(5, 10 / this.currentScale);
    ctx.textBaseline = "bottom";
    const haloWidth = Math.max(0.6, 2 / this.currentScale);
    for (const c of this.politicalCenters) {
      if (!c.name) continue;
      const x = c.x + 0.5;
      const y = c.y + 0.5 - Math.max(1.4, 3.6 / Math.sqrt(this.currentScale));
      this.drawLabel(ctx, "settlement", c.name, x, y, fs, "center",
        { halo: "rgba(0,0,0,0.8)", haloWidth });
    }
    ctx.textAlign = "start";
    ctx.textBaseline = "alphabetic";
  }

  /** Settlement-name labels (drawn when the "settlementNames" overlay is on). */
  /** Colony markers + their routed supply/monopoly lanes. Settlement colonies are
   *  violet/purple circles (radius by stage); house outposts are small GREY squares
   *  framed in the owner-house colour. Lanes follow the existing route network. */
  private renderColonies(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    // Markers only — the colony↔metropolis supply lanes were removed (the user doesn't
    // want the mainland-connection lines cluttering the map). Colonies still show their
    // pin; the grain lifeline lives in the Colonial Office panel.
    for (const c of this.colonies) {
      const x = c.x + 0.5, y = c.y + 0.5;
      if (c.kind === 2) {
        // House trade outpost: small grey square, framed in the owner's colour.
        const s = Math.max(1.0, 1.9 * inv);
        ctx.globalAlpha = 0.95;
        ctx.fillStyle = lineColors.houseOutpost;
        ctx.fillRect(x - s, y - s, s * 2, s * 2);
        ctx.strokeStyle = c.ownerColor || "#cccccc";
        ctx.lineWidth = Math.max(0.4, 0.9 * inv);
        ctx.strokeRect(x - s, y - s, s * 2, s * 2);
      } else if (c.kind === 3) {
        // Satellite city (a large metropolis's dependent port/granary/workshop town):
        // a RED disc so it stands apart from ordinary free towns and violet colonies.
        const rr = Math.max(1.1, 1.7 * inv);
        ctx.globalAlpha = 0.92;
        ctx.fillStyle = SATELLITE_COLOR;
        ctx.beginPath();
        ctx.arc(x, y, rr, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = "rgba(0,0,0,0.6)";
        ctx.lineWidth = Math.max(0.3, 0.7 * inv);
        ctx.stroke();
      } else {
        // Settlement colony: violet circle, radius grows with stage.
        const rr = Math.max(1.1, (1.4 + 0.5 * (c.stage || 1)) * inv);
        ctx.globalAlpha = 0.9;
        ctx.fillStyle = lineColors.settlementColony;
        ctx.beginPath();
        ctx.arc(x, y, rr, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = "rgba(0,0,0,0.6)";
        ctx.lineWidth = 0.4;
        ctx.stroke();
      }
    }
    ctx.globalAlpha = 1;
    // Colony/outpost NAME labels, in the colony/owner colour (so they read as colonies,
    // not ordinary cities). Drawn here — the white settlement-name pass skips them.
    const fs = Math.max(4, 8 / this.currentScale);
    ctx.font = `${fs}px sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "bottom";
    ctx.lineWidth = Math.max(0.5, 1.6 / this.currentScale);
    for (const c of this.colonies) {
      if (!c.name) continue;
      const rr = c.kind === 2 ? Math.max(1.0, 1.9 * inv)
        : c.kind === 3 ? Math.max(1.1, 1.7 * inv)
        : Math.max(1.1, (1.4 + 0.5 * (c.stage || 1)) * inv);
      const col = c.kind === 2 ? (c.ownerColor || "#c9a96a")
        : c.kind === 3 ? SATELLITE_COLOR
        : lineColors.settlementColony;
      ctx.strokeStyle = "rgba(0,0,0,0.8)";
      ctx.strokeText(c.name, c.x + 0.5, c.y + 0.5 - rr - 0.8);
      ctx.fillStyle = col;
      ctx.fillText(c.name, c.x + 0.5, c.y + 0.5 - rr - 0.8);
    }
    ctx.textAlign = "start";
    ctx.textBaseline = "alphabetic";
    ctx.lineCap = "butt";
    ctx.lineJoin = "miter";
  }

  private renderSettlementNames(ctx: CanvasRenderingContext2D) {
    const fs = Math.max(4, 8 / this.currentScale);
    ctx.font = `${fs}px sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "bottom";
    ctx.lineWidth = Math.max(0.5, 1.6 / this.currentScale);
    // Colony/outpost names are drawn (in their own colour) by renderColonies — skip
    // them here so they aren't double-drawn in white.
    const colonyKeys = new Set(this.colonies.map((c) => `${Math.round(c.x)},${Math.round(c.y)}`));
    // Draw biggest places first so, when two labels would overlap, the more
    // important city keeps its name and the lesser one is dropped (rather than the
    // two colliding into an unreadable "Rusapolyelgorod" blob).
    const rank: Record<string, number> = { capital: 5, city: 4, town: 3, village: 2, outpost: 1 };
    const ordered = [...this.settlements].sort((a, b) =>
      (rank[b.size] || 0) - (rank[a.size] || 0) || (b.population || 0) - (a.population || 0));
    for (const s of ordered) {
      if (!s.name) continue;
      if (colonyKeys.has(`${Math.round(s.x)},${Math.round(s.y)}`)) continue;
      const radius = SETTLEMENT_SIZES[s.size] || 1;
      const cx = s.x + 0.5, baseY = s.y + 0.5 - radius - 0.6;
      const wLbl = ctx.measureText(s.name).width;
      if (!this.reserveLabel(cx, baseY, wLbl, fs)) continue; // would overlap → skip
      ctx.strokeStyle = "rgba(0,0,0,0.75)";
      ctx.strokeText(s.name, cx, baseY);
      ctx.fillStyle = "#e8e8e0";
      ctx.fillText(s.name, cx, baseY);
    }
    ctx.textAlign = "start";
    ctx.textBaseline = "alphabetic";
  }

  /** Directional trade corridors: each hub→hub corridor drawn as one arrow in the
   *  NET flow direction (the larger of the two directional values), width ∝ the
   *  total value carried. Because corridors are hub-to-hub, the arrow direction can
   *  only change at a hub. */
  private renderCorridors(ctx: CanvasRenderingContext2D) {
    const half = (this.worldW || 1e9) / 2;
    let maxV = 0;
    for (const c of this.corridors) maxV = Math.max(maxV, c.fwd_value + c.bwd_value);
    if (maxV <= 0) return;
    const inv = 1 / Math.sqrt(this.currentScale);
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    for (const c of this.corridors) {
      if (c.points.length < 2) continue;
      const [pa, pb] = c.points;
      if (this.worldW && Math.abs(pa[0] - pb[0]) > half) continue; // wrap seam
      const total = c.fwd_value + c.bwd_value;
      const norm = total / maxV;
      // Net direction: forward (a→b) if fwd ≥ bwd, else reverse.
      const fwd = c.fwd_value >= c.bwd_value;
      const from = fwd ? pa : pb;
      const to = fwd ? pb : pa;
      // RULE: connection lines ALWAYS follow the existing route network — snap onto
      // the already-drawn roads. NEVER draw a straight line: skip if unrouteable.
      const routed = this.routeAlongTradeRoutes(from, to);
      if (!routed || routed.length < 2) continue;
      const path: [number, number][] = routed;
      ctx.globalAlpha = 0.4 + 0.5 * norm;
      ctx.strokeStyle = lineColors.corridor;
      ctx.lineWidth = Math.max(0.5, (1.0 + norm * 5.0) * inv);
      ctx.beginPath();
      ctx.moveTo(path[0][0] + 0.5, path[0][1] + 0.5);
      for (let i = 1; i < path.length; i++) ctx.lineTo(path[i][0] + 0.5, path[i][1] + 0.5);
      ctx.stroke();
      // Single arrowhead at the routed-path midpoint (net direction).
      this.drawFlowArrow(ctx, path, norm, inv, lineColors.corridorArrow);
    }
    ctx.lineCap = "butt";
    ctx.lineJoin = "miter";
    ctx.globalAlpha = 1;
  }

  /** A strategic chokepoint: a pulsing ring + label at the gateway edge. */
  private renderChokepoint(ctx: CanvasRenderingContext2D, cp: EconChokepoint) {
    if (cp.points.length < 2) return;
    const [a, b] = cp.points;
    const mx = (a[0] + b[0]) / 2 + 0.5;
    const my = (a[1] + b[1]) / 2 + 0.5;
    const r = Math.max(1.5, (3 + 6 * cp.share) / Math.sqrt(this.currentScale));

    // Gateway link.
    ctx.beginPath();
    ctx.moveTo(a[0] + 0.5, a[1] + 0.5);
    ctx.lineTo(b[0] + 0.5, b[1] + 0.5);
    ctx.lineWidth = Math.max(0.5, 1.6 / Math.sqrt(this.currentScale));
    ctx.strokeStyle = "rgba(230,90,70,0.85)";
    ctx.stroke();

    // Marker ring.
    ctx.beginPath();
    ctx.arc(mx, my, r, 0, Math.PI * 2);
    ctx.lineWidth = Math.max(0.4, 1.2 / Math.sqrt(this.currentScale));
    ctx.strokeStyle = "rgba(255,140,90,0.95)";
    ctx.fillStyle = "rgba(230,90,70,0.18)";
    ctx.fill();
    ctx.stroke();

    // Label.
    if (cp.name) {
      const fs = Math.max(5, 10 / this.currentScale);
      ctx.font = `${fs}px sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "bottom";
      ctx.fillStyle = "rgba(255,200,170,0.95)";
      ctx.fillText(cp.name, mx, my - r - fs * 0.1);
      ctx.textAlign = "start";
      ctx.textBaseline = "alphabetic";
    }
    ctx.globalAlpha = 1;
  }

  /** A highlighted supply-chain road with a price label at every hop. */
  private renderSupplyChain(ctx: CanvasRenderingContext2D, chain: EconChain) {
    const pts = chain.points;
    const half = (this.worldW || 1e9) / 2;
    const inv = 1 / Math.sqrt(this.currentScale);

    // Road line: dashed with clear direction (origin → consumer). RED for an
    // export/good-reach road, BLUE when it is an inbound IMPORT trace (clicked from
    // a hub's Imports column), so the two directions read distinctly.
    const imp = this.supplyChainImport;
    const lineCol = imp ? "rgba(70,150,255,0.95)" : "rgba(255,45,45,0.95)";
    const headCol = imp ? "rgba(90,165,255,0.98)" : "rgba(255,60,60,0.98)";
    const dash = Math.max(2, 7 * inv);
    const ah = Math.max(5.0, 11 * inv);
    ctx.lineWidth = Math.max(1.2, 3.0 * inv);
    ctx.strokeStyle = lineCol;
    ctx.lineCap = "round";
    ctx.setLineDash([dash, dash * 0.7]);
    for (let i = 0; i < pts.length - 1; i++) {
      const a = pts[i]; const b = pts[i + 1];
      if (this.worldW && Math.abs(a[0] - b[0]) > half) continue;
      ctx.beginPath();
      ctx.moveTo(a[0] + 0.5, a[1] + 0.5);
      ctx.lineTo(b[0] + 0.5, b[1] + 0.5);
      ctx.stroke();
    }
    ctx.setLineDash([]);
    // Directional arrowheads (downstream) on each leg.
    ctx.fillStyle = headCol;
    for (let i = 0; i < pts.length - 1; i++) {
      const a = pts[i]; const b = pts[i + 1];
      if (this.worldW && Math.abs(a[0] - b[0]) > half) continue;
      const dx = b[0] - a[0], dy = b[1] - a[1];
      const len = Math.hypot(dx, dy);
      if (len < 1e-3) continue;
      const nx = dx / len, ny = dy / len, px = -ny, py = nx;
      const cx = b[0] + 0.5 - nx * ah * 0.6, cy = b[1] + 0.5 - ny * ah * 0.6;
      ctx.beginPath();
      ctx.moveTo(cx + nx * ah * 0.6, cy + ny * ah * 0.6);
      ctx.lineTo(cx - nx * ah * 0.4 + px * ah * 0.5, cy - ny * ah * 0.4 + py * ah * 0.5);
      ctx.lineTo(cx - nx * ah * 0.4 - px * ah * 0.5, cy - ny * ah * 0.4 - py * ah * 0.5);
      ctx.closePath();
      ctx.fill();
    }
    ctx.lineCap = "butt";

    // Stop dots + price labels (origin green, consumer orange, hops gold).
    const r = Math.max(0.9, 2.2 * inv);
    const fs = Math.max(5, 10 / this.currentScale);
    ctx.font = `${fs}px sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "bottom";
    for (let i = 0; i < pts.length; i++) {
      const px = pts[i][0] + 0.5; const py = pts[i][1] + 0.5;
      ctx.beginPath();
      ctx.arc(px, py, r, 0, Math.PI * 2);
      ctx.fillStyle = i === 0 ? "rgba(120,220,140,0.95)"
        : (i === pts.length - 1 ? "rgba(255,140,90,0.95)" : "rgba(255,220,120,0.95)");
      ctx.fill();
      ctx.lineWidth = Math.max(0.3, 0.7 * inv);
      ctx.strokeStyle = "rgba(0,0,0,0.6)";
      ctx.stroke();
      const price = chain.stops[i]?.price ?? 0;
      ctx.fillStyle = "rgba(255,240,200,0.97)";
      ctx.fillText(`${price.toFixed(price < 10 ? 1 : 0)}×`, px, py - r - fs * 0.1);
    }
    ctx.textAlign = "start";
    ctx.textBaseline = "alphabetic";
    ctx.globalAlpha = 1;
  }

  /** Per-good reach: draw every route carrying the selected good (gold polylines,
   *  wrap-seam aware) and ring each hub it reaches. */
  private renderReachNetwork(ctx: CanvasRenderingContext2D) {
    const half = (this.worldW || 1e9) / 2;
    const inv = 1 / Math.sqrt(this.currentScale);
    ctx.save();
    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    // Dashed bright-red corridors for the selected good.
    const dash = Math.max(2, 7 * inv);
    ctx.lineWidth = Math.max(0.8, 2.2 * inv);
    ctx.strokeStyle = "rgba(255,45,45,0.95)";
    ctx.setLineDash([dash, dash * 0.7]);
    for (const ch of this.reachChains) {
      const pts = ch.points;
      for (let i = 0; i < pts.length - 1; i++) {
        const a = pts[i], b = pts[i + 1];
        if (this.worldW && Math.abs(a[0] - b[0]) > half) continue; // seam
        ctx.beginPath();
        ctx.moveTo(a[0] + 0.5, a[1] + 0.5);
        ctx.lineTo(b[0] + 0.5, b[1] + 0.5);
        ctx.stroke();
      }
    }
    ctx.setLineDash([]);

    // Arrowheads pointing DOWNSTREAM (origin → terminal hub). Enlarged (user
    // asked for clearly bigger direction arrows) and placed BOTH at the
    // downstream hub and at the midpoint of each leg, so the flow direction reads
    // at a glance even on long corridors.
    const ah = Math.max(5.0, 11 * inv);
    ctx.fillStyle = "rgba(255,60,60,0.98)";
    const drawHead = (cx: number, cy: number, nx: number, ny: number) => {
      const px = -ny, py = nx;
      ctx.beginPath();
      ctx.moveTo(cx + nx * ah * 0.6, cy + ny * ah * 0.6);
      ctx.lineTo(cx - nx * ah * 0.4 + px * ah * 0.5, cy - ny * ah * 0.4 + py * ah * 0.5);
      ctx.lineTo(cx - nx * ah * 0.4 - px * ah * 0.5, cy - ny * ah * 0.4 - py * ah * 0.5);
      ctx.closePath();
      ctx.fill();
    };
    for (const ch of this.reachChains) {
      const pts = ch.points;
      for (let i = 0; i < pts.length - 1; i++) {
        const a = pts[i], b = pts[i + 1];
        if (this.worldW && Math.abs(a[0] - b[0]) > half) continue;
        const dx = b[0] - a[0], dy = b[1] - a[1];
        const len = Math.hypot(dx, dy);
        if (len < 1e-3) continue;
        const nx = dx / len, ny = dy / len;
        // Head just shy of the downstream hub (points INTO it).
        drawHead(b[0] + 0.5 - nx * ah * 0.6, b[1] + 0.5 - ny * ah * 0.6, nx, ny);
        // Mid-leg head for long corridors.
        if (len > ah * 3) drawHead(a[0] + 0.5 + nx * len * 0.5, a[1] + 0.5 + ny * len * 0.5, nx, ny);
      }
    }

    // Ring every hub the good reaches (red, terminal markets brighter).
    const r = Math.max(1.4, 3.4 * inv);
    ctx.lineWidth = Math.max(0.6, 1.6 * inv);
    ctx.strokeStyle = "rgba(255,120,120,0.98)";
    ctx.fillStyle = "rgba(255,45,45,0.22)";
    for (const [hx, hy] of this.reachHubs) {
      ctx.beginPath();
      ctx.arc(hx + 0.5, hy + 0.5, r, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
    ctx.restore();
  }

  /** Small arrow for wind overlay */
  private renderArrow(ctx: CanvasRenderingContext2D, x: number, y: number, vx: number, vy: number, color: string, alpha: number) {
    const mag = Math.sqrt(vx * vx + vy * vy);
    if (mag < 0.01) return;
    const nx = vx / mag;
    const ny = vy / mag;
    const len = Math.min(mag * 8, 12);

    // Center the arrow on its cell rather than anchoring the tail there. With
    // the tail at the cell the whole arrow body sat to one side (the poleward
    // side for westerlies), so the wind belts read as shifted off their
    // latitude. Centering keeps each arrow visually on its own latitude line.
    const cx = x + 0.5;
    const cy = y + 0.5;
    const x1 = cx - nx * len * 0.5;
    const y1 = cy - ny * len * 0.5;
    const x2 = cx + nx * len * 0.5;
    const y2 = cy + ny * len * 0.5;

    ctx.globalAlpha = alpha;
    ctx.strokeStyle = color;
    ctx.lineWidth = 0.6;

    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();

    const headLen = len * 0.3;
    const perpX = -ny;
    const perpY = nx;
    ctx.beginPath();
    ctx.moveTo(x2, y2);
    ctx.lineTo(x2 - nx * headLen + perpX * headLen * 0.4, y2 - ny * headLen + perpY * headLen * 0.4);
    ctx.moveTo(x2, y2);
    ctx.lineTo(x2 - nx * headLen - perpX * headLen * 0.4, y2 - ny * headLen - perpY * headLen * 0.4);
    ctx.stroke();

    ctx.globalAlpha = 1;
  }

  clear() {
    this.rivers = [];
    this.lakes = [];
    this.settlements = [];
    this.windData = null;
    this.currentLines = [];
    this.tradeRoutes = [];
    this.travelRoute = [];
    this.ridgeSketch = [];
    this.riverHighlight = new Set();
    this.riverHighlightColors = {};
    this.lakeHighlight = -1;
    this.goodScarcity = [];
    this.toponyms = [];
    this.riverBreaks = [];
    this.fisheryBanks = [];
    this.sharkZones = [];
    this.shipwormZones = [];
    this.stormZones = [];
    this.monsoonZones = [];
    this.reefZones = [];
    this.goodRegions = [];
    this.tradeTrunks = [];
    this.dynamicTrunks = [];
    this.tradeCorridorList = [];
    this.expeditions = [];
    this.expeditionFails = [];
    this.politicalCenters = [];
    this.specCenters = [];
    this.houses = [];
    this.chokepoints = [];
    this.corridors = [];
    this.econRegions = [];
    this.reachChains = [];
    this.reachHubs = [];
    this.supplyChain = null;
    this.latLinesData = null;
    this.climateBands = null;
  }
}
