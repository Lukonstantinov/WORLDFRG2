import type { RiverData, LakeData, Settlement, VectorSample, Streamline, TradeRoute, FisheryBank, SharkZone, GoodRegion, CultureRegion, TradeTrunk, PoliticalCenter, EconChokepoint, EconChain, EconRegion, EconCorridor, HouseBrief, MerchantRoute, FuturesLane, SpecCenter, CoinUseCity } from "../types";
import { GOOD_DEFS, goodOverlayKey, goodSubtypes, type SubtypeDef } from "../goods";
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
const LAKE_COLOR = "rgba(51, 153, 221, 0.7)";

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

export class OverlayManager {
  private rivers: RiverData[] = [];
  private lakes: LakeData[] = [];
  private settlements: Settlement[] = [];
  private colonies: ColonyMarker[] = [];
  /** Atlas 2.0 · per-hub yearly trade throughput for the Trade Heat overlay. */
  private heatPoints: { x: number; y: number; v: number }[] = [];
  /** Atlas 2.0 · named trade basins (member positions hulled + labelled). */
  private basins: { name: string; pts: [number, number][]; cx: number; cy: number }[] = [];
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
  /** #37 · per-hub local price premium for the selected good (1 = par with the
   *  world base value; <1 cheap/abundant, >1 dear/scarce). */
  private goodScarcity: { x: number; y: number; premium: number }[] = [];
  /** #26 · named geographic features (rivers/mountains/lakes/regions). */
  private toponyms: { kind: string; name: string; x: number; y: number }[] = [];
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
  /** Transient highlight pin (searched settlement) in world coords. */
  private searchPin: { wx: number; wy: number } | null = null;
  /** Per-good display metadata (icon/color) from the active editable spec; falls
   *  back to the static GOOD_DEFS when absent. */
  private goodMeta: Map<string, { icon: string; color: string }> | null = null;
  private tradeTrunks: TradeTrunk[] = [];
  private dynamicTrunks: TradeTrunk[] = [];
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

  private visibility: Record<string, boolean> = {
    rivers: true, lakes: true, settlements: true,
    markers: false, wind: false, currents: false, latLines: false,
    tradeRoutes: false, fisheryBanks: false,
    sharkZones: false, shipwormZones: false, stormZones: false, monsoonZones: false, reefZones: false, tradeFlows: false,
    politicalInfluence: false, chokepoints: false, tradeCorridors: false,
    speculation: false,
    houseControl: false, merchantRoutes: false, futures: false,
    hubNames: false, settlementNames: false, tradeRegions: false, cultures: false,
  };

  private currentScale = 1;
  private worldW = 0;
  /** Cached region-mask boundary edges, keyed by the cell array of each region/
   *  zone (replaced wholesale on each data fetch, so the WeakMap auto-evicts).
   *  Each entry is a flat [x1,y1,x2,y2, …] list — built once, not per frame. */
  private edgeCache = new WeakMap<object, number[]>();
  /** Cached subtype-split edges, keyed by each region's `subtypes` array. */
  private subtypeEdgeCache = new WeakMap<object, number[]>();

  drawRivers(rivers: RiverData[]) { this.rivers = rivers; }
  drawLakes(lakes: LakeData[]) { this.lakes = lakes; }
  drawSettlements(settlements: Settlement[]) { this.settlements = settlements; }
  drawColonies(colonies: ColonyMarker[]) { this.colonies = colonies; }
  /** Atlas 2.0 · set the Trade Heat points (hub position + yearly throughput). */
  drawTradeHeat(pts: { x: number; y: number; v: number }[]) { this.heatPoints = pts; }
  /** Atlas 2.0 · set the named trade basins. */
  drawTradeBasins(b: { name: string; pts: [number, number][]; cx: number; cy: number }[]) { this.basins = b; }
  /** Atlas 2.0 · set the refugee roads (age01 0 = fresh, 1 = fully faded). */
  drawMigrations(m: { fx: number; fy: number; tx: number; ty: number; age: number }[]) { this.migrations = m; }

  // ── Route-bound migration overlay (dots · ribbon · focus) ──
  private migrationRoutes: { path: [number, number][]; culture: string; volume: number; to: number; age: number }[] = [];
  private migrationMode: "dots" | "ribbon" | "focus" = "ribbon";
  private migrationFocusHub: number | null = null;
  private migMaxVol = 1;
  /** Set the reworked migration flows (polylines along trade routes + culture + volume). */
  setMigrationRoutes(r: { path: [number, number][]; culture: string; volume: number; to: number; age: number }[]) {
    this.migrationRoutes = r;
    this.migMaxVol = r.reduce((m, x) => Math.max(m, x.volume), 1);
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
      ctx.font = `600 ${fs}px Georgia, 'Times New Roman', serif`;
      ctx.textAlign = "center";
      ctx.lineWidth = Math.max(1.5, 2.5 * inv);
      ctx.strokeStyle = "rgba(0,0,0,0.7)";
      ctx.strokeText(b.name, b.cx + 0.5, b.cy - grow - 2 * inv);
      ctx.fillStyle = col;
      ctx.fillText(b.name, b.cx + 0.5, b.cy - grow - 2 * inv);
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
      if (r.path.length < 2 || !spanOk(r.path)) continue;
      if (this.migrationMode === "focus" && focus != null && r.to !== focus) continue;
      const focused = this.migrationMode === "focus" && focus != null && r.to === focus;
      const fade = Math.max(0.12, 1 - r.age / 6); // ~6y lifetime
      const col = this.cultureHue(r.culture);
      const volN = Math.min(1, r.volume / this.migMaxVol);

      if (this.migrationMode === "dots") {
        // Markers spaced along the routed polyline.
        ctx.strokeStyle = this.rgba(col, fade * 0.28);
        ctx.lineWidth = Math.max(0.4, 0.8 * inv);
        this.tracePath(ctx, r.path);
        ctx.stroke();
        const step = 4;
        for (let i = 1; i < r.path.length; i++) {
          const [x0, y0] = r.path[i - 1], [x1, y1] = r.path[i];
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
        // Ribbon (and focus) — a stroked path whose width tracks volume.
        ctx.strokeStyle = this.rgba(col, focused ? Math.min(1, fade + 0.25) : fade * 0.85);
        ctx.lineWidth = Math.max(0.6, (focused ? 2.2 : 1.0 + volN * 2.4) * inv);
        ctx.lineJoin = "round"; ctx.lineCap = "round";
        this.tracePath(ctx, r.path);
        ctx.stroke();
        // Arrowhead at the destination.
        const n = r.path.length;
        const [px, py] = r.path[n - 2], [qx, qy] = r.path[n - 1];
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

  /** Set (or clear with []) the per-hub scarcity discs for the selected good. */
  drawGoodScarcity(cities: { x: number; y: number; premium: number }[]) {
    this.goodScarcity = cities;
  }

  /** Set (or clear with []) the named geographic features to label. */
  drawToponyms(t: { kind: string; name: string; x: number; y: number }[]) {
    this.toponyms = t;
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

  /** Drop a transient highlight pin at a world cell (searched settlement). */
  setSearchPin(wx: number, wy: number) {
    this.searchPin = { wx, wy };
  }
  clearSearchPin() {
    this.searchPin = null;
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

  /** Render all overlays to a 2D context (called within viewport transform) */
  render(ctx: CanvasRenderingContext2D) {
    // Trade-region territories first (under everything else) so markers/routes
    // stay legible on top.
    if (this.visibility.tradeRegions && this.econRegions.length > 0) {
      this.renderEconRegions(ctx);
    }

    if (this.visibility.lakes && this.lakes.length > 0) {
      ctx.fillStyle = LAKE_COLOR;
      for (const lake of this.lakes) {
        for (const [x, y] of lake.cells) {
          ctx.fillRect(x, y, 1, 1);
        }
      }
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
      for (const river of this.rivers) {
        if (river.points.length < 2) continue;
        // Width scales with the river's Strahler order (headwater creek → great
        // trunk): order 1 ≈ 1 px, a high-order trunk ≈ 2.6 px, zoom-compensated.
        // Width never balloons into a blob.
        const ord = river.order ?? (river.major ? 4 : 1);
        const baseW = 0.9 + Math.min(ord, 6) * 0.32;
        const riverW = Math.max(0.8, Math.min(3, baseW) * inv);
        ctx.strokeStyle = riverShade(river.major);
        ctx.lineWidth = riverW;
        // Catmull-Rom smoothing so the drainage lines read as natural meanders
        // rather than the 8-neighbour grid staircase of the raw cell path.
        strokeSmoothPath(ctx, river.points);
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
      ctx.font = `${fs}px serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      for (const r of this.cultureRegions) {
        ctx.lineWidth = Math.max(0.6, 2.4 / this.currentScale);
        ctx.strokeStyle = "rgba(0,0,0,0.75)";
        ctx.strokeText(r.label, r.x, r.y);
        ctx.fillStyle = "#f4ecd6";
        ctx.fillText(r.label, r.x, r.y);
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

    // #37 · per-good scarcity: graduated discs at each hub, green where the good
    // is cheap/abundant through to red where it is dear/scarce.
    if (this.visibility.goodScarcity && this.goodScarcity.length > 0) {
      this.renderGoodScarcity(ctx);
    }

    // #26 · geographic toponyms: culture-styled labels for rivers/peaks/lakes/regions.
    if (this.visibility.toponyms && this.toponyms.length > 0) {
      this.renderToponyms(ctx);
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

    // Coin-usage overlay: tint settlements that settle in the selected coin.
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

  /** #26 · draw toponym labels. Regions read as faint uppercase tracking; rivers/
   *  peaks/lakes get a small kind-coloured dot + italic-ish name. Sizes are
   *  zoom-compensated and kept legible with a dark halo. */
  private renderToponyms(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const COLORS: Record<string, string> = {
      river: "#7fc8e0", mountain: "#d8c0a0", lake: "#9ad0e8", region: "#caa6e0",
    };
    ctx.textBaseline = "middle";
    ctx.lineJoin = "round";
    for (const t of this.toponyms) {
      const region = t.kind === "region";
      const fs = Math.max(6, Math.min(16, (region ? 13 : 9) * inv));
      ctx.font = `${region ? "700 " : ""}${fs}px -apple-system, Segoe UI, sans-serif`;
      const label = region ? t.name.toUpperCase() : t.name;
      const col = COLORS[t.kind] ?? "#cfe2f6";
      const dotR = Math.max(0.5, 1.4 * inv);
      const tx = t.x + 0.5 + (region ? 0 : dotR + 1.5 * inv);
      // Non-region features get a small locator dot.
      if (!region) {
        ctx.beginPath();
        ctx.arc(t.x + 0.5, t.y + 0.5, dotR, 0, Math.PI * 2);
        ctx.fillStyle = col;
        ctx.fill();
      }
      ctx.textAlign = region ? "center" : "left";
      ctx.lineWidth = Math.max(0.6, 2.2 * inv);
      ctx.strokeStyle = "rgba(6,12,18,0.85)";
      ctx.strokeText(label, tx, t.y + 0.5);
      ctx.fillStyle = region ? "rgba(202,166,224,0.85)" : col;
      ctx.fillText(label, tx, t.y + 0.5);
    }
    ctx.textAlign = "left";
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
  private renderCoinUsage(ctx: CanvasRenderingContext2D) {
    const inv = 1 / Math.sqrt(this.currentScale);
    const cities = this.coinUse.filter((u) => u.coin === this.coinOverlayHub);
    let maxVol = 0;
    for (const u of cities) maxVol = Math.max(maxVol, u.volume);
    const base = Math.max(2, 4 * inv);
    ctx.lineCap = "round";
    for (const u of cities) {
      const t = maxVol > 0 ? u.volume / maxVol : 0;
      const r = u.mint ? base * 2.0 : base * (1.0 + 0.8 * t);
      const cx = u.x + 0.5, cy = u.y + 0.5;
      if (u.reserve_reach) {
        ctx.strokeStyle = "#37a05a"; ctx.globalAlpha = 0.9;
        ctx.lineWidth = Math.max(0.8, 1.4 * inv);
        ctx.setLineDash([Math.max(1.5, 3 * inv), Math.max(1.5, 2 * inv)]);
        ctx.beginPath(); ctx.arc(cx, cy, r + 3 * inv, 0, Math.PI * 2); ctx.stroke();
        ctx.setLineDash([]);
      }
      ctx.fillStyle = u.mint ? "#f0d77a" : t > 0.55 ? "#c9a227" : "#7a6320";
      ctx.globalAlpha = u.mint ? 1 : 0.55 + 0.4 * t;
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fill();
      if (u.mint) {
        ctx.strokeStyle = "#fff3c0"; ctx.globalAlpha = 0.9; ctx.lineWidth = Math.max(0.6, 1 * inv);
        ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.stroke();
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
    ctx.font = `${fs}px sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "bottom";
    ctx.lineWidth = Math.max(0.6, 2 / this.currentScale);
    for (const c of this.politicalCenters) {
      if (!c.name) continue;
      const x = c.x + 0.5;
      const y = c.y + 0.5 - Math.max(1.4, 3.6 / Math.sqrt(this.currentScale));
      ctx.strokeStyle = "rgba(0,0,0,0.8)";
      ctx.strokeText(c.name, x, y);
      ctx.fillStyle = "#dCEBFF";
      ctx.fillText(c.name, x, y);
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
    for (const s of this.settlements) {
      if (!s.name) continue;
      if (colonyKeys.has(`${Math.round(s.x)},${Math.round(s.y)}`)) continue;
      const radius = SETTLEMENT_SIZES[s.size] || 1;
      ctx.strokeStyle = "rgba(0,0,0,0.75)";
      ctx.strokeText(s.name, s.x + 0.5, s.y + 0.5 - radius - 0.6);
      ctx.fillStyle = "#e8e8e0";
      ctx.fillText(s.name, s.x + 0.5, s.y + 0.5 - radius - 0.6);
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
    this.goodScarcity = [];
    this.toponyms = [];
    this.fisheryBanks = [];
    this.sharkZones = [];
    this.shipwormZones = [];
    this.stormZones = [];
    this.monsoonZones = [];
    this.reefZones = [];
    this.goodRegions = [];
    this.tradeTrunks = [];
    this.dynamicTrunks = [];
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
  }
}
