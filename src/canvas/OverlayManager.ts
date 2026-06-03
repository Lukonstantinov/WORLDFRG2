import type { RiverData, LakeData, Settlement, VectorSample, Streamline, TradeRoute, FisheryBank, SharkZone, GoodRegion, TradeTrunk, PoliticalCenter } from "../types";
import { GOOD_DEFS, goodOverlayKey } from "../goods";

const GOOD_BY_NAME = new Map(GOOD_DEFS.map((g) => [g.name, g]));
const SHARK_COLOR = "#e04040";
const SHIPWORM_COLOR = "#b98a4a";
const TRADE_TRUNK = "#e0c060"; // bundled commodity-flow trunk (amber)
const POLITICAL_COLOR = "#d65fd0"; // influence disc (magenta)

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
  private goodRegions: GoodRegion[] = [];
  private tradeTrunks: TradeTrunk[] = [];
  private politicalCenters: PoliticalCenter[] = [];
  private latLinesData: { gridW: number; gridH: number; equatorOffset: number; latScale: number } | null = null;

  private visibility: Record<string, boolean> = {
    rivers: true, lakes: true, settlements: true,
    markers: false, wind: false, currents: false, latLines: false,
    tradeRoutes: false, fisheryBanks: false,
    sharkZones: false, shipwormZones: false, tradeFlows: false,
    politicalInfluence: false,
  };

  private currentScale = 1;
  private worldW = 0;

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

  drawGoodRegions(regions: GoodRegion[]) {
    this.goodRegions = regions;
  }

  drawTradeTrunks(trunks: TradeTrunk[], gridW: number) {
    this.tradeTrunks = trunks;
    this.worldW = gridW;
  }

  drawPolitical(centers: PoliticalCenter[]) {
    this.politicalCenters = centers;
  }

  drawLatLines(gridW: number, gridH: number, equatorOffset = 0.5, latScale = 1) {
    this.latLinesData = { gridW, gridH, equatorOffset, latScale };
  }

  setVisible(type: string, visible: boolean) {
    this.visibility[type] = visible;
  }

  updateScale(scale: number) {
    this.currentScale = scale;
  }

  /** Render all overlays to a 2D context (called within viewport transform) */
  render(ctx: CanvasRenderingContext2D) {
    if (this.visibility.lakes && this.lakes.length > 0) {
      ctx.fillStyle = LAKE_COLOR;
      for (const lake of this.lakes) {
        for (const [x, y] of lake.cells) {
          ctx.fillRect(x, y, 1, 1);
        }
      }
    }

    if (this.visibility.rivers && this.rivers.length > 0) {
      ctx.strokeStyle = RIVER_COLOR;
      ctx.globalAlpha = 0.85;
      for (const river of this.rivers) {
        if (river.points.length < 2) continue;
        ctx.lineWidth = Math.max(0.5, river.width * 0.4);
        ctx.beginPath();
        ctx.moveTo(river.points[0][0] + 0.5, river.points[0][1] + 0.5);
        for (let i = 1; i < river.points.length; i++) {
          ctx.lineTo(river.points[i][0] + 0.5, river.points[i][1] + 0.5);
        }
        ctx.stroke();
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
      const { gridW, gridH, equatorOffset, latScale } = this.latLinesData;
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
      const scale = latScale <= 1e-4 ? 1 : latScale;
      for (const { lat, label } of lines) {
        // Inverse of the Rust lat_from_y mapping.
        const y = (equatorOffset - (lat * scale) / 180) * gridH;
        // Crop: skip lines whose latitude falls outside the canvas.
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
        const color = def ? def.color : "#cccccc";
        const emoji = def ? def.emoji : "";
        this.renderRegionMask(ctx, r.cells, r.cell_size, color, emoji, r.x, r.y, 0.16 + 0.18 * Math.min(1, r.score), r.sublabel);
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
        // City names intentionally not drawn — ranked dots only (size = rank).
      }
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
    const lineWidth = Math.max(0.5, 1.6 / Math.sqrt(this.currentScale));
    const dash = Math.max(1.5, 4 / Math.sqrt(this.currentScale));

    ctx.globalAlpha = 0.8;
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

  /** A filled cell-mask AREA (the real distribution shape) with a boundary
   *  outline and a centered emoji glyph (shark / trade-good belt). */
  private renderRegionMask(
    ctx: CanvasRenderingContext2D,
    cells: [number, number][], cellSize: number, color: string, emoji: string,
    lx: number, ly: number, alpha: number, sublabel: string = "",
  ) {
    if (cells.length === 0) return;

    // Fill every coarse cell.
    ctx.globalAlpha = alpha;
    ctx.fillStyle = color;
    for (const [cx, cy] of cells) ctx.fillRect(cx, cy, cellSize, cellSize);

    // Boundary outline: stroke only edges whose neighbour is outside the region.
    const set = new Set(cells.map(([cx, cy]) => `${cx},${cy}`));
    ctx.globalAlpha = Math.min(0.85, alpha + 0.4);
    ctx.strokeStyle = color;
    ctx.lineWidth = Math.max(0.4, 1.0 / Math.sqrt(this.currentScale));
    ctx.beginPath();
    for (const [cx, cy] of cells) {
      if (!set.has(`${cx},${cy - cellSize}`)) { ctx.moveTo(cx, cy); ctx.lineTo(cx + cellSize, cy); }
      if (!set.has(`${cx},${cy + cellSize}`)) { ctx.moveTo(cx, cy + cellSize); ctx.lineTo(cx + cellSize, cy + cellSize); }
      if (!set.has(`${cx - cellSize},${cy}`)) { ctx.moveTo(cx, cy); ctx.lineTo(cx, cy + cellSize); }
      if (!set.has(`${cx + cellSize},${cy}`)) { ctx.moveTo(cx + cellSize, cy); ctx.lineTo(cx + cellSize, cy + cellSize); }
    }
    ctx.stroke();
    ctx.globalAlpha = 1;

    // Emoji at the label centroid (font in world space ⇒ ~constant screen px).
    if (emoji) {
      const fs = Math.max(6, 16 / this.currentScale);
      ctx.font = `${fs}px sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(emoji, lx, ly);
      if (sublabel) {
        const ss = Math.max(5, 9 / this.currentScale);
        ctx.font = `${ss}px sans-serif`;
        ctx.fillStyle = "#f0f0f0";
        ctx.fillText(sublabel, lx, ly + fs * 0.85);
      }
      ctx.textAlign = "start";
      ctx.textBaseline = "alphabetic";
    }
  }

  /** Bundled commodity trunks: each routed coarse edge drawn with width ∝ the
   *  total goods volume travelling along it, so shared corridors read as trunks. */
  private renderTradeTrunks(ctx: CanvasRenderingContext2D) {
    let maxVol = 0;
    for (const t of this.tradeTrunks) maxVol = Math.max(maxVol, t.volume);
    if (maxVol <= 0) return;

    ctx.globalAlpha = 0.7;
    ctx.strokeStyle = TRADE_TRUNK;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    for (const t of this.tradeTrunks) {
      const pts = t.points;
      if (pts.length < 2) continue;
      const a = pts[0], b = pts[1];
      // Skip edges spanning the cylindrical wrap seam.
      if (this.worldW > 0 && Math.abs(a[0] - b[0]) > this.worldW / 2) continue;
      const norm = t.volume / maxVol;
      ctx.lineWidth = Math.max(0.5, (0.6 + norm * 5.0) / Math.sqrt(this.currentScale));
      ctx.beginPath();
      ctx.moveTo(a[0] + 0.5, a[1] + 0.5);
      ctx.lineTo(b[0] + 0.5, b[1] + 0.5);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
  }

  /** A political influence disc: radius ∝ trade power, brightest for the rank-0
   *  powers, with a centre dot. */
  private renderPoliticalCenter(ctx: CanvasRenderingContext2D, c: PoliticalCenter) {
    const alpha = 0.10 + 0.22 * Math.min(1, c.power);
    ctx.beginPath();
    ctx.arc(c.x + 0.5, c.y + 0.5, c.radius, 0, Math.PI * 2);
    ctx.fillStyle = POLITICAL_COLOR;
    ctx.globalAlpha = alpha;
    ctx.fill();
    ctx.globalAlpha = Math.min(0.85, alpha + 0.4);
    ctx.strokeStyle = POLITICAL_COLOR;
    ctx.lineWidth = Math.max(0.4, 1.0 / Math.sqrt(this.currentScale));
    ctx.stroke();
    // Centre marker.
    ctx.beginPath();
    ctx.arc(c.x + 0.5, c.y + 0.5, Math.max(0.8, 2.0 / Math.sqrt(this.currentScale)), 0, Math.PI * 2);
    ctx.fillStyle = POLITICAL_COLOR;
    ctx.globalAlpha = 0.95;
    ctx.fill();
    ctx.globalAlpha = 1;
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
    this.goodRegions = [];
    this.tradeTrunks = [];
    this.politicalCenters = [];
    this.latLinesData = null;
  }
}
