import type { RiverData, LakeData, Settlement, VectorSample, Streamline, TradeRoute, FisheryBank, SharkZone, GoodRegion, TradeTrunk, PoliticalCenter, EconChokepoint, EconChain, EconRegion } from "../types";
import { GOOD_DEFS, goodOverlayKey, goodSubtypes, type SubtypeDef } from "../goods";
import { drawGoodIcon } from "./goodIcons";
import { latLineY } from "./projection";

/** Per-cell abundance → one of 4 discrete quality tiers (1 negligible … 4 very
 *  high). The cell's fill opacity steps with the tier so richer deposits read as
 *  more solid and poor deposits as faint. */
const TIER_ALPHA = [0, 0.32, 0.55, 0.78, 1.0]; // index 1..4

const GOOD_BY_NAME = new Map(GOOD_DEFS.map((g) => [g.name, g]));
const SHARK_COLOR = "#e04040";
const SHIPWORM_COLOR = "#b98a4a";
const STORM_COLOR = "#c050d0";
const REEF_COLOR = "#30c0b0";
const TRADE_TRUNK = "#e0c060"; // major bundled commodity-flow trunk (amber)
const TRADE_TRUNK_MINOR = "#b8a878"; // minor/low-volume trunk (muted amber)
const POLITICAL_COLOR = "#d65fd0"; // trade-hub marker (magenta) — legacy
const STAR_COLOR = "#ffd24a"; // power-tier stars on major hubs (gold) — legacy
const HUB_BLUE = "#3a86d6"; // trade-hub circle

const RIVER_COLOR = "#2288cc";
const LAKE_COLOR = "rgba(51, 153, 221, 0.7)";

const SETTLEMENT_COLORS: Record<string, string> = {
  capital: "#ffd700",
  city: "#ff8844",
  town: "#cccccc",
  village: "#88aa88",
};

const SETTLEMENT_SIZES: Record<string, number> = {
  capital: 3,
  city: 2.2,
  town: 1.6,
  village: 1,
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
  private reefZones: SharkZone[] = [];
  private goodRegions: GoodRegion[] = [];
  /** Per-good display metadata (icon/color) from the active editable spec; falls
   *  back to the static GOOD_DEFS when absent. */
  private goodMeta: Map<string, { icon: string; color: string }> | null = null;
  private tradeTrunks: TradeTrunk[] = [];
  private politicalCenters: PoliticalCenter[] = [];
  private chokepoints: EconChokepoint[] = [];
  private econRegions: EconRegion[] = [];
  private supplyChain: EconChain | null = null;
  /** Per-good reach: chains carrying the selected good + the hubs it reaches. */
  private reachChains: EconChain[] = [];
  private reachHubs: [number, number][] = [];
  private latLinesData: { gridW: number; gridH: number; equatorOffset: number; latScale: number; lineRatio: number } | null = null;

  private visibility: Record<string, boolean> = {
    rivers: true, lakes: true, settlements: true,
    markers: false, wind: false, currents: false, latLines: false,
    tradeRoutes: false, fisheryBanks: false,
    sharkZones: false, shipwormZones: false, stormZones: false, reefZones: false, tradeFlows: false,
    politicalInfluence: false, chokepoints: false,
    hubNames: false, settlementNames: false, tradeRegions: false,
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

  drawReefZones(zones: SharkZone[]) {
    this.reefZones = zones;
  }

  drawGoodRegions(regions: GoodRegion[]) {
    this.goodRegions = regions;
  }

  setGoodMeta(meta: Map<string, { icon: string; color: string }>) {
    this.goodMeta = meta;
  }

  drawTradeTrunks(trunks: TradeTrunk[], gridW: number) {
    this.tradeTrunks = trunks;
    this.worldW = gridW;
  }

  drawPolitical(centers: PoliticalCenter[]) {
    this.politicalCenters = centers;
  }

  drawChokepoints(chokepoints: EconChokepoint[]) {
    this.chokepoints = chokepoints;
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
  setSupplyChain(chain: EconChain | null) {
    this.supplyChain = chain;
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
      for (const river of this.rivers) {
        if (river.points.length < 2) continue;
        // Navigable trade arteries read a touch brighter/wider than minor streams.
        ctx.strokeStyle = river.navigable ? "#3aa6e6" : RIVER_COLOR;
        ctx.lineWidth = Math.max(0.5, river.width * (river.navigable ? 0.55 : 0.4));
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
          ctx.lineWidth = Math.max(0.4, river.width * 0.25);
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

    // Political influence: translucent discs sized by trade power.
    if (this.visibility.politicalInfluence && this.politicalCenters.length > 0) {
      for (const c of this.politicalCenters) this.renderPoliticalCenter(ctx, c);
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
        const radius = SETTLEMENT_SIZES[s.size] || 1;
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

  /** A trade hub: a blue circle. Large hubs (top power tier, ≥4 of 5) get a white
   *  square inside to mark them as the great entrepôts. No stars, no disc. The
   *  hub's name is drawn separately when the "Hub names" overlay is on. */
  private renderPoliticalCenter(ctx: CanvasRenderingContext2D, c: PoliticalCenter) {
    const x = c.x + 0.5;
    const y = c.y + 0.5;
    const stars = Math.max(0, Math.min(5, Math.round(c.stars)));
    const large = stars >= 4;
    // Larger marker for the great hubs so they read at a glance.
    const r = Math.max(1.0, (large ? 3.4 : 2.2) / Math.sqrt(this.currentScale));

    ctx.beginPath();
    ctx.arc(x, y, r, 0, Math.PI * 2);
    ctx.fillStyle = HUB_BLUE;
    ctx.globalAlpha = 0.95;
    ctx.fill();
    ctx.lineWidth = Math.max(0.3, 0.9 / Math.sqrt(this.currentScale));
    ctx.strokeStyle = "rgba(8,20,40,0.85)";
    ctx.stroke();

    // Great hubs: a white square inscribed in the circle (Venice/Malacca tier).
    if (large) {
      const s = r * 0.92; // square side spanning most of the disc
      ctx.fillStyle = "#ffffff";
      ctx.globalAlpha = 1;
      ctx.fillRect(x - s / 2, y - s / 2, s, s);
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

    // Road line (skip the wrap-seam segment).
    ctx.lineWidth = Math.max(1.2, 3.5 * inv);
    ctx.strokeStyle = "rgba(255,220,120,0.9)";
    ctx.lineCap = "round";
    for (let i = 0; i < pts.length - 1; i++) {
      const a = pts[i]; const b = pts[i + 1];
      if (this.worldW && Math.abs(a[0] - b[0]) > half) continue;
      ctx.beginPath();
      ctx.moveTo(a[0] + 0.5, a[1] + 0.5);
      ctx.lineTo(b[0] + 0.5, b[1] + 0.5);
      ctx.stroke();
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
    ctx.strokeStyle = "rgba(255,210,90,0.85)";
    ctx.lineWidth = Math.max(0.6, 1.8 * inv);
    ctx.lineCap = "round";
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
    ctx.lineCap = "butt";
    // Ring every hub the good reaches.
    const r = Math.max(1.2, 3.2 * inv);
    ctx.lineWidth = Math.max(0.5, 1.4 * inv);
    ctx.strokeStyle = "rgba(255,230,150,0.95)";
    ctx.fillStyle = "rgba(255,210,90,0.25)";
    for (const [hx, hy] of this.reachHubs) {
      ctx.beginPath();
      ctx.arc(hx + 0.5, hy + 0.5, r, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    }
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
    this.reefZones = [];
    this.goodRegions = [];
    this.tradeTrunks = [];
    this.politicalCenters = [];
    this.chokepoints = [];
    this.econRegions = [];
    this.reachChains = [];
    this.reachHubs = [];
    this.supplyChain = null;
    this.latLinesData = null;
  }
}
