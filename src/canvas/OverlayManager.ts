import type { RiverData, LakeData, Settlement, VectorSample, Streamline, TradeRoute, FisheryBank, SharkZone, GoodRegion, CultureRegion, TradeTrunk, PoliticalCenter, EconChokepoint, EconChain, EconRegion, EconCorridor, HouseBrief, MerchantRoute } from "../types";
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
const TIER_ALPHA = [0, 0.32, 0.55, 0.78, 1.0]; // index 1..4

const GOOD_BY_NAME = new Map(GOOD_DEFS.map((g) => [g.name, g]));
const SHARK_COLOR = "#e04040";
const SHIPWORM_COLOR = "#b98a4a";
const STORM_COLOR = "#c050d0";
const MONSOON_COLOR = "#3a9ad0";
const REEF_COLOR = "#30c0b0";
const TRADE_TRUNK = "#e0c060"; // major bundled commodity-flow trunk (amber)
const TRADE_TRUNK_MINOR = "#b8a878"; // minor/low-volume trunk (muted amber)
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
const LAKE_COLOR = "rgba(51, 153, 221, 0.7)";

const SETTLEMENT_COLORS: Record<string, string> = {
  capital: "#ffd700",
  city: "#ff8844",
  town: "#cccccc",
  village: "#88aa88",
  outpost: "#111111", // trade posts: small black dots
};

const SETTLEMENT_SIZES: Record<string, number> = {
  capital: 3,
  city: 2.2,
  town: 1.6,
  village: 1,
  outpost: 0.8, // small
};

const WARM_CURRENT = "#ee5533";
const COLD_CURRENT = "#3399ee";
const NEUTRAL_CURRENT = "#9bb0c0";
const WIND_COLOR = "#aaccee";
const LAT_LINE_COLOR = "#cccc66";
const TRADE_LAND = "#caa15a"; // overland caravan route (tan)
const TRADE_SEA = "#7fd0d8";  // maritime route (pale cyan)
const TRADE_RIVER = "#9fe07a"; // river-following inland route (green)
const FISHERY_BANK = "#39d3c0"; // grand-bank fishing ground (teal)

export class OverlayManager {
  private rivers: RiverData[] = [];
  private lakes: LakeData[] = [];
  private settlements: Settlement[] = [];
  private windData: { samples: VectorSample[]; gridW: number; gridH: number } | null = null;
  private currentLines: Streamline[] = [];
  private tradeRoutes: TradeRoute[] = [];
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
  private merchantRoutes: MerchantRoute[] = [];
  private politicalCenters: PoliticalCenter[] = [];
  private houses: HouseBrief[] = [];
  private allHouses: HouseBrief[] = [];
  private selectedHouseIdx: number | null = null;
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
    houseControl: false, merchantRoutes: false,
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

  drawWindArrows(data: VectorSample[], gridW: number, gridH: number) {
    this.windData = { samples: data, gridW, gridH };
  }

  drawCurrentStreamlines(lines: Streamline[]) {
    this.currentLines = lines;
  }

  drawTradeRoutes(routes: TradeRoute[]) {
    this.tradeRoutes = routes;
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

  drawMerchantRoutes(routes: MerchantRoute[], gridW: number) {
    this.merchantRoutes = routes;
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

  drawPolitical(centers: PoliticalCenter[]) {
    this.politicalCenters = centers;
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
      for (const river of this.rivers) {
        if (river.points.length < 2) continue;
        // Every river renders thin: ordinary streams ~1 px, major trunks ~2.5 px,
        // zoom-compensated. Width never balloons into a blob.
        const baseW = river.major ? 2.5 : 1.1;
        const riverW = Math.max(0.8, Math.min(3, baseW) * inv);
        ctx.strokeStyle = riverShade(river.major);
        ctx.lineWidth = riverW;
        ctx.beginPath();
        ctx.moveTo(river.points[0][0] + 0.5, river.points[0][1] + 0.5);
        for (let i = 1; i < river.points.length; i++) {
          ctx.lineTo(river.points[i][0] + 0.5, river.points[i][1] + 0.5);
        }
        ctx.stroke();
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

    if (this.visibility.tradeRoutes && this.tradeRoutes.length > 0) {
      for (const route of this.tradeRoutes) {
        this.renderTradeRoute(ctx, route);
      }
    }

    // Merchant layer: live family/guild routes coloured by the owning house.
    if (this.visibility.merchantRoutes && this.merchantRoutes.length > 0) {
      this.renderMerchantRoutes(ctx);
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

    // Merchant-family control: settlements a house dominates (>=50% of local
    // trade) and the trade routes it runs are tinted that house's unique colour;
    // every other settlement is a small grey dot and every other route is grey.
    if (this.visibility.houseControl && this.settlements.length > 0) {
      this.renderHouseControlLayer(ctx);
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

    if (this.visibility.settlements && this.settlements.length > 0) {
      for (const s of this.settlements) {
        // Dot scales continuously with population (log) on top of the tier base,
        // so the emergent carrying-capacity / trade hierarchy reads on the map.
        const popf = Math.min(1.7, 0.6 + Math.log10(Math.max(s.population, 100)) / 5);
        const radius = (SETTLEMENT_SIZES[s.size] || 1) * popf;
        const color = SETTLEMENT_COLORS[s.size] || "#cccccc";

        ctx.beginPath();
        ctx.arc(s.x + 0.5, s.y + 0.5, radius, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.globalAlpha = 0.9;
        ctx.fill();
        ctx.strokeStyle = "rgba(0,0,0,0.6)";
        ctx.lineWidth = 0.3;
        ctx.stroke();
        ctx.globalAlpha = 1;
      }
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

    const color = route.kind === 1 ? TRADE_SEA : route.kind === 2 ? TRADE_RIVER : TRADE_LAND;
    // Minor connector roads (a lesser town's single link) are drawn thinner and
    // fainter than the major inter-hub routes, so every settlement is on the
    // network without the small roads overpowering the trunks.
    const lineWidth = Math.max(0.4, (route.minor ? 0.8 : 1.6) / Math.sqrt(this.currentScale));
    const dash = Math.max(1.5, 4 / Math.sqrt(this.currentScale));

    ctx.globalAlpha = route.minor ? 0.5 : 0.8;
    ctx.strokeStyle = color;
    ctx.lineWidth = lineWidth;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.setLineDash([dash, dash]);

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

    // Fill every coarse cell — by subtype tint if present, else the good colour;
    // opacity steps through 4 discrete quality tiers from per-cell abundance, so
    // rich deposits read solid and negligible ones nearly transparent.
    for (let i = 0; i < cells.length; i++) {
      const [cx, cy] = cells[i];
      ctx.fillStyle = hasSub ? (subPalette![subtypes![i]]?.color ?? color) : color;
      if (hasVals) {
        const tier = Math.min(4, Math.max(1, Math.ceil((values![i] / 255) * 4)));
        ctx.globalAlpha = alpha * TIER_ALPHA[tier];
      } else {
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
  private renderMerchantRoutes(ctx: CanvasRenderingContext2D) {
    let maxVol = 0;
    for (const r of this.merchantRoutes) maxVol = Math.max(maxVol, r.volume);
    if (maxVol <= 0) return;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    const dash = Math.max(1.5, 3 / Math.sqrt(this.currentScale));
    for (const r of this.merchantRoutes) {
      const ax = r.a[0] + 0.5, ay = r.a[1] + 0.5, bx = r.b[0] + 0.5, by = r.b[1] + 0.5;
      if (this.worldW > 0 && Math.abs(ax - bx) > this.worldW / 2) continue; // wrap seam
      const norm = r.volume / maxVol;
      ctx.globalAlpha = 0.5 + 0.4 * norm;
      ctx.strokeStyle = r.color || "#cccccc";
      ctx.lineWidth = Math.max(0.5, (0.8 + norm * 4.0) / Math.sqrt(this.currentScale));
      ctx.setLineDash(r.sea ? [] : [dash, dash]);
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      ctx.lineTo(bx, by);
      ctx.stroke();
      ctx.setLineDash([]);
      // Origin dot at the FOUNDER city (a); destination gets an arrowhead so the
      // route reads directionally — goods flow FROM the founder's seat outward.
      const dotR = Math.max(0.8, 1.6 / Math.sqrt(this.currentScale));
      ctx.globalAlpha = 0.85;
      ctx.fillStyle = r.color || "#cccccc";
      ctx.beginPath(); ctx.arc(ax, ay, dotR, 0, Math.PI * 2); ctx.fill();
      // Direction arrowheads sitting ON the route line, pointing a → b. Several are
      // spaced along the segment so the direction is legible even when zoomed out.
      let dx = bx - ax, dy = by - ay;
      const len = Math.hypot(dx, dy);
      if (len > 0.001) {
        dx /= len; dy /= len;
        const px = -dy, py = dx;
        const hl = Math.max(2.2, (3.5 + norm * 4.0) / Math.sqrt(this.currentScale));
        const heads = Math.max(1, Math.min(4, Math.floor(len / (hl * 3))));
        ctx.fillStyle = r.color || "#cccccc";
        for (let i = 1; i <= heads; i++) {
          const t = i / (heads + 1);
          const hx = ax + dx * len * t, hy = ay + dy * len * t;
          ctx.beginPath();
          ctx.moveTo(hx, hy);
          ctx.lineTo(hx - dx * hl + px * hl * 0.5, hy - dy * hl + py * hl * 0.5);
          ctx.lineTo(hx - dx * hl - px * hl * 0.5, hy - dy * hl - py * hl * 0.5);
          ctx.closePath();
          ctx.fill();
        }
      }
    }
    ctx.globalAlpha = 1;
    ctx.setLineDash([]);
  }

  private renderTradeTrunks(ctx: CanvasRenderingContext2D) {
    let maxVol = 0;
    for (const t of this.tradeTrunks) maxVol = Math.max(maxVol, t.volume);
    if (maxVol <= 0) return;

    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    const dash = Math.max(1.5, 3.5 / Math.sqrt(this.currentScale));
    // Two tiers so the main arteries stand out from the feeder routes: major
    // corridors (high volume) are solid, bright and thick; minor ones are thin,
    // muted and dashed. Each trunk carries a direction arrowhead (toward the
    // consuming hub) and the major arteries are labelled (Spice Road / Silk Road).
    const labels: { x: number; y: number; text: string }[] = [];
    for (const t of this.tradeTrunks) {
      const pts = t.points;
      if (pts.length < 2) continue;
      const a = pts[0], b = pts[1];
      // Skip edges spanning the cylindrical wrap seam.
      if (this.worldW > 0 && Math.abs(a[0] - b[0]) > this.worldW / 2) continue;
      const norm = t.volume / maxVol;
      const major = norm >= 0.45;
      ctx.globalAlpha = major ? 0.85 : 0.5;
      ctx.strokeStyle = major ? TRADE_TRUNK : TRADE_TRUNK_MINOR;
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
        ctx.fillStyle = major ? TRADE_TRUNK : TRADE_TRUNK_MINOR;
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

    // ── Sphere of business: a translucent convex hull around the relevant house(s)'
    //    cities, partners and seat, tinted their colour (drawn first, under all). ──
    const hullSrc = sel ? [{ color: selColor, pts: selPts }] : handled;
    for (const h of hullSrc) {
      const hull = convexHull(h.pts);
      if (hull.length < 3) continue;
      let spansSeam = false;
      for (let i = 0; i < hull.length && !spansSeam; i++) {
        const a = hull[i], b = hull[(i + 1) % hull.length];
        if (worldW && Math.abs(a[0] - b[0]) > worldW * 0.5) spansSeam = true;
      }
      if (spansSeam) continue; // skip seam-spanning hulls (rare; avoids wrap artifacts)
      ctx.beginPath();
      ctx.moveTo(hull[0][0] + 0.5, hull[0][1] + 0.5);
      for (let i = 1; i < hull.length; i++) ctx.lineTo(hull[i][0] + 0.5, hull[i][1] + 0.5);
      ctx.closePath();
      ctx.fillStyle = h.color;
      ctx.globalAlpha = sel ? 0.16 : 0.10;
      ctx.fill();
      ctx.globalAlpha = sel ? 0.6 : 0.34;
      ctx.lineWidth = Math.max(0.7, (sel ? 1.6 : 1.2) * inv);
      ctx.strokeStyle = h.color;
      ctx.stroke();
    }
    ctx.globalAlpha = 1;

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

    // ── Focused house: OFFICE pins (squares) + a bold SEAT ring ──
    if (sel) {
      const offR = Math.max(1.3, 2.2 * inv);
      for (const office of sel.offices ?? []) {
        const pos = office[1];
        if (!pos) continue;
        ctx.fillStyle = selColor;
        ctx.strokeStyle = "rgba(8,16,28,0.95)";
        ctx.lineWidth = Math.max(0.5, 1.0 * inv);
        ctx.fillRect(pos[0] + 0.5 - offR, pos[1] + 0.5 - offR, offR * 2, offR * 2);
        ctx.strokeRect(pos[0] + 0.5 - offR, pos[1] + 0.5 - offR, offR * 2, offR * 2);
      }
      if (sel.seat) {
        const sr = Math.max(1.8, 3.0 * inv);
        ctx.beginPath();
        ctx.arc(sel.seat[0] + 0.5, sel.seat[1] + 0.5, sr, 0, Math.PI * 2);
        ctx.strokeStyle = selColor;
        ctx.lineWidth = Math.max(0.9, 1.8 * inv);
        ctx.stroke();
        ctx.beginPath();
        ctx.fillStyle = selColor;
        ctx.arc(sel.seat[0] + 0.5, sel.seat[1] + 0.5, Math.max(0.9, 1.5 * inv), 0, Math.PI * 2);
        ctx.fill();
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
  private renderSettlementNames(ctx: CanvasRenderingContext2D) {
    const fs = Math.max(4, 8 / this.currentScale);
    ctx.font = `${fs}px sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "bottom";
    ctx.lineWidth = Math.max(0.5, 1.6 / this.currentScale);
    for (const s of this.settlements) {
      if (!s.name) continue;
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
      const ax = from[0] + 0.5, ay = from[1] + 0.5, bx = to[0] + 0.5, by = to[1] + 0.5;
      ctx.globalAlpha = 0.4 + 0.5 * norm;
      ctx.strokeStyle = "#5fc8a8";
      ctx.lineWidth = Math.max(0.5, (1.0 + norm * 5.0) * inv);
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      ctx.lineTo(bx, by);
      ctx.stroke();
      // Arrowhead at the consumer end (net direction).
      let dx = bx - ax, dy = by - ay;
      const m = Math.hypot(dx, dy);
      if (m > 0.001) {
        dx /= m; dy /= m;
        const hl = Math.max(2, 8 * inv);
        const px = -dy, py = dx;
        const mxp = (ax + bx) / 2, myp = (ay + by) / 2; // arrow at corridor midpoint
        ctx.beginPath();
        ctx.moveTo(mxp + dx * hl * 0.5, myp + dy * hl * 0.5);
        ctx.lineTo(mxp - dx * hl * 0.5 + px * hl * 0.5, myp - dy * hl * 0.5 + py * hl * 0.5);
        ctx.lineTo(mxp - dx * hl * 0.5 - px * hl * 0.5, myp - dy * hl * 0.5 - py * hl * 0.5);
        ctx.closePath();
        ctx.fillStyle = "#7fe0c0";
        ctx.fill();
      }
    }
    ctx.lineCap = "butt";
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
    this.fisheryBanks = [];
    this.sharkZones = [];
    this.shipwormZones = [];
    this.stormZones = [];
    this.monsoonZones = [];
    this.reefZones = [];
    this.goodRegions = [];
    this.tradeTrunks = [];
    this.politicalCenters = [];
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
