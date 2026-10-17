import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { initPixiApp, type MapApp } from "@canvas/PixiApp";
import { TileViewport } from "@canvas/TileViewport";
import { TileManager } from "@canvas/TileManager";
import { OverlayManager, type ColonyMarker } from "@canvas/OverlayManager";
import { setExportSnapshotFn, setExportSnapshotRawFn } from "@canvas/mapExport";
import { createPaintOverlay, drawCursorRing, paintStamp, clearPaintOverlay } from "@canvas/PaintOverlay";
import { useWorldStore, decodeProvinceRaster } from "@state/worldStore";
import { useViewportStore } from "@state/viewportStore";
import { useUIStore } from "@state/uiStore";
import { useGoodsStore } from "@state/goodsStore";
import { useCampaignStore } from "@state/campaignStore";
import { useSettingsStore, MAP_STYLES } from "@state/settingsStore";
import { usePaletteStore } from "@state/paletteStore";
import { FORGE_ONLY } from "./mapThemes";
import { plateName } from "./plateNames";
import { RealmLayer } from "@canvas/RealmLayer";
import { usePoliticalStore, loadPolitical } from "@state/politicalStore";
import { simPaintProvinceCells, paintStroke, undoAction, redoAction, computeOverlays, computeGoodBeltMasks, campaignGoodAtlas, computeStormZones, computeMonsoonZones, computeClimateBands, computeCultureRegions, computeTradeRoutes, computeTradeMatrix, computePolitical, getEconomy, getRiverSystems, getLakeSystems, campaignMerchantRoutes, campaignFuturesLanes, campaignGetSpeculation, campaignGetTradeFlow, campaignGetCorridors, campaignGetExpeditions, campaignCoinUsage, campaignGetBanks, campaignGetEpidemics, campaignGetGuilds, campaignGetFigures, campaignGetLandmarks, campaignGetDynasties, campaignGetTradeBasins, campaignGetGoodHeat, campaignGetCultures, campaignCultureHubs, campaignGetMigrationRoutes, computeStates, getCellInfo, getPlateMotion, computeCoarseRoute, placeSettlementAt, campaignHouseAtlas, getDepositSites, campaignExploitedGoods } from "@bridge";
import type { MerchantRoute, FuturesLane, Toponym, CoarseRoute } from "@types";
import { goodOverlayKey, GOOD_DEFS } from "@goods";
import type { PaintValue, EconChain, Settlement, CampaignHubBrief } from "@types";

/** Route resolution for CAMPAIGN trade layers (Dynamic Trade Flow, Corridors,
 *  and every `laneBetween`-fed overlay — merchant routes, futures lanes, the
 *  Goods Atlas/Codex flow lines) used to reuse `bioParams.tradeReach`/
 *  `maxCrossing` — the WORLDGEN Biological-step crossing preference (default
 *  reach=1 "coastal+short", maxCrossing=0.12 of the map width), meant to
 *  bound how many sea lanes worldgen's OWN trade-route graph synthesizes.
 *  A campaign's own trade horizon is much wider (`TRADE_STAGING_AND_POSTS_
 *  PLAN.md` §1.1 measures it at 0.24×world_w), so a real, already-realized
 *  campaign flow longer than the worldgen bound could never resolve a route
 *  here and always fell back to the dashed direct line — "trade routes read
 *  as straight lines instead of following the coastline/rivers" for exactly
 *  the longest, most interesting flows. This is a DISPLAY-ONLY lookup (it
 *  finds the best physical path for a trade the sim already decided
 *  happened; it creates no new trade and changes no economics), so it asks
 *  for the widest reach available: reach 0, with a maxCrossing that is moot
 *  there. Worldgen's own trade-route graph (Toolbar → Trade Goods → routes)
 *  still honours `bioParams` — this only changes how a campaign's own
 *  realized flows are drawn.
 *
 *  NOTE — reach 0 is no longer "no cap". `MAX_OPEN_SEA_CROSSING_KM` is a WORLD
 *  RULE and binds at every reach except 2, so a flow needing more than that of
 *  unbroken open water resolves no path and falls back to the dashed direct
 *  line (rule 35's honest rendering of an open-water lane). Such a flow should
 *  now be rare rather than routine: trade COMPONENTS are built across the same
 *  crossing limit, so two shores that far apart no longer share a market to
 *  generate the flow in the first place. */
const CAMPAIGN_ROUTE_REACH = 0;
/** "Asked, and there is genuinely no legal route." Cached as-is so the pair is
 *  not re-fetched every poll; that lane keeps the dashed fallback (rule 35). */
const EMPTY_ROUTE: CoarseRoute = { points: [], sea: [] };
const CAMPAIGN_ROUTE_MAX_CROSSING = 1.0;

/** Must mirror the backend's tile grid (`TILE_SIZE` in `tile/coords.rs` /
 *  `TileManager.ts` / `TileViewport.ts`). */
const TILE_SIZE = 128;

/** WORLD-WIDTH COPIES (infinite horizontal scroll). The world is a cylinder with
 *  period EXACTLY `grid_width` cells, but tiles are 128-cell blocks and no shipped
 *  width is a multiple of 128 (3600 = 28.1 tiles), so the last tile column is
 *  partly sea padding. Wrapping the TILE grid (3712 cells) put that padding strip
 *  in the middle of the scroll as a pale band with cut coastlines. Instead each
 *  visible copy k is drawn from the CANONICAL tile columns 0..baseTilesX-1,
 *  translated by `k * grid_width` and clipped to one world width, so copy k's
 *  last real cell touches copy k+1's first one with nothing between. Returns, per
 *  visible copy, the canonical tile-column range it needs. */
function visibleCopies(viewX: number, scaleX: number, screenW: number, gridW: number) {
  if (gridW <= 0 || !(scaleX > 0)) return [];
  const baseTilesX = Math.ceil(gridW / TILE_SIZE);
  const leftWorld = (0 - viewX) / scaleX;
  const rightWorld = (screenW - viewX) / scaleX;
  const kMin = Math.floor(leftWorld / gridW);
  const kMax = Math.floor(rightWorld / gridW);
  const out: { k: number; txMin: number; txMax: number }[] = [];
  for (let k = kMin; k <= kMax; k++) {
    const lo = Math.max(0, leftWorld - k * gridW);
    const hi = Math.min(gridW - 1, rightWorld - k * gridW);
    out.push({
      k,
      txMin: Math.max(0, Math.floor(lo / TILE_SIZE) - 1),
      txMax: Math.min(baseTilesX - 1, Math.floor(hi / TILE_SIZE) + 1),
    });
  }
  return out;
}

/** Distinct canonical tile-column intervals the visible copies need (a view that
 *  straddles the seam wants the tail AND the head, not the whole row). */
function tileColumnRanges(copies: { txMin: number; txMax: number }[]): [number, number][] {
  const seen = new Set<string>();
  const out: [number, number][] = [];
  for (const c of copies) {
    const key = `${c.txMin}:${c.txMax}`;
    if (!seen.has(key)) { seen.add(key); out.push([c.txMin, c.txMax]); }
  }
  return out;
}

/** Largest box with the world's aspect ratio that fits inside the pane. */
function fitBox(paneW: number, paneH: number, gridW: number, gridH: number) {
  if (paneW <= 0 || paneH <= 0 || gridW <= 0 || gridH <= 0) {
    return { w: paneW, h: paneH };
  }
  const aspect = gridW / gridH;
  if (paneW / paneH > aspect) {
    // Pane is wider than the world → height-limited.
    const h = paneH;
    return { w: Math.round(h * aspect), h };
  }
  const w = paneW;
  return { w, h: Math.round(w / aspect) };
}

export function MapCanvas() {
  const paneRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [box, setBox] = useState<{ w: number; h: number } | null>(null);
  const appRef = useRef<MapApp | null>(null);
  const viewportRef = useRef<TileViewport | null>(null);
  const tileManagerRef = useRef<TileManager | null>(null);
  const overlayManagerRef = useRef<OverlayManager | null>(null);
  const realmLayerRef = useRef(new RealmLayer());
  const politicalDownRef = useRef<{ x: number; y: number; wx: number; wy: number } | null>(null);
  const isPaintingRef = useRef(false);
  const hoveredHubRef = useRef<number>(-1);
  const isErasingRef = useRef(false);
  const pendingCellsRef = useRef<Set<number>>(new Set());
  const initRef = useRef(false);
  const rafRef = useRef(0);
  const needsRenderRef = useRef(true);
  const [initError, setInitError] = useState<string | null>(null);
  const [initStatus, setInitStatus] = useState("waiting");

  const meta = useWorldStore((s) => s.meta);
  const latConfig = useWorldStore((s) => s.latConfig);
  const rivers = useWorldStore((s) => s.rivers);
  const lakes = useWorldStore((s) => s.lakes);
  const settlements = useWorldStore((s) => s.settlements);
  const provinces = useWorldStore((s) => s.provinces);
  const provinceRaster = useWorldStore((s) => s.provinceRaster);
  const selectedProvince = useUIStore((s) => s.selectedProvince);
  const markedProvinces = useUIStore((s) => s.markedProvinces);
  const provinceOpacity = useUIStore((s) => s.provinceOpacity);
  const provinceFillMode = useUIStore((s) => s.provinceFillMode);
  const provinceSingleColor = useUIStore((s) => s.provinceSingleColor);
  const provinceBorderColor = useUIStore((s) => s.provinceBorderColor);
  const economy = useWorldStore((s) => s.economy);
  const setEconomy = useWorldStore((s) => s.setEconomy);
  const activeLayer = useUIStore((s) => s.activeLayer);
  const selectedPlate = useUIStore((s) => s.selectedPlate);
  const appMode = useUIStore((s) => s.appMode);
  const activeTool = useUIStore((s) => s.activeTool);
  const brushRadius = useUIStore((s) => s.brushRadius);
  const elevationValue = useUIStore((s) => s.elevationValue);
  const tileVersion = useViewportStore((s) => s.tileVersion);
  const focusTarget = useViewportStore((s) => s.focusTarget);
  const searchPin = useViewportStore((s) => s.searchPin);
  const warHighlight = useViewportStore((s) => s.warHighlight);
  const overlayVisibility = useUIStore((s) => s.overlayVisibility);
  const goodQualityHeatmap = useUIStore((s) => s.goodQualityHeatmap);
  const merchantRouteRisk = useUIStore((s) => s.merchantRouteRisk);
  const lineColors = useSettingsStore((s) => s.lineColors);
  // D10 · the renderer's OWN belt-quality scale, served rather than copied (§8.18).
  const palettes = usePaletteStore((s) => s.palettes);
  const loadPalettes = usePaletteStore((s) => s.load);
  const houses = useCampaignStore((s) => s.houses);
  const selectedHouseIdx = useCampaignStore((s) => s.selectedHouseIdx);
  const campaignSnapshot = useCampaignStore((s) => s.snapshot);
  const stretchToFit = useUIStore((s) => s.stretchToFit);
  const setStatus = useUIStore((s) => s.setStatus);
  // Trade routes/flows are a product of the Biological-Trade step (8), not an
  // automatic response to settlement changes.
  const step8Done = useUIStore((s) => s.stepCompleted[8]);
  const stormMonth = useUIStore((s) => s.bioParams.stormMonth);
  const calendarMonths = useUIStore((s) => s.bioParams.calendarMonths);
  const goodsSpecs = useGoodsStore((s) => s.specs);
  const loadGoodsFromWorld = useGoodsStore((s) => s.loadFromWorld);
  const step9Done = useUIStore((s) => s.stepCompleted[9]);
  const bioParams = useUIStore((s) => s.bioParams);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const setSelectedMerchantRoute = useUIStore((s) => s.setSelectedMerchantRoute);
  const merchantRoutesRef = useRef<MerchantRoute[]>([]);
  const setSelectedFuturesLane = useUIStore((s) => s.setSelectedFuturesLane);
  const selectedFuturesLane = useUIStore((s) => s.selectedFuturesLane);
  const futuresFocus = useUIStore((s) => s.futuresFocus);
  const flowHighlight = useUIStore((s) => s.flowHighlight);
  const personRoad = useUIStore((s) => s.personRoad);
  const futuresLanesRef = useRef<FuturesLane[]>([]);
  const selectedChain = useUIStore((s) => s.selectedChain);
  const selectedHub = useUIStore((s) => s.selectedHub);
  const migrationMode = useUIStore((s) => s.migrationMode);
  const openRoads = useUIStore((s) => s.openRoads);
  const selectedExport = useUIStore((s) => s.selectedExport);
  const setSelectedGood = useUIStore((s) => s.setSelectedGood);
  const hubDisplay = useUIStore((s) => s.hubDisplay);
  const reachGood = useUIStore((s) => s.reachGood);
  // Ridge-drawing tool: current pen settings, drawn lines, and the committer.
  const ridgeParams = useUIStore((s) => s.ridgeParams);
  const ridgeLines = useUIStore((s) => s.ridgeLines);
  const addRidgeLine = useUIStore((s) => s.addRidgeLine);
  // Landmass lasso: the committer for a finished freehand selection polygon.
  const setLassoPolygon = useUIStore((s) => s.setLassoPolygon);

  // Stable identity for the loaded world. Heavy effects (tile reloads + sim IPC)
  // key off this string rather than the `meta` object, so latitude-slider edits
  // — which no longer touch `meta` at all, but even a persisted change keeps the
  // same dimensions/name — don't retrigger tile-cache rebuilds or sim queries.
  const worldKey = meta ? `${meta.name}|${meta.grid_width}|${meta.grid_height}` : null;

  const metaRef = useRef(meta);
  metaRef.current = meta;
  const stretchToFitRef = useRef(stretchToFit);
  stretchToFitRef.current = stretchToFit;
  // The isolated class rides in the LAYER KEY, so the tile cache treats an
  // isolated view as its own layer and invalidates correctly for free.
  const isolateClass = useUIStore((s) => s.isolateClass);
  // ELEVATION STYLES: only meaningful on the two layers that carry a style
  // (`render_tile_full`'s dispatch ignores `#style=` on any other key), so it
  // rides the layer key here exactly like `isolateClass` above.
  const elevationStyle = useUIStore((s) => s.elevationStyle);
  // MAP STYLE: plates on the flat `land` base or the plain `terrain` hillshade are
  // drawn on the chosen style's own terrain (Atlas / Parchment / Game Board).
  const mapStyle = useSettingsStore((s) => s.mapStyle);
  const mapTerrain = MAP_STYLES.find((m) => m.id === mapStyle)?.terrain ?? null;
  const compareLayer = useUIStore((s) => s.compareLayer);
  const comparePos = useUIStore((s) => s.comparePos);
  const setComparePos = useUIStore((s) => s.setComparePos);
  const compareRef = useRef<{ layer: string | null; pos: number }>({ layer: null, pos: 0.5 });
  compareRef.current = { layer: compareLayer, pos: comparePos };
  const activeLayerRef = useRef(activeLayer);
  activeLayerRef.current = (() => {
    if (activeLayer === "political") return "land" as typeof activeLayer;
    if (mapTerrain && (activeLayer === "land" || activeLayer === "terrain")) {
      return `terrain#style=${mapTerrain}` as typeof activeLayer;
    }
    let key: string = activeLayer;
    if (isolateClass !== null) key += `#iso=${isolateClass}`;
    if (elevationStyle && (activeLayer === "elevation" || activeLayer === "terrain")) {
      key += `#style=${elevationStyle}`;
    }
    return key as typeof activeLayer;
  })();
  const activeToolRef = useRef(activeTool);
  activeToolRef.current = activeTool;
  // GENERATION_UX_REDESIGN_PLAN.md Slice 10 (F11) — the base-layer opacity
  // slider was wired to nothing (§F11). `provinceOpacity` one field over IS
  // wired via `om.setProvinceOpacity` below, which is what proved this was an
  // omission rather than a design; the base layer gets the same treatment.
  const layerOpacity = useUIStore((s) => s.layerOpacity);
  const layerOpacityRef = useRef(layerOpacity);
  layerOpacityRef.current = layerOpacity;
  const overlayOpacity = useUIStore((s) => s.overlayOpacity);
  const brushRadiusRef = useRef(brushRadius);
  brushRadiusRef.current = brushRadius;
  const economyRef = useRef(economy);
  economyRef.current = economy;
  // Live campaign hubs (includes in-campaign colonies/outposts that the frozen economy
  // snapshot lacks) — used so a click on a colony can select it and open its detail.
  const campaignSnapshotRef = useRef(campaignSnapshot);
  campaignSnapshotRef.current = campaignSnapshot;
  const elevationValueRef = useRef(elevationValue);
  elevationValueRef.current = elevationValue;
  // Ridge tool: live pen params + drawn lines (for callbacks), and the in-progress
  // polyline being dragged (world cells), committed on pointer-up.
  const ridgeParamsRef = useRef(ridgeParams);
  ridgeParamsRef.current = ridgeParams;
  const ridgeLinesRef = useRef(ridgeLines);
  ridgeLinesRef.current = ridgeLines;
  const ridgeDraftRef = useRef<[number, number][]>([]);
  // Landmass lasso: the in-progress polygon being dragged (world cells),
  // committed to the store on pointer-up. Mirrors ridgeDraftRef exactly.
  const lassoDraftRef = useRef<[number, number][]>([]);
  // Good regions kept for click hit-testing (click a good belt → good-flow panel).
  const goodRegionsRef = useRef<import("@types").GoodRegion[]>([]);
  const overlayVisibilityRef = useRef(overlayVisibility);
  overlayVisibilityRef.current = overlayVisibility;
  // Unified clickable-settlement candidates — EXACTLY the dots the map draws: every
  // live campaign hub PLUS every worldgen/hinterland settlement below the sim cap
  // (deduped by position, live hub wins). Both hover and click hit-test against this
  // ONE list so the ring shines on — and a click selects — the dot under the cursor,
  // never a farther live hub (fixes the "another becomes shiny / can't click" bug).
  const clickCandidatesRef = useRef<{ x: number; y: number; id: number; colony: number }[]>([]);

  /** Mark canvas as needing a repaint */
  const requestRender = useCallback(() => {
    needsRenderRef.current = true;
  }, []);

  /** Render the map to the 2D canvas */
  // GENERATION_UX_REDESIGN_PLAN.md Slice 8 — the scene-drawing body, factored
  // out of `renderFrame` so the export snapshot can call the EXACT same code
  // against an offscreen canvas at a boosted pixel ratio, instead of a second
  // implementation that could drift from what the screen actually shows
  // (§8.18's "never reintroduce a second copy" discipline, applied to a
  // render path rather than a colour table). `w`/`h` are CSS-pixel view
  // dimensions; `dpr` is the EFFECTIVE device-pixel ratio to render at —
  // `window.devicePixelRatio` for the live screen, or `devicePixelRatio ×
  // multiplier` for a higher-resolution export — so the exported image is a
  // real higher-density render of the SAME view, not an upscaled screenshot.
  const drawScene = useCallback((
    ctx: CanvasRenderingContext2D, w: number, h: number, dpr: number,
  ) => {
    const viewport = viewportRef.current;
    const tileManager = tileManagerRef.current;
    const overlayManager = overlayManagerRef.current;
    if (!viewport || !tileManager) return;

    // Clear
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = "#15110b";
    ctx.fillRect(0, 0, w, h);

    // Debug: draw a visible border so we know the canvas renders
    ctx.strokeStyle = "#5a4d36";
    ctx.lineWidth = 2;
    ctx.strokeRect(1, 1, w - 2, h - 2);

    // Apply viewport transform (independent X/Y scale so the world fills the
    // canvas with no letterbox bars).
    ctx.setTransform(dpr * viewport.scaleX, 0, 0, dpr * viewport.scaleY, dpr * viewport.x, dpr * viewport.y);
    ctx.imageSmoothingEnabled = false;

    // The world is cylindrical (X wraps, rule 6) and the viewport pans freely past
    // either edge, so the map scrolls infinitely: see `visibleCopies`.
    const m = metaRef.current;
    const gw = m ? m.grid_width : 0;
    const gh = m ? m.grid_height : 0;
    viewport.wrapWidth = gw;
    // Every whole world-width COPY touching the screen, each with the canonical
    // tile columns it needs (`visibleCopies`). Tiles of copy k are the SAME
    // canonical columns translated by `k * gw` and clipped to one world width, so
    // the seam between copies is the world's real wrap with no padding strip.
    const copies = m ? visibleCopies(viewport.x, viewport.scaleX, w, gw) : [];

    // Draw tiles (only those within the visible tile range).
    // GENERATION_UX_REDESIGN_PLAN.md Slice 10 (F11) — the base layer's own
    // opacity, previously wired to nothing (the slider drew a live % but no
    // renderer ever read `uiStore.layerOpacity`).
    const visible = viewport.getVisibleTileRange(w, h);
    const baseAlpha = layerOpacityRef.current;
    const cmp = compareRef.current;
    // The political layer paints the whole world itself (sea included), so the tiles
    // under it are not drawn while it has provinces to show.
    const politicalOn = useUIStore.getState().activeLayer === "political" && realmLayerRef.current.ready;
    if (!m) {
      tileManager.draw(ctx, visible);
    } else if (!politicalOn) {
      for (const c of copies) {
        const range = { txMin: c.txMin, txMax: c.txMax, tyMin: visible.tyMin, tyMax: visible.tyMax };
        ctx.save();
        ctx.beginPath();
        ctx.rect(c.k * gw, 0, gw, gh);
        ctx.clip();
        ctx.translate(c.k * gw, 0);
        if (baseAlpha < 1) ctx.globalAlpha = Math.max(0, baseAlpha);
        tileManager.draw(ctx, range);
        ctx.globalAlpha = 1;
        // SWIPE COMPARE: the second layer over the same ground, clipped to the right
        // of the divider (computed in this copy's local world space).
        if (cmp.layer) {
          const dividerWorldX = (w * cmp.pos - viewport.x) / viewport.scaleX - c.k * gw;
          const x0 = Math.max(0, Math.min(gw, dividerWorldX));
          ctx.save();
          ctx.beginPath();
          ctx.rect(x0, 0, gw - x0, gh);
          ctx.clip();
          tileManager.draw(ctx, range, cmp.layer);
          ctx.restore();
        }
        ctx.restore();
      }
    }

    // Political fill + borders for EVERY copy first; labels follow in the overlay pass below.
    if (politicalOn && m) {
      for (const c of copies) {
        ctx.save();
        ctx.beginPath();
        ctx.rect(c.k * gw, 0, gw, gh);
        ctx.clip();
        ctx.translate(c.k * gw, 0);
        realmLayerRef.current.renderFill(ctx, viewport.scale);
        ctx.restore();
      }
    }

    // Overlays are vector data positioned once in canonical [0, gw) space, so
    // they are drawn again per visible copy (capped: redrawing rivers/labels/
    // routes N times per frame is real cost, unlike a tile blit), shifted by
    // `k * gw`. The clip is a little wider than the world so a label or glyph
    // hanging over the seam is not sliced off.
    if (overlayManager && m) {
      const MAX_COPIES = 6;
      let kMinO = copies.length ? copies[0].k : 0;
      let kMaxO = copies.length ? copies[copies.length - 1].k : 0;
      if (kMaxO - kMinO + 1 > MAX_COPIES) {
        const mid = Math.round((kMinO + kMaxO) / 2);
        kMinO = mid - Math.floor(MAX_COPIES / 2);
        kMaxO = kMinO + MAX_COPIES - 1;
      }
      const margin = 48;
      for (let k = kMinO; k <= kMaxO; k++) {
        ctx.save();
        ctx.beginPath();
        ctx.rect(k * gw - margin, 0, gw + 2 * margin, gh);
        ctx.clip();
        if (k !== 0) ctx.translate(k * gw, 0);
        if (politicalOn) {
          realmLayerRef.current.renderLabelPass(ctx, viewport.scale, {
            x0: (0 - viewport.x) / viewport.scaleX - k * gw, x1: (w - viewport.x) / viewport.scaleX - k * gw,
            y0: (0 - viewport.y) / viewport.scaleY, y1: (h - viewport.y) / viewport.scaleY,
          });
        }
        overlayManager.render(ctx);
        ctx.restore();
      }
    }

    // Reset transform
    ctx.setTransform(1, 0, 0, 1, 0, 0);
  }, []);

  const renderFrame = useCallback(() => {
    const mapApp = appRef.current;
    if (!mapApp) return;
    const dpr = window.devicePixelRatio || 1;
    const w = mapApp.canvas.width / dpr;
    const h = mapApp.canvas.height / dpr;
    drawScene(mapApp.ctx, w, h, dpr);
  }, [drawScene]);

  // GENERATION_UX_REDESIGN_PLAN.md Slice 8 — export at `multiplier`×
  // additional pixel density over the current view, by running the identical
  // `drawScene` onto a fresh offscreen canvas instead of the live one.
  // Registered on the `mapExport` module singleton so the Export dialog
  // (which lives outside this component) can call it without MapCanvas
  // having to lift its internal TileManager/OverlayManager/viewport refs out
  // as props.
  const exportSnapshot = useCallback((multiplier: number): string | null => {
    const mapApp = appRef.current;
    if (!mapApp) return null;
    const dpr = window.devicePixelRatio || 1;
    const cssW = mapApp.canvas.width / dpr;
    const cssH = mapApp.canvas.height / dpr;
    const effDpr = dpr * Math.max(1, multiplier);
    const off = document.createElement("canvas");
    off.width = Math.max(1, Math.round(cssW * effDpr));
    off.height = Math.max(1, Math.round(cssH * effDpr));
    const octx = off.getContext("2d");
    if (!octx) return null;
    drawScene(octx, cssW, cssH, effDpr);
    return off.toDataURL("image/png");
  }, [drawScene]);

  const exportSnapshotRaw = useCallback((multiplier: number): ImageData | null => {
    const mapApp = appRef.current;
    if (!mapApp) return null;
    const dpr = window.devicePixelRatio || 1;
    const cssW = mapApp.canvas.width / dpr;
    const cssH = mapApp.canvas.height / dpr;
    const effDpr = dpr * Math.max(1, multiplier);
    const off = document.createElement("canvas");
    off.width = Math.max(1, Math.round(cssW * effDpr));
    off.height = Math.max(1, Math.round(cssH * effDpr));
    const octx = off.getContext("2d");
    if (!octx) return null;
    drawScene(octx, cssW, cssH, effDpr);
    return octx.getImageData(0, 0, off.width, off.height);
  }, [drawScene]);

  useEffect(() => {
    setExportSnapshotFn(exportSnapshot);
    setExportSnapshotRawFn(exportSnapshotRaw);
    return () => { setExportSnapshotFn(null); setExportSnapshotRawFn(null); };
  }, [exportSnapshot, exportSnapshotRaw]);

  const refreshTiles = useCallback(() => {
    const viewport = viewportRef.current;
    const tileManager = tileManagerRef.current;
    const el = containerRef.current;
    const m = metaRef.current;
    if (!viewport || !tileManager || !el || !m) return;

    const { tyMin, tyMax } = viewport.getVisibleTileRange(el.clientWidth, el.clientHeight);
    const cols = tileColumnRanges(visibleCopies(viewport.x, viewport.scaleX, el.clientWidth, m.grid_width));

    // viewport.scale picks the LOD: zoomed out → coarser supertiles, so the
    // request size stays bounded no matter how much of the world is visible.
    const cmpLayer = compareRef.current.layer;
    Promise.all([
      tileManager.loadVisibleTiles(
        cols, tyMin, tyMax,
        activeLayerRef.current, m.grid_width, m.grid_height,
        viewport.scale,
      ),
      cmpLayer
        ? tileManager.loadVisibleTiles(
        cols, tyMin, tyMax,
            cmpLayer, m.grid_width, m.grid_height,
            viewport.scale,
          )
        : Promise.resolve(),
    ]).then(() => requestRender());
  }, [requestRender]);

  // Initialize Canvas 2D
  useEffect(() => {
    const el = containerRef.current;
    if (!el || initRef.current) return;
    initRef.current = true;

    let destroyed = false;

    try {
      const mapApp = initPixiApp(el);
      appRef.current = mapApp;

      const viewport = new TileViewport();
      viewportRef.current = viewport;

      const tileManager = new TileManager();
      // Repaint as each chunk of tiles arrives so large fetches fill in
      // progressively instead of popping in all at once at the end.
      tileManager.onTilesLoaded = () => requestRender();
      tileManagerRef.current = tileManager;

      const overlayManager = new OverlayManager();
      overlayManagerRef.current = overlayManager;

      createPaintOverlay(el);

      mapApp.canvas.addEventListener("wheel", (e) => {
        e.preventDefault();
        // Only moves the viewport's wheel-zoom TARGET — the render loop's own
        // `viewport.update` call eases the displayed scale/position toward it
        // every frame from here, fetching tiles and updating overlay scale as
        // it goes (see the loop's own comment), so nothing needs doing here
        // beyond waking the loop up.
        viewport.onWheel(e);
        requestRender();
      }, { passive: false });

      // Resize handler for the (aspect-locked) canvas container: resize the
      // backing buffer and re-fit the world so it always fills the box exactly.
      let resizeTimer: ReturnType<typeof setTimeout> | undefined;
      const resizeObserver = new ResizeObserver((entries) => {
        const entry = entries[0];
        if (!entry) return;
        const { width, height } = entry.contentRect;
        if (width <= 0 || height <= 0) return;
        // Resize the backing buffer immediately (cheap; avoids a blurry canvas)
        // and redraw, but debounce the world-fit + tile refetch so dragging the
        // window edge doesn't re-fit and refetch on every intermediate size.
        const dpr = window.devicePixelRatio || 1;
        mapApp.canvas.width = width * dpr;
        mapApp.canvas.height = height * dpr;
        requestRender();
        if (resizeTimer) clearTimeout(resizeTimer);
        resizeTimer = setTimeout(() => {
          if (destroyed) return;
          const m2 = metaRef.current;
          if (m2) viewport.fitWorld(m2.grid_width, m2.grid_height, width, height, stretchToFitRef.current);
          refreshTiles();
          requestRender();
        }, 120);
      });
      resizeObserver.observe(el);

      // Observe the outer pane: size the canvas container to the largest box
      // matching the world's aspect ratio so the canvas itself takes the
      // template's proportions (no internal letterbox, no distortion). The dark
      // area around it is simply the empty pane.
      const pane = paneRef.current;
      const paneObserver = new ResizeObserver((entries) => {
        const entry = entries[0];
        const m2 = metaRef.current;
        if (entry && m2) {
          const { width, height } = entry.contentRect;
          setBox(fitBox(width, height, m2.grid_width, m2.grid_height));
        }
      });
      if (pane) paneObserver.observe(pane);

      // Render loop. Also drives the wheel-zoom EASE (`viewport.update`) every
      // frame: a wheel tick only moves the viewport's TARGET (see
      // `TileViewport.onWheel`), so something has to step the displayed
      // scale/position toward it each frame for the EU4/CK3-style smooth-zoom
      // glide to actually appear, instead of the old jump-straight-to-target
      // behaviour. `dt` is clamped so a backgrounded-tab stall (or any long
      // gap between frames) can't make the very next frame leap the ease
      // forward as if a huge amount of time passed.
      let lastFrameTime = 0;
      const loop = (time: number) => {
        if (destroyed) return;
        const dt = lastFrameTime ? Math.min(0.05, (time - lastFrameTime) / 1000) : 0;
        lastFrameTime = time;
        if (viewport.update(dt)) {
          overlayManager.updateScale(viewport.scale);
          refreshTiles();
          needsRenderRef.current = true;
        }
        if (needsRenderRef.current) {
          needsRenderRef.current = false;
          renderFrame();
        }
        rafRef.current = requestAnimationFrame(loop);
      };
      rafRef.current = requestAnimationFrame(loop);

      // Fit world if already loaded
      const m = metaRef.current;
      if (m) {
        viewport.fitWorld(m.grid_width, m.grid_height, el.clientWidth, el.clientHeight, stretchToFitRef.current);
        refreshTiles();
      }

      setInitStatus("ready");
      console.log("[map] Canvas 2D setup complete");
    } catch (err) {
      console.error("[map] init FAILED:", err);
      setInitError(String(err));
      setInitStatus("FAILED");
      initRef.current = false;
    }

    return () => {
      destroyed = true;
      cancelAnimationFrame(rafRef.current);
    };
  }, [refreshTiles, renderFrame, requestRender]);

  // Switching the active layer does NOT wipe the cache — tiles are keyed by
  // (layer, tx, ty), so a previously viewed layer's tiles are still cached and
  // switching back is instant. Just (re)load the now-active layer's visible tiles.
  useEffect(() => {
    refreshTiles();
  }, [activeLayer, isolateClass, elevationStyle, mapTerrain, compareLayer, refreshTiles]);

  // Wipe the cache only when the world changes or its data changes (a sim step
  // bumps tileVersion) — every layer's rendered tiles are then stale.
  useEffect(() => {
    const tm = tileManagerRef.current;
    if (tm) tm.clear();
    refreshTiles();
  }, [worldKey, tileVersion, refreshTiles]);

  // Recompute the aspect-locked canvas box when the world (aspect) changes, so
  // a freshly imported template of a different shape re-proportions the canvas.
  useEffect(() => {
    const pane = paneRef.current;
    if (!pane || !meta) return;
    setBox(fitBox(pane.clientWidth, pane.clientHeight, meta.grid_width, meta.grid_height));
  }, [worldKey]); // eslint-disable-line react-hooks/exhaustive-deps

  // Fit world when the world (dimensions) changes or stretch-to-fit is toggled.
  useEffect(() => {
    const viewport = viewportRef.current;
    const el = containerRef.current;
    if (!viewport || !el || !meta) return;
    viewport.fitWorld(meta.grid_width, meta.grid_height, el.clientWidth, el.clientHeight, stretchToFit);
    const tm = tileManagerRef.current;
    if (tm) tm.clear();
    refreshTiles();
    requestRender();
  }, [worldKey, stretchToFit, box, refreshTiles, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Live settlement markers: when a campaign is running, derive them from the
  // simulation hubs (current population + any new estates) and re-tier each dot
  // by its share of the largest city's population; otherwise show the static
  // Phase-7 list. This makes a dot's size track the LIVE population (#5).
  // Batch 1 · era scrubber: when the Atlas year slider sets a frame, the map's
  // markers (and heat, below) time-travel to that year instead of the live world.
  const eraFrame = useUIStore((s) => s.eraFrame);
  const heatGood = useUIStore((s) => s.heatGood);
  const selectedCulture = useUIStore((s) => s.selectedCulture);
  const colonyHighlight = useUIStore((s) => s.colonyHighlight);

  const liveSettlementsNow = useMemo<Settlement[]>(() => {
    // Estates are INTERNAL to their parent city (co-located, never their own dot).
    // Filter on the `is_estate` FLAG, not the id range: in-campaign-founded towns,
    // satellites and colonies all get id >= 100000 too (from create_organic_town),
    // so an id<100000 test wrongly hid every swarm town, finished satellite and
    // independent ex-colony — they vanished from the map. `!is_estate` keeps them.
    const hubs = campaignSnapshot?.active ? campaignSnapshot.hubs.filter((h) => !h.is_estate) : null;
    if (!hubs || hubs.length === 0) return settlements;
    // Tier by ABSOLUTE population, not as a ratio to the single largest city — a
    // metropolis used to push every other city below 8% of its pop, collapsing real
    // 30k cities to the tiny near-invisible "outpost" tier even though they're alive.
    // Tiers on the campaign's HUMBLE population scale (settlements seed at
    // 500/2000/10000 = village/town/city and grow from there). Applies only to
    // LIVE campaign hubs; worldgen-preview settlements keep their own `s.size`.
    const tier = (pop: number): Settlement["size"] => {
      if (pop >= 35_000) return "capital";
      if (pop >= 8_000) return "city";
      if (pop >= 1_500) return "town";
      if (pop >= 350) return "village";
      return "outpost";
    };
    // The campaign sim only keeps the strongest ~150 settlements as live economic
    // hubs (to bound tick cost). The REST are still real places, so we MERGE: start
    // from the full static settlement list and overlay live population/tier where a
    // hub exists, keeping every other settlement on the map (it just isn't simulated).
    // Without this, every settlement past the top 150 vanished the moment a campaign
    // started.
    // NOTE: static `Settlement.id` is "s-N"/"o-N" but a live hub's id is the plain
    // index, so we match by POSITION (both use the same cell coords), not id.
    const posKey = (x: number, y: number) => `${Math.round(x)},${Math.round(y)}`;
    const liveByPos = new Map(hubs.map((h) => [posKey(h.x, h.y), h]));
    const matched = new Set<string>();
    // Atlas 2.0 lifecycle flags: † ruin when abandoned (or the pop collapsed),
    // gold founding star while a mid-campaign settlement is still young (<15y).
    const nowTick = campaignSnapshot?.clock?.tick ?? 0;
    const isDead = (h: CampaignHubBrief) => h.abandoned || h.population < 100;
    const isNew = (h: CampaignHubBrief) => !isDead(h)
      && h.founded_tick > 0 && nowTick - h.founded_tick < 15 * 365;
    const merged: Settlement[] = settlements.map((s) => {
      const k = posKey(s.x, s.y);
      const h = liveByPos.get(k);
      if (!h) return s; // outside the sim → keep the static marker as-is
      matched.add(k);
      return {
        ...s,
        size: tier(h.population),
        population: h.population,
        score: h.population,
        dead: isDead(h),
        isNew: isNew(h),
        hubClass: h.hub_class ?? 0,
        isTransitKnot: h.is_transit_knot ?? false,
        relayCount: h.relay_count ?? 0,
      };
    });
    // Add any live hubs with no static counterpart (organic towns swarmed and
    // colonies founded in-campaign).
    for (const h of hubs) {
      const k = posKey(h.x, h.y);
      if (matched.has(k)) continue;
      merged.push({
        id: String(h.id), x: h.x, y: h.y, name: h.name,
        size: tier(h.population), population: h.population, score: h.population,
        dead: isDead(h),
        isNew: isNew(h),
        hubClass: h.hub_class ?? 0,
        isTransitKnot: h.is_transit_knot ?? false,
        relayCount: h.relay_count ?? 0,
      });
    }
    return merged;
  }, [campaignSnapshot, settlements]);

  // Batch 1 · era view: rebuild the marker list from the selected past year's
  // frame (dead/new states as they stood THEN); live view passes through.
  const liveSettlements = useMemo<Settlement[]>(() => {
    if (!eraFrame) return liveSettlementsNow;
    // Tiers on the campaign's HUMBLE population scale (settlements seed at
    // 500/2000/10000 = village/town/city and grow from there). Applies only to
    // LIVE campaign hubs; worldgen-preview settlements keep their own `s.size`.
    const tier = (pop: number): Settlement["size"] => {
      if (pop >= 35_000) return "capital";
      if (pop >= 8_000) return "city";
      if (pop >= 1_500) return "town";
      if (pop >= 350) return "village";
      return "outpost";
    };
    return (eraFrame.hubs ?? []).map((h, i) => ({
      id: `era-${i}`, x: h.x, y: h.y, name: h.name,
      size: tier(h.population), population: h.population, score: h.population,
      dead: h.dead, isNew: h.is_new,
    }));
  }, [eraFrame, liveSettlementsNow]);

  // Redraw overlays when data changes
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.drawRivers(rivers);
    om.drawLakes(lakes);
    // While a campaign is running, drive the markers from the LIVE simulation
    // hubs (population shifts, estates appear) instead of the frozen Phase-7 list,
    // so a city's dot grows/shrinks and re-tiers with its current population.
    om.drawSettlements(liveSettlements);
    requestRender();
  }, [rivers, lakes, liveSettlements, requestRender]);

  // Rebuild the unified clickable-settlement candidate list whenever the live sim or
  // the frozen economy changes. Live hubs first (they carry live population/colony
  // state); then every OTHER worldgen/hinterland settlement by position, so sub-cap
  // villages — drawn but not simulated — are just as hoverable/clickable as live hubs.
  useEffect(() => {
    const posKey = (x: number, y: number) => `${Math.round(x)},${Math.round(y)}`;
    const list: { x: number; y: number; id: number; colony: number }[] = [];
    const seen = new Set<string>();
    if (campaignSnapshot?.active) {
      for (const h of campaignSnapshot.hubs) {
        // `is_estate` is an OWNERSHIP flag, not a geography flag (CLAUDE.md rule
        // 32). Most estates are CO-LOCATED inside their parent city and must not
        // be separate click targets — a click there belongs to the city. But a
        // HOUSE TRADE OUTPOST (`colony_kind === 2`) is a REMOTE SITE standing on
        // its own ground, often hundreds of km from any city: the sim's own
        // `is_remote_site` predicate. Excluding every estate made those outposts
        // unclickable — drawn on the map, named on the map, listed in the Colonial
        // Office, and impossible to open. Same failure the rule already records
        // for routing and lifelines, at a call site the rule had not reached.
        if (h.is_estate && (h.colony_kind ?? 0) !== 2) continue;
        list.push({ x: h.x, y: h.y, id: h.id, colony: h.colony_kind ?? 0 });
        seen.add(posKey(h.x, h.y));
      }
    }
    if (economy) {
      for (const h of economy.hubs) {
        if (seen.has(posKey(h.x, h.y))) continue;
        list.push({ x: h.x, y: h.y, id: h.id, colony: 0 });
      }
    }
    clickCandidatesRef.current = list;
  }, [economy, campaignSnapshot]);

  // Colony & house-outpost markers + their routed supply lanes (from the live sim
  // hubs; settlement colonies kind 1, house outposts kind 2).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    const hubs = campaignSnapshot?.active ? campaignSnapshot.hubs : [];
    const markers: ColonyMarker[] = [];
    for (const h of hubs) {
      if (h.colony_kind !== 1 && h.colony_kind !== 2 && h.colony_kind !== 3) continue;
      const f = h.founder_hub >= 0 && h.founder_hub < hubs.length ? hubs[h.founder_hub] : undefined;
      const ownerColor = h.colony_kind === 2
        ? (houses.find((ho) => ho.idx === h.owner_house)?.color ?? "#c9a96a")
        : "";
      markers.push({
        x: h.x, y: h.y, name: h.name, kind: h.colony_kind, stage: h.colony_stage,
        ownerColor, founderX: f ? f.x : -1, founderY: f ? f.y : -1,
      });
    }
    om.drawColonies(markers);
    requestRender();
  }, [campaignSnapshot, houses, requestRender]);

  // Atlas 2.0 · Trade Heat — per-hub yearly throughput from the live sim (updates
  // each New Year when the flow ledger rolls). Batch 1 layers two lenses on top:
  // an ERA FRAME (the Atlas year slider) wins outright; otherwise a single-good
  // filter (Goods Codex 🔥) fetches that good's per-hub volumes. Migration
  // arrows ride the live snapshot: refugee roads fade over ~4 years.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    const hubs = campaignSnapshot?.active ? campaignSnapshot.hubs : [];
    let alive = true;
    if (eraFrame) {
      om.drawTradeHeat((eraFrame.hubs ?? [])
        .filter((h) => h.trade > 0 && !h.dead)
        .map((h) => ({ x: h.x, y: h.y, v: h.trade })));
    } else if (heatGood && campaignSnapshot?.active) {
      campaignGetGoodHeat(heatGood).then((pts) => {
        if (!alive || !overlayManagerRef.current) return;
        overlayManagerRef.current.drawTradeHeat(pts.map((p) => ({ x: p[0], y: p[1], v: p[2] })));
        requestRender();
      }).catch(() => {});
    } else {
      om.drawTradeHeat(hubs
        .filter((h) => !h.is_estate && !h.abandoned && h.trade_volume > 0)
        .map((h) => ({ x: h.x, y: h.y, v: h.trade_volume })));
    }
    const nowTick = campaignSnapshot?.clock?.tick ?? 0;
    om.drawMigrations(eraFrame ? [] : (campaignSnapshot?.migrations ?? []).map((m) => ({
      fx: m[0], fy: m[1], tx: m[2], ty: m[3],
      age: Math.min(1, (nowTick - m[4]) / (4 * 365)),
    })));
    requestRender();
    return () => { alive = false; };
  }, [campaignSnapshot, eraFrame, heatGood, requestRender]);

  // Route-bound migration overlay: fetch the trade-route-following flows and drive the
  // ribbon/dots/focus mode (focus isolates the selected city's inbound flows).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setMigrationMode(migrationMode);
    om.setMigrationFocus(migrationMode === "focus" ? (selectedHub ?? null) : null);
    if (!campaignSnapshot?.active || eraFrame) { om.setMigrationRoutes([]); requestRender(); return; }
    let alive = true;
    campaignGetMigrationRoutes().then((rs) => {
      if (!alive) return;
      om.setMigrationRoutes(rs.map((r) => ({
        path: r.path, culture: r.culture, volume: r.volume, to: r.to_hub, age: r.age_years,
      })));
      requestRender();
    }).catch(() => {});
    return () => { alive = false; };
  }, [campaignSnapshot, eraFrame, migrationMode, selectedHub, requestRender]);

  // Colony/satellite ↔ metropolis link highlight (set by the Colonial panel).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setColonyLink(colonyHighlight);
    requestRender();
  }, [colonyHighlight, requestRender]);

  // #1/#23 · Culture isolation: when a people is picked in the Peoples panel, shade
  // every settlement by that culture's share (halo + inner fill). Refetch yearly.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (!selectedCulture || !campaignSnapshot?.active) {
      om.setCultureShares([], [200, 120, 220]);
      requestRender();
      return;
    }
    let alive = true;
    Promise.all([campaignGetCultures(), campaignCultureHubs(selectedCulture)]).then(([cults, hubs]) => {
      if (!alive || !overlayManagerRef.current) return;
      const color = (cults.find((c) => c.name === selectedCulture)?.color ?? [200, 120, 220]) as [number, number, number];
      overlayManagerRef.current.setCultureShares(hubs.map((h) => ({ x: h[0], y: h[1], share: h[2] })), color);
      requestRender();
    }).catch(() => {});
    return () => { alive = false; };
  }, [selectedCulture, campaignSnapshot?.active, Math.floor((campaignSnapshot?.clock.tick ?? 0) / 365), requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Atlas 2.0 · named trade basins — refetched as years pass while the overlay
  // (or the Atlas Regions tab) wants them.
  const basinsOn = overlayVisibility.tradeBasins ?? false;
  const campaignYear = campaignSnapshot?.active ? Math.floor(campaignSnapshot.clock.tick / 365) : 0;
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (!basinsOn || !campaignSnapshot?.active) {
      om.drawTradeBasins([]);
      requestRender();
      return;
    }
    let alive = true;
    campaignGetTradeBasins().then((bs) => {
      if (!alive || !overlayManagerRef.current) return;
      overlayManagerRef.current.drawTradeBasins(bs.map((b) => ({
        name: b.name, pts: b.pts, cx: b.cx, cy: b.cy,
      })));
      requestRender();
    }).catch(() => {});
    return () => { alive = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [basinsOn, campaignYear, requestRender]);

  // Re-paint overlays when the user changes the appearance palette (the
  // settingsStore has already pushed the new colours into OverlayManager's
  // live `lineColors` registry; we just need a redraw).
  useEffect(() => { requestRender(); }, [lineColors, requestRender]);

  // Trade routes are generated by the Biological-Trade step (gated on step 8),
  // then refreshed on tileVersion bumps / settlement changes.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (!step8Done || settlements.length < 2) {
      om.drawTradeRoutes([]);
      requestRender();
      return;
    }
    computeTradeRoutes(
      settlements.map((s) => ({ x: s.x, y: s.y, score: s.score })),
      rivers.map((r) => ({ points: r.points })),
      bioParams.tradeReach,
      bioParams.maxCrossing,
      bioParams.desertRoutes,
      bioParams.economicRegions,
      bioParams.piracyLevel,
      bioParams.tradeSeason,
      bioParams.calendarMonths,
    ).then((routes) => {
      om.drawTradeRoutes(routes);
      requestRender();
    }).catch(() => {});
  }, [step8Done, settlements, rivers, tileVersion, bioParams.tradeReach, bioParams.maxCrossing, bioParams.desertRoutes, bioParams.economicRegions, bioParams.piracyLevel, bioParams.tradeSeason, bioParams.calendarMonths, requestRender]);

  // All map overlays in ONE IPC round-trip (see `compute_overlays`): wind/current
  // vectors + streamlines, fishery banks, reef/storm danger zones
  // and trade-good regions. They previously fired as ~8 separate calls that each
  // re-locked the DB and re-read the tile cache; now they share one read. Storm
  // zones are seasonal, so the month sliders are deps too.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    const { grid_width, grid_height } = meta;
    computeOverlays().then((o) => {
      om.drawWindArrows(o.vectors.wind, grid_width, grid_height);
      om.drawCurrentStreamlines(o.streamlines);
      om.drawFisheryBanks(o.fishery_banks);
      om.drawReefZones(o.reef_zones);
      om.drawGoodRegions(o.good_regions);
      goodRegionsRef.current = o.good_regions;
      requestRender();
    }).catch(() => {});
    // Monsoon-climate flood areas (Natural Disasters overlay) — koppen-derived, so
    // they're fetched here (world/version), not in the seasonal storm-month effect.
    computeMonsoonZones().then((zones) => {
      om.drawMonsoonZones(zones);
      requestRender();
    }).catch(() => {});
    // Climate bands: ITCZ line + circulation belts (subtropical high / polar front),
    // positioned by the rotation-driven Circulation model.
    computeClimateBands().then((bands) => {
      om.setClimateBands(bands);
      requestRender();
    }).catch(() => {});
    // Peoples / culture territories (organic hearth map) for the Peoples overlay.
    computeCultureRegions().then((regions) => {
      om.drawCultureRegions(regions);
      requestRender();
    }).catch(() => {});
    // Plate motion arrows (§8.24 B2) — tiny, one-shot per-plate data, so it gets
    // its own fetch rather than folding into `compute_overlays`' per-cell payload.
    getPlateMotion().then((arrows) => {
      om.drawPlateMotion(arrows, grid_width);
      om.setPlateLabels(arrows.map((a) => ({ id: a.id, x: a.x, y: a.y, oceanic: a.is_oceanic, name: plateName(a.id, a.is_oceanic, worldKey ?? "world") })));
      requestRender();
    }).catch(() => {});
  }, [worldKey, tileVersion, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Storm zones depend on the seasonal month slider, so they get their OWN light
  // effect (a single command). Scrubbing the month must not recompute every other
  // overlay (that was the storm-slider lag).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    computeStormZones(stormMonth, calendarMonths).then((zones) => {
      om.drawStormZones(zones);
      requestRender();
    }).catch(() => {});
  }, [worldKey, tileVersion, stormMonth, calendarMonths, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Load the world's editable good specs (default 30 or custom) so overlays/labels
  // use the right icons/colors, including any custom goods.
  useEffect(() => { if (meta) void loadGoodsFromWorld(); }, [worldKey, tileVersion, loadGoodsFromWorld]); // eslint-disable-line react-hooks/exhaustive-deps

  // Push per-good display metadata (icon/color) to the overlay manager.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setGoodMeta(new Map(goodsSpecs.map((g) => [g.id, { icon: g.icon, color: g.color }])));
    requestRender();
  }, [goodsSpecs, requestRender]);

  // Province partition overlay (a separate layer, generated after settlements).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.updateProvinces(
      provinceRaster ? {
        data: provinceRaster.data, w: provinceRaster.w, h: provinceRaster.h,
        gridW: provinceRaster.gridW, gridH: provinceRaster.gridH,
      } : null,
      provinces.map((p) => ({
        id: p.id, culture: p.culture, name: p.name, seat_x: p.seat_x, seat_y: p.seat_y,
        // Label anchor + how much room the name has. Absent on pre-anchor worlds →
        // the overlay falls back to the seat and derives a radius from the area.
        label_x: p.label_x, label_y: p.label_y, label_r: p.label_r, cells: p.cells,
      })),
    );
    requestRender();
  }, [provinces, provinceRaster, requestRender]);

  // The province picked on the map (or in the Provinces browser) — outline + seat.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setSelectedProvince(selectedProvince);
    requestRender();
  }, [selectedProvince, provinceRaster, requestRender]);

  // Provinces marked for a batch merge/split (shift-click) — a distinct outline set.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setMarkedProvinces(markedProvinces);
    requestRender();
  }, [markedProvinces, provinceRaster, requestRender]);

  // Province fill opacity (fill only — borders/names/selection stay full strength).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setProvinceOpacity(provinceOpacity);
    requestRender();
  }, [provinceOpacity, requestRender]);

  // GENERATION_UX_REDESIGN_PLAN.md Slice 10 (F11) — per-overlay opacity.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setOverlayOpacity(overlayOpacity);
    requestRender();
  }, [overlayOpacity, requestRender]);

  useEffect(() => { requestRender(); }, [layerOpacity, requestRender]);

  // Province fill style: distinct-per-province vs one flat colour, + custom border.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setProvinceStyle({
      fillMode: provinceFillMode,
      singleColor: provinceSingleColor,
      borderColor: provinceBorderColor,
    });
    requestRender();
  }, [provinceFillMode, provinceSingleColor, provinceBorderColor, requestRender]);

  // Region↔region trade flows (routed + bundled trunks) — a product of the
  // Biological-Trade step, gated by the chosen trade reach.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    if (!step8Done || settlements.length < 2) {
      om.drawTradeTrunks([], meta.grid_width);
      requestRender();
      return;
    }
    computeTradeMatrix(
      settlements.map((s) => ({ x: s.x, y: s.y, score: s.score })),
      rivers.map((r) => ({ points: r.points })),
      bioParams.tradeReach,
      bioParams.maxCrossing,
      bioParams.desertRoutes,
      bioParams.economicRegions,
      bioParams.luxuryBias,
      bioParams.piracyLevel,
    ).then((matrix) => {
      om.drawTradeTrunks(matrix.trunks, meta.grid_width);
      requestRender();
    }).catch(() => {});
  }, [step8Done, worldKey, settlements, rivers, tileVersion, bioParams.tradeReach, bioParams.maxCrossing, bioParams.desertRoutes, bioParams.economicRegions, bioParams.luxuryBias, bioParams.piracyLevel, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Merchant layer — live family/guild routes from the running campaign.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    if (!overlayVisibility.merchantRoutes || !campaignSnapshot?.active) {
      om.drawMerchantRoutes([], meta.grid_width);
      merchantRoutesRef.current = [];
      requestRender();
      return;
    }
    campaignMerchantRoutes().then((routes) => {
      om.drawMerchantRoutes(routes, meta.grid_width);
      merchantRoutesRef.current = routes;
      useUIStore.getState().setRouteLegend(om.merchantRouteLegend());
      requestRender();
    }).catch(() => {});
  }, [overlayVisibility.merchantRoutes, campaignSnapshot?.active, campaignSnapshot?.clock.tick, meta, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Trade ▸ Flows highlight — glowing arrows from a settlement to its partners,
  // set by the Flows subtab (purely frontend, redraws when the selection changes).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    om.setFlowHighlight(flowHighlight, meta.grid_width);
    om.setFlowHighlightPaths([]); // clear stale routes from the previous selection
    requestRender();
  }, [flowHighlight, meta, requestRender]);

  // TECTONICS_AND_ISOLATION_PLAN.md Part A3 — route each highlighted flow
  // segment over the SAME coarse cost grid + crossing rule the Dynamic Trade
  // Flow layer uses, instead of the worldgen trade-route graph
  // `renderFlowHighlight`'s own `laneBetween` fallback is bounded by. A
  // campaign sea lane the worldgen layer never joined used to always fall back
  // to a dashed direct line; now it routes legally whenever the coarse grid
  // actually connects the two points within `maxCrossing` — the dashed
  // fallback (rule 35) only fires for the genuinely unroutable remainder.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta || flowHighlight.length === 0 || !campaignSnapshot?.active) return;
    let alive = true;
    Promise.all(flowHighlight.map((s) =>
      computeCoarseRoute([s.ax, s.ay], [s.bx, s.by], rivers.map((r) => ({ points: r.points })),
        CAMPAIGN_ROUTE_REACH, CAMPAIGN_ROUTE_MAX_CROSSING).catch(() => EMPTY_ROUTE),
    )).then((paths) => {
      if (!alive) return;
      om.setFlowHighlightPaths(paths);
      requestRender();
    });
    return () => { alive = false; };
  }, [flowHighlight, meta, campaignSnapshot?.active, rivers, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Every OTHER lane the map draws — merchant routes, a house's trading web,
  // futures lanes, Goods Atlas flows — goes through `laneBetween` too, and so
  // fell through to the same dashed direct line whenever the worldgen
  // trade-route graph could not join two cities ("merchant/guild spread is using
  // a direct-to-city approach, not the trade routes"). `laneBetween` now records
  // the pairs it could not serve; this drains them, routes each over the coarse
  // cost grid, and hands them back — which re-snaps all of those layers at once.
  //
  // A poll rather than a dependency list: wants are produced by whichever
  // overlay setter last ran, and there are five of them across four effects, so
  // enumerating the triggers would be a list that silently goes stale. The check
  // itself is a Set size test and costs nothing when there is nothing to do.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    let alive = true;
    const tick = async () => {
      const wanted = om.takeWantedLanes();
      if (wanted.length === 0) return;
      const rv = rivers.map((r) => ({ points: r.points }));
      const resolved = await Promise.all(wanted.map(async (want) => ({
        key: want.key,
        // An empty path is cached deliberately: it means "asked, no legal route",
        // and caching it is what stops this re-fetching the same dead pair on
        // every poll. That lane keeps the dashed fallback (rule 35).
        path: await computeCoarseRoute(want.a, want.b, rv,
          CAMPAIGN_ROUTE_REACH, CAMPAIGN_ROUTE_MAX_CROSSING).catch(() => EMPTY_ROUTE),
      })));
      if (!alive) return;
      if (om.setLaneRoutes(resolved)) requestRender();
    };
    const id = window.setInterval(() => { void tick(); }, 600);
    void tick();
    return () => { alive = false; window.clearInterval(id); };
  }, [meta, rivers, bioParams.tradeReach, bioParams.maxCrossing, requestRender]);

  // Futures layer — contractual supply lanes from the running campaign.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    if (!overlayVisibility.futures || !campaignSnapshot?.active) {
      om.drawFutures([], meta.grid_width, null, null);
      futuresLanesRef.current = [];
      requestRender();
      return;
    }
    campaignFuturesLanes().then((lanes) => {
      om.drawFutures(lanes, meta.grid_width, selectedFuturesLane, futuresFocus);
      futuresLanesRef.current = lanes;
      requestRender();
    }).catch(() => {});
  }, [overlayVisibility.futures, campaignSnapshot?.active, campaignSnapshot?.clock.tick, meta, selectedFuturesLane, futuresFocus, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // DLC 3.5 · Dynamic Trade Flow — last year's actual shipped volume, routed over
  // the cost grid + bundled (busiest arteries thickest). Recomputed yearly in the
  // tick, so this only re-fetches when the year ticks over.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    if (!overlayVisibility.dynamicFlow || !campaignSnapshot?.active) {
      om.drawDynamicFlow([], meta.grid_width);
      requestRender();
      return;
    }
    // Debounced: this is an expensive coarse-grid routing recompute. Fast-forwarding
    // through years fires the year key repeatedly; a trailing debounce collapses the
    // burst into ONE fetch after the year settles, so it never stalls the year-tick
    // frame (the year-start lag). ~400 ms is imperceptible when you pause to look.
    const t = window.setTimeout(() => {
      campaignGetTradeFlow(rivers.map((r) => ({ points: r.points })), CAMPAIGN_ROUTE_REACH, CAMPAIGN_ROUTE_MAX_CROSSING)
        .then((trunks) => {
          om.drawDynamicFlow(trunks, meta.grid_width);
          requestRender();
        }).catch(() => {});
    }, 400);
    return () => window.clearTimeout(t);
  }, [overlayVisibility.dynamicFlow, campaignSnapshot?.active, Math.floor((campaignSnapshot?.clock.tick ?? 0) / 365), meta, rivers, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Caravan corridors — long river/terrain-routed hauls owned by houses, strung
  // with waystations. Recomputed on the year tick (like the dynamic flow).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    if (!overlayVisibility.campaignCorridors || !campaignSnapshot?.active) {
      om.drawTradeCorridors([], meta.grid_width);
      requestRender();
      return;
    }
    // Debounced like the dynamic-flow overlay above: campaign_get_corridors runs up
    // to 24 coarse-grid Dijkstra routes, the single biggest new year-boundary cost.
    // A trailing debounce collapses a fast-forward burst into one recompute so it
    // stops stuttering every year. (Superseded later by the event-driven corridor cache.)
    const t = window.setTimeout(() => {
      campaignGetCorridors(rivers.map((r) => ({ points: r.points })), CAMPAIGN_ROUTE_REACH, CAMPAIGN_ROUTE_MAX_CROSSING)
        .then((corridors) => {
          om.drawTradeCorridors(corridors, meta.grid_width);
          requestRender();
        }).catch(() => {});
    }, 400);
    return () => window.clearTimeout(t);
  }, [overlayVisibility.campaignCorridors, campaignSnapshot?.active, Math.floor((campaignSnapshot?.clock.tick ?? 0) / 365), meta, rivers, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // States (§3.3) — a tier 1-2 city's own writ, redrawn on year boundaries. City
  // tier moves monthly and province holders move yearly, so a year-granularity
  // refresh (like campaignCorridors above) is often enough without polling every tick.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    // The Game Board style tints provinces by realm and crowns capitals, so it needs
    // realm data even while the States overlay itself is off.
    if (!(overlayVisibility.states || mapStyle === "game") || !campaignSnapshot?.active) {
      om.drawStates([]);
      requestRender();
      return;
    }
    computeStates().then((states) => {
      const hubs = useCampaignStore.getState().snapshot?.hubs ?? [];
      om.setRealmCapitals(states.flatMap((st) => {
        const h = hubs.find((x) => x.id === st.capital_hub);
        return h ? [[h.x, h.y] as [number, number]] : [];
      }));
      om.drawStates(states);
      requestRender();
    }).catch(() => {});
  }, [overlayVisibility.states, mapStyle, campaignSnapshot?.active, Math.floor((campaignSnapshot?.clock.tick ?? 0) / 365), meta, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // A map-style change can switch the province wash to the realm tint and back.
  useEffect(() => {
    overlayManagerRef.current?.refreshProvinceColors();
    requestRender();
  }, [mapStyle, requestRender]);

  // Expeditions — financed ventures crawling toward distant cities + recent failed
  // ✕'s. Cheap read (no routing), so it refreshes every tick to show them moving.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    if (!overlayVisibility.expeditions || !campaignSnapshot?.active) {
      om.drawExpeditions([], [], meta.grid_width);
      requestRender();
      return;
    }
    campaignGetExpeditions()
      .then((p) => {
        om.drawExpeditions(p.active, p.failed, meta.grid_width);
        requestRender();
      }).catch(() => {});
  }, [overlayVisibility.expeditions, campaignSnapshot?.active, campaignSnapshot?.clock.tick, meta, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Political influence — product of the Political step (9).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    if (!step9Done || settlements.length < 2) {
      om.drawPolitical([]);
      requestRender();
      return;
    }
    computePolitical(
      settlements.map((s) => ({ x: s.x, y: s.y, score: s.score, population: s.population })),
      rivers.map((r) => ({ points: r.points })),
      bioParams.tradeReach,
      bioParams.maxCrossing,
      bioParams.desertRoutes,
      bioParams.economicRegions,
      bioParams.piracyLevel,
    ).then((centers) => {
      om.drawPolitical(centers);
      requestRender();
    }).catch(() => {});
  }, [step9Done, worldKey, settlements, rivers, tileVersion, bioParams.tradeReach, bioParams.maxCrossing, bioParams.desertRoutes, bioParams.economicRegions, bioParams.piracyLevel, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // DLC 3 · speculation risk — the cached yearly read from the live campaign sim.
  // Refetched when the overlay turns on or the campaign clock advances a year.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    if (!overlayVisibility.speculation || !campaignSnapshot?.active) {
      om.drawSpeculation([]);
      requestRender();
      return;
    }
    campaignGetSpeculation().then((centers) => {
      om.drawSpeculation(centers);
      requestRender();
    }).catch(() => {});
  }, [overlayVisibility.speculation, campaignSnapshot?.active, campaignSnapshot?.clock.tick, meta, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Hydrate the persisted economy snapshot when a world loads (survives save/open).
  useEffect(() => {
    if (!worldKey) return;
    getEconomy().then((e) => setEconomy(e.hubs.length > 0 ? e : null)).catch(() => {});
  }, [worldKey, setEconomy]);

  // Goods batch G5 · ore/gem/stone workings for the Deposits layer; refetched when
  // Resources (goods) or People (which marks what towns know) re-runs.
  const step7Known = useUIStore((s) => s.stepCompleted[7]);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !worldKey || !meta || !overlayVisibility.deposits) return;
    getDepositSites().then((s) => { om.setDepositSites(s, meta.grid_width); requestRender(); }).catch(() => {});
    // During a campaign, discoveries change it yearly (G5b).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [worldKey, step8Done, step7Known, overlayVisibility.deposits, campaignSnapshot?.active, Math.floor((campaignSnapshot?.clock.tick ?? 0) / 365)]);

  // Owner 2026-10-17: the map shows every good; in a campaign the belts nobody works
  // yet are tinted and the exploited ones stay clear. Refreshed yearly.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (!campaignSnapshot?.active) { om.setExploitedGoods(null); requestRender(); return; }
    campaignExploitedGoods().then((rows) => {
      om.setExploitedGoods(new Map(rows.map((r) => [r.good, new Set(r.provinces)])));
      requestRender();
    }).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [campaignSnapshot?.active, Math.floor((campaignSnapshot?.clock.tick ?? 0) / 365)]);

  // Economy chokepoints + trade-region territories — product of the Economy step.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.drawChokepoints(economy?.chokepoints ?? []);
    om.drawEconRegions(economy?.regions ?? []);
    om.drawCorridors(economy?.corridors ?? [], meta?.grid_width ?? 0);
    requestRender();
  }, [economy, meta, requestRender]);

  // Adjustable trade-hub marker display (size + highlight intensity).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setHubDisplay(hubDisplay.size, hubDisplay.intensity);
    requestRender();
  }, [hubDisplay, requestRender]);

  // Reach-network highlight (red routes + hub rings). One source at a time, by
  // priority: the route window's independently-opened roads > a hub's selected
  // export good (its destinations) > the toolbar per-good reach view.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    let chains: EconChain[] = [];
    if (economy) {
      if (openRoads.length > 0) {
        const set = new Set(openRoads);
        chains = economy.chains.filter((c) => set.has(c.id));
      } else if (selectedExport && selectedHub !== null) {
        // Export flows leaving the selected hub for the chosen good (origin = stop 0).
        chains = economy.chains.filter((c) =>
          c.good_name === selectedExport && c.stops.length > 0 && c.stops[0].hub === selectedHub);
      } else if (reachGood) {
        chains = economy.chains.filter((c) => c.good_name === reachGood);
      }
    }
    // Destination hub = last stop of each chain; dedupe by position.
    const seen = new Set<string>();
    const hubs: [number, number][] = [];
    for (const c of chains) {
      const last = c.points[c.points.length - 1];
      if (!last) continue;
      const key = `${last[0]},${last[1]}`;
      if (seen.has(key)) continue;
      seen.add(key);
      hubs.push([last[0], last[1]]);
    }
    om.setReachNetwork(chains, hubs);
    requestRender();
  }, [economy, reachGood, openRoads, selectedExport, selectedHub, requestRender]);

  // Single import-trace road (hub inspector's Imports column) → drawn BLUE.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    const chain = (economy && selectedChain !== null)
      ? economy.chains.find((c) => c.id === selectedChain) ?? null : null;
    om.setSupplyChain(chain, true);
    requestRender();
  }, [economy, selectedChain, requestRender]);

  // Sync overlay visibility
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    for (const [type, visible] of Object.entries(overlayVisibility)) {
      om.setVisible(type, visible && !(appMode === "chronicle" && FORGE_ONLY.has(type))
        && !(activeLayer === "political" && (type === "provinces" || type === "states")));
    }
    // The Fisheries layer carries its grand-bank circles; circulation belts ride on Wind.
    om.setVisible("fisheryBanks", activeLayer === "fisheries");
    om.setVisible("plateNames", activeLayer === "plates");
    if (meta) om.setWorldWidth(meta.grid_width);
    om.setVisible("windBelts", !!overlayVisibility.wind);
    requestRender();
  }, [overlayVisibility, activeLayer, appMode, meta, requestRender]);

  // POLITICAL LAYER: load realms/culture/order/goods shares whenever the layer is on and
  // the campaign year turns (or the provinces change), and hand them to the renderer.
  const campaignActive = !!campaignSnapshot?.active;
  const polYear = Math.floor((campaignSnapshot?.clock.tick ?? 0) / 365);
  useEffect(() => {
    if (activeLayer !== "political" || !provinceRaster) return;
    loadPolitical(campaignActive).catch(() => {});
  }, [activeLayer, provinceRaster, provinces, campaignActive, polYear]);
  const politicalVersion = usePoliticalStore((s) => s.version);
  const political = usePoliticalStore();
  useEffect(() => {
    const names = new Map(provinces.map((p) => [p.id, p.name]));
    realmLayerRef.current.setData({
      raster: provinceRaster, owner: political.owner, names, cultures: political.culture,
      unrest: political.unrest, shares: political.shares, pop: political.pop,
    });
    requestRender();
  }, [provinceRaster, provinces, politicalVersion, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    realmLayerRef.current.setView({
      fill: political.fill, borders: political.borders, goods: political.goods, names: political.names, popMode: political.popMode,
      selected: selectedProvince, goodFilter: political.goodFilter,
    });
    requestRender();
  }, [political.fill, political.borders, political.goods, political.names, political.popMode, political.goodFilter, selectedProvince, requestRender]);

  // The picked plate rings its name on the Plates layer.
  useEffect(() => {
    overlayManagerRef.current?.setSelectedPlate(selectedPlate);
    requestRender();
  }, [selectedPlate, requestRender]);

  // CLAUDE.md §8.19 (goods localities, shipped) Slice 5 · the FULL-RESOLUTION belt masks, fetched for
  // the goods actually toggled on. They are a real payload (a full-resolution
  // coverage mask per good), which is exactly why the command takes a list rather
  // than returning every good the way `compute_overlays` does — and why this effect
  // keys on the SET of enabled goods, not on `overlayVisibility` as a whole (which
  // changes whenever any unrelated overlay is toggled).
  const enabledGoodsKey = useMemo(() => {
    const on: string[] = [];
    for (const [k, v] of Object.entries(overlayVisibility)) {
      if (v && k.startsWith("good_")) on.push(k.slice(5));
    }
    return on.sort().join(",");
  }, [overlayVisibility]);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    if (!enabledGoodsKey) { om.drawGoodBeltMasks([]); requestRender(); return; }
    let stale = false;
    computeGoodBeltMasks(enabledGoodsKey.split(",")).then((masks) => {
      if (stale) return;
      om.drawGoodBeltMasks(masks);
      requestRender();
    }).catch(() => {});
    return () => { stale = true; };
  }, [enabledGoodsKey, worldKey, tileVersion, meta, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // D10 · push the SERVED absolute quality scale into the overlay manager, so the
  // belt wash reads from the same table Rust holds rather than a copy (§8.18).
  useEffect(() => { void loadPalettes(); }, [loadPalettes]);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !palettes) return;
    om.setGoodQualityScale(palettes.good_quality, palettes.good_quality_pale, palettes.good_quality_heatmap);
    requestRender();
  }, [palettes, requestRender]);
  // Heat-ramp reading of the same scale (dark blue → red), toggled independently.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setGoodQualityHeatmap(goodQualityHeatmap);
    requestRender();
  }, [goodQualityHeatmap, requestRender]);
  // Merchant Routes: colour by voyage risk, or by network / by good.
  const routeColorMode = useUIStore((s) => s.routeColorMode);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setMerchantRouteRisk(merchantRouteRisk);
    om.setRouteColorMode(routeColorMode);
    useUIStore.getState().setRouteLegend(merchantRouteRisk ? [] : om.merchantRouteLegend());
    requestRender();
  }, [merchantRouteRisk, routeColorMode, requestRender]);

  // Merchant-family control overlay — which settlements each house holds. Driven
  // by the live campaign houses (updates dynamically as the campaign advances).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.drawHouseControl(houses, meta?.grid_width ?? 0, selectedHouseIdx);
    requestRender();
  }, [houses, meta, selectedHouseIdx, requestRender]);

  // S11 (HOUSES_GUILDS_AND_MARKET_PLAN.md) · the focused house's own trade
  // atlas, so its map web draws with real per-lane volume/dominant-good
  // instead of the plain flat-red style. Fetched only while a house is
  // selected; `setHouseAtlas` itself guards against a stale fetch outliving
  // a house switch (its own `houseAtlasFor` check).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (selectedHouseIdx == null) { om.setHouseAtlas(null, null); requestRender(); return; }
    let alive = true;
    campaignHouseAtlas(selectedHouseIdx).then((atlas) => {
      if (!alive) return;
      om.setHouseAtlas(selectedHouseIdx, atlas);
      requestRender();
    }).catch(() => {});
    return () => { alive = false; };
  }, [selectedHouseIdx, requestRender]);

  // The house web's good filter (the dossier's "lanes by good" window).
  const houseLaneGood = useUIStore((s) => s.houseLaneGood);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setHouseLaneGood(houseLaneGood);
    requestRender();
  }, [houseLaneGood, requestRender]);

  // Draw latitude lines. Driven by the live `latConfig` slice (not `meta`) so
  // dragging the Latitude Frame sliders repaints ONLY this overlay — no tile
  // reloads, no sim IPC.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    om.drawLatLines(meta.grid_width, meta.grid_height, latConfig.equatorOffset, latConfig.latScale, latConfig.lineRatio);
    requestRender();
  }, [worldKey, latConfig, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // 2026-09-30b · a person's road (Character window ▸ "Show on the map").
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setPersonRoad(personRoad);
    requestRender();
  }, [personRoad, requestRender]);

  // Center the camera when a focus target is requested (e.g. clicking a city).
  useEffect(() => {
    const viewport = viewportRef.current;
    const om = overlayManagerRef.current;
    const el = containerRef.current;
    if (!viewport || !el || !focusTarget) return;
    viewport.centerOn(focusTarget.wx, focusTarget.wy, el.clientWidth, el.clientHeight);
    if (om) om.updateScale(viewport.scale);
    refreshTiles();
    requestRender();
  }, [focusTarget, refreshTiles, requestRender]);

  // Drop a transient highlight pin on the searched settlement, then clear it.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !searchPin) return;
    om.setSearchPin(searchPin.wx, searchPin.wy);
    requestRender();
    const t = setTimeout(() => { om.clearSearchPin(); requestRender(); }, 5000);
    return () => clearTimeout(t);
  }, [searchPin, requestRender]);

  // War highlight: light the two belligerent cities (attacker red / defender blue)
  // while a war is picked in the War Council; cleared when the panel clears it.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (warHighlight) om.setWarHighlight(warHighlight.ax, warHighlight.ay, warHighlight.bx, warHighlight.by);
    else om.clearWarHighlight();
    requestRender();
  }, [warHighlight, requestRender]);

  // Coin-usage overlay: when a coin is selected in the Currencies panel, fetch the
  // per-city settlement snapshot and tint the map by use of that coin.
  const coinOverlayHub = useUIStore((s) => s.coinOverlayHub);
  const coinDominanceOn = overlayVisibility.coinDominance ?? false;
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    // Fetch the per-city coin snapshot when EITHER the all-coins dominance map is on
    // or a coin is selected for drill-down; the same data feeds both renders.
    if ((coinOverlayHub == null && !coinDominanceOn) || !campaignSnapshot?.active) {
      om.setCoinUsage([], null); requestRender(); return;
    }
    let alive = true;
    campaignCoinUsage()
      .then((u) => { if (alive) { om.setCoinUsage(u, coinOverlayHub); requestRender(); } })
      .catch(() => {});
    return () => { alive = false; };
  }, [coinOverlayHub, coinDominanceOn, campaignSnapshot?.active, campaignSnapshot?.clock.tick, requestRender]);

  // #23 · the chosen itinerary route, pushed from the Itinerary panel via uiStore.
  const travelRoute = useUIStore((s) => s.travelRoute);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.drawTravelRoute(travelRoute ?? []);
    requestRender();
  }, [travelRoute, requestRender]);

  // Ridge-drawing tool: sync the committed drawn lines onto the overlay (the
  // in-progress line is pushed directly during a drag).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setRidgeSketch(ridgeLines);
    requestRender();
  }, [ridgeLines, requestRender]);

  // Landmass lasso: sync the committed selection polygon onto the overlay (the
  // in-progress polygon is pushed directly during a drag). Clearing the store
  // value (an op consumed it, or the user pressed Clear) hides the sketch.
  const lassoPolygon = useUIStore((s) => s.lassoPolygon);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setLassoSketch(lassoPolygon);
    requestRender();
  }, [lassoPolygon, requestRender]);

  // GENERATION_UX_REDESIGN_PLAN.md Slice 6 — sync drawn plate seeds onto the overlay.
  const plateSeeds = useUIStore((s) => s.plateSeeds);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setPlateSeedDrafts(plateSeeds);
    requestRender();
  }, [plateSeeds, requestRender]);

  // 🌊 Hydrology · the selected river system's subtree glows on the map, each
  // tributary in its own colour (branch / order scheme).
  const riverHighlight = useUIStore((s) => s.riverHighlight);
  const riverHighlightColors = useUIStore((s) => s.riverHighlightColors);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setRiverHighlight(riverHighlight, riverHighlightColors);
    requestRender();
  }, [riverHighlight, riverHighlightColors, requestRender]);

  // 🌊 Hydrology · the lake selected in the Lakes tab glows; others dim.
  const lakeHighlight = useUIStore((s) => s.lakeHighlight);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setLakeHighlight(lakeHighlight);
    requestRender();
  }, [lakeHighlight, requestRender]);

  // 🌊 Load classified river/lake SYSTEMS on world load, so the map can label
  // rivers/lakes (and mark reach breaks) even when the optional Toponyms step
  // (#26) was never run. River names are settlement-independent, so this is keyed
  // on rivers/lakes only (settlements read live) to avoid campaign-growth churn.
  useEffect(() => {
    if (rivers.length === 0 && lakes.length === 0) {
      useWorldStore.getState().setRiverSystems([]);
      useWorldStore.getState().setLakeSystems([]);
      return;
    }
    let cancelled = false;
    const sts = useWorldStore.getState().settlements.map((s) => ({ x: s.x, y: s.y, name: s.name, size: s.size }));
    getRiverSystems(rivers, sts).then((rs) => { if (!cancelled) useWorldStore.getState().setRiverSystems(rs); }).catch(() => {});
    getLakeSystems(lakes, rivers).then((ls) => { if (!cancelled) useWorldStore.getState().setLakeSystems(ls); }).catch(() => {});
    return () => { cancelled = true; };
  }, [rivers, lakes]);

  // #26 · geographic toponym labels. Prefer the saved toponyms (from the Toponyms
  // step); otherwise synthesize hydronyms from the loaded river/lake systems so
  // river & lake names always appear on the map.
  const toponyms = useWorldStore((s) => s.toponyms);
  const riverSystems = useWorldStore((s) => s.riverSystems);
  const lakeSystems = useWorldStore((s) => s.lakeSystems);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (toponyms.length > 0) {
      om.drawToponyms(toponyms);
    } else {
      const synth: Toponym[] = [];
      for (const r of riverSystems) {
        if (!r.tributary && r.name) synth.push({ kind: "river", name: r.name, x: r.mid_x, y: r.mid_y });
      }
      for (const l of lakeSystems) {
        if (l.name) synth.push({ kind: "lake", name: l.name, x: l.cx, y: l.cy });
      }
      om.drawToponyms(synth);
      if (synth.length > 0) useUIStore.getState().setOverlayVisible("toponyms", true);
    }
    requestRender();
  }, [toponyms, riverSystems, lakeSystems, requestRender]);

  // 🌊 Reach breaks: mark where a trunk river changes upper→middle→delta. The
  // zone boundary km (from the river systems) is mapped onto the river's cell
  // polyline by cumulative arc-length (both monotonic along the same path).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    const firstWord = (s: string) => (s || "").split(" ")[0];
    const fmtKm = (km: number) => `${Math.round(km).toLocaleString()} km`;
    const breaks: { x: number; y: number; tx: number; ty: number; label: string }[] = [];
    for (const node of riverSystems) {
      if (node.tributary || node.order < 3 || node.length_km < 1) continue;
      const zones = node.zones ?? [];
      if (zones.length < 2) continue;
      const pts = rivers[node.id]?.points;
      if (!pts || pts.length < 3) continue;
      // Cumulative cell-length along the channel.
      const cum: number[] = [0];
      for (let i = 1; i < pts.length; i++) {
        cum.push(cum[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
      }
      const total = cum[cum.length - 1] || 1;
      for (let z = 0; z < zones.length - 1; z++) {
        const frac = Math.max(0, Math.min(1, zones[z].end_km / node.length_km));
        const target = frac * total;
        // Locate the segment containing `target`.
        let i = 1;
        while (i < cum.length - 1 && cum[i] < target) i++;
        const seg = (cum[i] - cum[i - 1]) || 1;
        const t = Math.max(0, Math.min(1, (target - cum[i - 1]) / seg));
        const ax = pts[i - 1][0], ay = pts[i - 1][1], bx = pts[i][0], by = pts[i][1];
        breaks.push({
          x: ax + (bx - ax) * t,
          y: ay + (by - ay) * t,
          tx: bx - ax,
          ty: by - ay,
          label: `${firstWord(zones[z].label)} › ${firstWord(zones[z + 1].label)} · ${fmtKm(zones[z].end_km)}`,
        });
      }
    }
    om.drawRiverBreaks(breaks);
    requestRender();
  }, [riverSystems, rivers, requestRender]);

  // #37 · per-good scarcity discs: when the overlay is on and a good is chosen in
  // the Goods Codex, colour each hub by its local price premium for that good.
  const codexGood = useUIStore((s) => s.codexGood);
  const goodScarcityOn = useUIStore((s) => s.overlayVisibility.goodScarcity);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (!goodScarcityOn || !codexGood || !economy) { om.drawGoodScarcity([]); requestRender(); return; }
    const cities: { x: number; y: number; premium: number }[] = [];
    for (const h of economy.hubs) {
      const mg = h.market?.prices.find((p) => p.good_name === codexGood);
      if (!mg || mg.base_value <= 0) continue;
      cities.push({ x: h.x, y: h.y, premium: mg.price / mg.base_value });
    }
    om.drawGoodScarcity(cities);
    requestRender();
  }, [goodScarcityOn, codexGood, economy, requestRender]);

  // Goods Atlas · the selected good's yearly flow, snapped onto the existing trade
  // routes (OverlayManager routes each lane; off-network lanes are dropped, never a
  // straight slash — the maintainer's MUST). Refreshes on good or campaign YEAR change.
  const goodFlowOn = useUIStore((s) => s.overlayVisibility.goodFlow);
  const goodFlowColors = useUIStore((s) => s.goodFlowColors);
  const goodFlowColorGlobal = useUIStore((s) => s.goodFlowColorGlobal);
  const campYear = campaignSnapshot?.clock?.year ?? 0;
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    const gw = meta.grid_width;
    if (!goodFlowOn || !codexGood || !campaignSnapshot?.active) {
      om.drawGoodFlows([], gw, "#e0b24a"); requestRender(); return;
    }
    let stale = false;
    // Colour resolution: a global override wins, else this good's own override, else
    // its `GOOD_DEFS` default (the halo under the lane keeps even a pale one legible).
    const color = goodFlowColorGlobal
      ?? goodFlowColors[codexGood]
      ?? GOOD_DEFS.find((d) => d.name === codexGood)?.color ?? "#e0b24a";
    campaignGoodAtlas(codexGood).then((atlas) => {
      if (stale) return;
      om.drawGoodFlows(atlas.flows, gw, color);
      requestRender();
    }).catch(() => {});
    return () => { stale = true; };
  }, [goodFlowOn, codexGood, campYear, campaignSnapshot?.active, meta, goodFlowColors, goodFlowColorGlobal, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Bank icons: mark each live bank's seat on the map (toggle in the Toolbar).
  const showBankIcons = useUIStore((s) => s.showBankIcons);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (!showBankIcons || !campaignSnapshot?.active) {
      om.setBankIcons([]); requestRender(); return;
    }
    let alive = true;
    campaignGetBanks()
      .then((banks) => {
        if (!alive) return;
        om.setBankIcons(banks.map((b) => ({ x: b.seat_x, y: b.seat_y, name: b.name, defunct: b.defunct, color: b.color })));
        requestRender();
      })
      .catch(() => {});
    return () => { alive = false; };
  }, [showBankIcons, campaignSnapshot?.active, campaignSnapshot?.clock.tick, requestRender]);

  // Phase 6 · plague overlay: struck cities + contagion routes (source→city).
  const showPlagueZones = useUIStore((s) => s.overlayVisibility.plagueZones);
  const plagueReplay = useUIStore((s) => s.plagueReplay);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    // The layer shows when the toggle is on OR a spread REPLAY is running.
    if ((!showPlagueZones && !plagueReplay) || !campaignSnapshot?.active) { om.setEpidemics([], []); requestRender(); return; }
    let alive = true;
    campaignGetEpidemics().then((eps) => {
      if (!alive) return;
      if (plagueReplay) {
        // REPLAY one outbreak: reveal its spread up to `step` (0 = origin), tracing the
        // contagion route in plague-red with the origin city bolded.
        const ep = eps.find((e) => e.id === plagueReplay.id);
        const cs = ep ? ep.cities.filter((c) => c.order <= plagueReplay.step) : [];
        const cities = cs.map((c) => ({ x: c.x, y: c.y, active: true, deaths: c.deaths, origin: c.order === 0 }));
        const coord = new Map<string, { x: number; y: number }>();
        for (const c of ep?.cities ?? []) coord.set(c.name, { x: c.x, y: c.y });
        const edges = cs.filter((c) => c.order > 0).flatMap((c) => {
          const src = c.from_name ? coord.get(c.from_name) : undefined;
          return src ? [{ ax: src.x, ay: src.y, bx: c.x, by: c.y }] : [];
        });
        om.setEpidemics(cities, edges);
        requestRender();
        return;
      }
      // Only ACTIVE outbreaks are shown on the map (user rule) — historical strikes
      // live in the Plagues panel, not as permanent map zones.
      const cities = eps.flatMap((e) => e.cities.filter((c) => c.active)
        .map((c) => ({ x: c.x, y: c.y, active: true, deaths: c.deaths, origin: c.order === 0 })));
      const coord = new Map<string, { x: number; y: number }>();
      for (const e of eps) for (const c of e.cities) coord.set(c.name, { x: c.x, y: c.y });
      const edges = eps.flatMap((e) => e.cities.filter((c) => c.active).flatMap((c) => {
        const src = c.from_name ? coord.get(c.from_name) : undefined;
        return src ? [{ ax: src.x, ay: src.y, bx: c.x, by: c.y }] : [];
      }));
      om.setEpidemics(cities, edges);
      requestRender();
    }).catch(() => {});
    return () => { alive = false; };
  }, [showPlagueZones, plagueReplay, campaignSnapshot?.active, campaignSnapshot?.clock.tick, requestRender]);

  // Phase 6 · guild-city overlay: markers with each guild's good emoji.
  const showGuildCities = useUIStore((s) => s.overlayVisibility.guildCities);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (!showGuildCities || !campaignSnapshot?.active) { om.setGuilds([]); requestRender(); return; }
    let alive = true;
    const color: Record<string, string> = Object.fromEntries(GOOD_DEFS.map((g) => [g.name, g.color]));
    campaignGetGuilds().then((guilds) => {
      if (!alive) return;
      om.setGuilds(guilds.map((g) => ({ x: g.x, y: g.y, good: g.good_name, color: color[g.good_name] ?? "#c0a040", label: g.exceptional ? g.brand : "" })));
      requestRender();
    }).catch(() => {});
    return () => { alive = false; };
  }, [showGuildCities, campaignSnapshot?.active, campaignSnapshot?.clock.tick, requestRender]);

  // Phase 7 · living notable figures as map markers (role emoji).
  const showFigureMarks = useUIStore((s) => s.overlayVisibility.figureMarks);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (!showFigureMarks || !campaignSnapshot?.active) { om.setFigureMarks([]); requestRender(); return; }
    let alive = true;
    const roleEmoji: Record<string, string> = { "Admiral": "⚓", "Demagogue": "📢", "Master Craftsman": "⚒", "Great Banker": "🏦", "Explorer": "🧭" };
    campaignGetFigures().then((figs) => {
      if (!alive) return;
      om.setFigureMarks(figs.filter((f) => f.alive).map((f) => ({ x: f.x, y: f.y, emoji: roleEmoji[f.role] ?? "⚜️" })));
      requestRender();
    }).catch(() => {});
    return () => { alive = false; };
  }, [showFigureMarks, campaignSnapshot?.active, campaignSnapshot?.clock.tick, requestRender]);

  // Phase 7 · landmarks & sacred sites as map markers (kind emoji).
  const showLandmarks = useUIStore((s) => s.overlayVisibility.landmarks);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (!showLandmarks || !campaignSnapshot?.active) { om.setLandmarkMarks([]); requestRender(); return; }
    let alive = true;
    const kindEmoji: Record<string, string> = { wonder: "🗿", temple: "⛪", fair: "🎪", guildhall: "🏛" };
    campaignGetLandmarks().then((lms) => {
      if (!alive) return;
      om.setLandmarkMarks(lms.map((l) => ({ x: l.x, y: l.y, emoji: kindEmoji[l.kind] ?? "🗿" })));
      requestRender();
    }).catch(() => {});
    return () => { alive = false; };
  }, [showLandmarks, campaignSnapshot?.active, campaignSnapshot?.clock.tick, requestRender]);

  // Phase 7 · dynasty ties (alliances + feuds) between house seat cities.
  const showDynastyLinks = useUIStore((s) => s.overlayVisibility.dynastyLinks);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (!showDynastyLinks || !campaignSnapshot?.active) { om.setDynastyLinks([]); requestRender(); return; }
    let alive = true;
    campaignGetDynasties().then((d) => {
      if (!alive) return;
      const links = [
        ...d.alliances.map((l) => ({ ax: l.ax, ay: l.ay, bx: l.bx, by: l.by, ally: true })),
        ...d.feuds.map((l) => ({ ax: l.ax, ay: l.ay, bx: l.bx, by: l.by, ally: false })),
      ];
      om.setDynastyLinks(links);
      requestRender();
    }).catch(() => {});
    return () => { alive = false; };
  }, [showDynastyLinks, campaignSnapshot?.active, campaignSnapshot?.clock.tick, requestRender]);

  // #5: selecting a hub from ANY list/panel (Richest Cities, Trade matrix, Houses,
  // Banks, …) recenters the map on that city AND drops the shiny highlight pin — so
  // clicking a city name jumps the view there and makes it glow.
  const focusOn = useViewportStore((s) => s.focusOn);
  const setSearchPin = useViewportStore((s) => s.setSearchPin);
  useEffect(() => {
    if (selectedHub == null) return;
    const hub = campaignSnapshot?.hubs.find((h) => h.id === selectedHub)
      ?? economyRef.current?.hubs.find((h) => h.id === selectedHub);
    if (hub && typeof hub.x === "number" && typeof hub.y === "number") {
      focusOn(hub.x + 0.5, hub.y + 0.5);
      setSearchPin(hub.x + 0.5, hub.y + 0.5);
    }
  }, [selectedHub]); // eslint-disable-line react-hooks/exhaustive-deps

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === "z") {
        e.preventDefault();
        try {
          const modified = await undoAction();
          if (modified) {
            tileManagerRef.current?.invalidate(modified);
            refreshTiles();
          }
        } catch (err) { console.error("Undo failed:", err); }
      } else if (e.ctrlKey && (e.key === "y" || (e.shiftKey && e.key === "Z"))) {
        e.preventDefault();
        try {
          const modified = await redoAction();
          if (modified) {
            tileManagerRef.current?.invalidate(modified);
            refreshTiles();
          }
        } catch (err) { console.error("Redo failed:", err); }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [refreshTiles]);

  const getPaintMode = useCallback((erasing: boolean): string => {
    const tool = activeToolRef.current;
    if (tool === "elevation") return erasing ? "elevation-erase" : "elevation";
    if (tool === "shelf") return erasing ? "shelf-erase" : "shelf";
    if (tool === "volcano") return "volcano";
    return erasing ? "sea" : "land";
  }, []);

  const getBrushScreenRadius = useCallback((): number => {
    const viewport = viewportRef.current;
    if (!viewport) return 10;
    return brushRadiusRef.current * viewport.scale;
  }, []);

  const applyBrush = useCallback((e: React.PointerEvent) => {
    const viewport = viewportRef.current;
    const rect = containerRef.current?.getBoundingClientRect();
    const m = metaRef.current;
    if (!viewport || !rect || !m) return;

    const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
    const r = brushRadiusRef.current;
    for (let dy = -r; dy <= r; dy++) {
      for (let dx = -r; dx <= r; dx++) {
        if (dx * dx + dy * dy > r * r) continue;
        const cx = wx + dx;
        const cy = wy + dy;
        if (cy < 0 || cy >= m.grid_height) continue;
        const wrappedX = ((cx % m.grid_width) + m.grid_width) % m.grid_width;
        // Numeric key (y*width + x) avoids per-cell string alloc on dense strokes.
        pendingCellsRef.current.add(cy * m.grid_width + wrappedX);
      }
    }

    const screenX = e.clientX - rect.left;
    const screenY = e.clientY - rect.top;
    paintStamp(screenX, screenY, getBrushScreenRadius(), getPaintMode(isErasingRef.current));
  }, [getBrushScreenRadius, getPaintMode]);

  const setInspectedCell = useUIStore((s) => s.setInspectedCell);

  const onPointerDown = useCallback((e: React.PointerEvent) => {
    const viewport = viewportRef.current;
    const m = metaRef.current;
    if (!viewport || !m) return;
    politicalDownRef.current = null;

    if (e.button === 2 || e.altKey) {
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
        if (wx >= 0 && wx < m.grid_width && wy >= 0 && wy < m.grid_height) {
          setInspectedCell({ wx, wy });
        }
      }
      return;
    }

    // Hub selection (Phase 3): a left-click on/near a trade hub opens its
    // inspector. Only with the pan/select tools so painting is never hijacked.
    if (e.button === 0 && (activeToolRef.current === "pan" || activeToolRef.current === "select")) {
      const econ = economyRef.current;
      if (econ && econ.hubs.length > 0) {
        const rect = containerRef.current?.getBoundingClientRect();
        if (rect) {
          const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
          // Screen-space hit radius: ~10px regardless of zoom, capped at 8 world cells
          // so clicking 5 cm away never accidentally selects a settlement when zoomed in.
          const thresh = Math.min(8, 10 / viewport.scaleX);
          // A bank icon takes click priority when bank icons are shown → open the Bank panel.
          if (useUIStore.getState().showBankIcons) {
            const bi = overlayManagerRef.current?.bankIconAt(wx, wy, thresh) ?? -1;
            if (bi >= 0) {
              useUIStore.getState().setSelectedBankIdx(bi);
              useUIStore.getState().setShowBank(true);
              return;
            }
          }
          // Hit-test the SAME unified candidate list the map draws — live hubs AND
          // sub-cap worldgen/hinterland villages, deduped by position (live hub wins).
          // Picking the truly-nearest dot means every drawn settlement is selectable and
          // a click never gets captured by a farther live hub. Markers are drawn at the
          // cell CENTRE (h.x+0.5), so hit-test the centre (else every click reads half a
          // cell up-left). A click on a colony/outpost also opens the Colonial Office.
          const cands = clickCandidatesRef.current;
          let best = -1; let bestD = thresh * thresh; let bestColony = 0;
          for (const h of cands) {
            let dx = Math.abs(h.x + 0.5 - wx);
            if (dx > m.grid_width / 2) dx = m.grid_width - dx;
            const dy = h.y + 0.5 - wy;
            const d = dx * dx + dy * dy;
            if (d < bestD) { bestD = d; best = h.id; bestColony = h.colony; }
          }
          if (best >= 0) {
            setSelectedHub(best);
            if (bestColony === 1 || bestColony === 2) useUIStore.getState().setShowColonial(true);
            return;
          }
          // No hub hit — if the merchant layer is on, try to pick a route.
          const om = overlayManagerRef.current;
          if (om && merchantRoutesRef.current.length > 0) {
            const route = om.pickMerchantRoute(wx, wy, Math.max(4, m.grid_width * 0.008));
            if (route) { setSelectedMerchantRoute(route, om.siblingMerchantLeg(route)); return; }
          }
          // …or, if the futures layer is on, pick a contract lane.
          if (om && futuresLanesRef.current.length > 0) {
            const lane = om.pickFuturesLane(wx, wy, Math.max(4, m.grid_width * 0.008));
            if (lane) { setSelectedFuturesLane(lane); return; }
          }
        }
      }

      // Good-belt selection: a click inside a VISIBLE good region opens the
      // good-flow panel (routes + price/days graph for that good).
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect && goodRegionsRef.current.length > 0) {
        const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
        const vis = overlayVisibilityRef.current;
        for (const r of goodRegionsRef.current) {
          if (!vis[goodOverlayKey(r.good)]) continue;
          const cs = r.cell_size;
          let hit = false;
          for (const [cx, cy] of r.cells) {
            if (wx >= cx && wx < cx + cs && wy >= cy && wy < cy + cs) { hit = true; break; }
          }
          if (hit) { setSelectedGood(r.good); return; }
        }
      }

      // Province selection — LAST, so a settlement, route, futures lane or good
      // belt always wins the click; the province is what's underneath everything.
      // Gated on the province layer being visible so it never hijacks ordinary map
      // use. The full-resolution raster already lives on the client, so this is one
      // array read, no IPC.
      const politicalPick = useUIStore.getState().activeLayer === "political" && realmLayerRef.current.ready;
      if (rect && politicalPick) {
        const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
        politicalDownRef.current = { x: e.clientX, y: e.clientY, wx, wy };
      } else if (rect && overlayVisibilityRef.current.provinces) {
        const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
        const pid = overlayManagerRef.current?.provinceAt(wx, wy) ?? null;
        if (pid !== null) {
          // Shift-click MARKS a province for the batch merge/split "affect only these"
          // set; a plain click selects it (and opens its dossier).
          if (e.shiftKey || useUIStore.getState().provinceSelectMode) useUIStore.getState().toggleMarkedProvince(pid);
          else useUIStore.getState().setSelectedProvince(pid);
          return;
        }
      }
    }

    if (activeToolRef.current === "pan" || e.button === 1) {
      viewport.startPan(e.clientX, e.clientY);
      return;
    }

    if (activeToolRef.current === "select") {
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
        if (wx >= 0 && wx < m.grid_width && wy >= 0 && wy < m.grid_height) {
          setInspectedCell({ wx, wy });
        }
      }
      return;
    }

    const tool = activeToolRef.current;
    // A finalized (locked) map is read-only — geography edits are disabled in the
    // UI (the backend also rejects them via ensure_unfrozen).
    if (m.frozen && (tool === "paint" || tool === "elevation" || tool === "shelf" || tool === "volcano" || tool === "ridge" || tool === "lasso" || tool === "provinceBrush")) {
      return;
    }
    if (tool === "paint" || tool === "elevation" || tool === "shelf" || tool === "provinceBrush") {
      isPaintingRef.current = true;
      isErasingRef.current = e.shiftKey;
      pendingCellsRef.current.clear();
      clearPaintOverlay();
      applyBrush(e);
    } else if (tool === "volcano") {
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
        if (wx >= 0 && wx < m.grid_width && wy >= 0 && wy < m.grid_height) {
          const wrappedX = ((wx % m.grid_width) + m.grid_width) % m.grid_width;
          const val: PaintValue = { type: "volcanic", value: e.shiftKey ? 0 : 1 };
          paintStroke([[wrappedX, wy]], val).then((modified) => {
            tileManagerRef.current?.invalidate(modified);
            refreshTiles();
          }).catch(console.error);
        }
      }
    } else if (tool === "placeSettlement") {
      // GENERATION_UX_REDESIGN_PLAN.md Slice 7 (F8) — click, the world decides.
      // Read-only on the backend (no tile write, no save); the derived
      // Settlement is added straight to the store, exactly like a generated
      // batch, so trade routes pick it up the moment it's added (F8's own
      // "aliveness" claim — MapCanvas's route effect already lists
      // `settlements` in its dependency array).
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
        if (wx >= 0 && wx < m.grid_width && wy >= 0 && wy < m.grid_height) {
          const wrappedX = Math.floor(((wx % m.grid_width) + m.grid_width) % m.grid_width);
          const wrappedY = Math.floor(wy);
          const currentRivers = useWorldStore.getState().rivers;
          const currentSettlements = useWorldStore.getState().settlements;
          // PLACES_DEMAND_AND_GROWTH_PLAN.md slice 3b (D3) — the explicit
          // "province capital" toggle, read at click time.
          const isCapital = useUIStore.getState().placeSettlementIsCapital;
          placeSettlementAt(wrappedX, wrappedY, JSON.stringify(currentRivers), currentSettlements, isCapital)
            .then((s) => {
              useWorldStore.getState().setSettlements([...currentSettlements, s]);
              setStatus(`Placed ${s.name} (${s.size}, ~${s.population.toLocaleString()})`
                + (isCapital ? " as a province capital" : "") + " — the world's own figure.");
              requestRender();
            })
            .catch((err) => setStatus(`Cannot place a settlement there: ${err}`));
        }
      }
    } else if (tool === "plateSeed") {
      // GENERATION_UX_REDESIGN_PLAN.md Slice 6 — click to drop a plate seed
      // (centre + the current class/type draft), shift-click the nearest one
      // to delete it. Purely a store append/remove — the seeds are submitted
      // together when the panel's own Generate button is pressed.
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
        if (wx >= 0 && wx < m.grid_width && wy >= 0 && wy < m.grid_height) {
          const wrappedX = Math.floor(((wx % m.grid_width) + m.grid_width) % m.grid_width);
          const wrappedY = Math.floor(wy);
          const store = useUIStore.getState();
          if (e.shiftKey && store.plateSeeds.length > 0) {
            let best = -1; let bestD = Infinity;
            store.plateSeeds.forEach((s, i) => {
              const d = (s.x - wrappedX) ** 2 + (s.y - wrappedY) ** 2;
              if (d < bestD) { bestD = d; best = i; }
            });
            if (best >= 0) store.removePlateSeed(best);
          } else {
            store.addPlateSeed(wrappedX, wrappedY);
          }
        }
      }
    } else if (tool === "ridge") {
      // Start a new ridge polyline (Shift = erase/flatten). Points accumulate on
      // pointer-move and commit to the store on pointer-up; the in-progress line
      // is pushed straight onto the overlay so it draws live.
      const rect = containerRef.current?.getBoundingClientRect();
      if (!rect) return;
      const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
      isPaintingRef.current = true;
      isErasingRef.current = e.shiftKey;
      ridgeDraftRef.current = [[wx, wy]];
      const p = ridgeParamsRef.current;
      const draft = { points: [[wx, wy]] as [number, number][], width: p.width, height: p.height, character: p.character, noise: p.noise, erase: e.shiftKey };
      overlayManagerRef.current?.setRidgeSketch([...ridgeLinesRef.current, draft]);
      requestRender();
    } else if (tool === "lasso") {
      // Start a new freehand selection polygon. Points accumulate on
      // pointer-move and commit to the store on pointer-up.
      const rect = containerRef.current?.getBoundingClientRect();
      if (!rect) return;
      const { wx, wy } = viewport.screenToWorldWrapped(e.clientX - rect.left, e.clientY - rect.top);
      isPaintingRef.current = true;
      lassoDraftRef.current = [[wx, wy]];
      overlayManagerRef.current?.setLassoSketch(lassoDraftRef.current);
      requestRender();
    }
  }, [applyBrush, setInspectedCell, setSelectedHub, setSelectedGood, refreshTiles, requestRender]);

  // Hover-readout throttling state (see the block inside onPointerMove).
  const lastHoverCellRef = useRef("");
  const lastHoverAtRef = useRef(0);
  const hoverSeqRef = useRef(0);
  const setHoverInfo = useUIStore((s) => s.setHoverInfo);

  const onPointerMove = useCallback((e: React.PointerEvent) => {
    const viewport = viewportRef.current;
    const m = metaRef.current;
    if (!viewport || !m) return;

    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;

    const screenX = e.clientX - rect.left;
    const screenY = e.clientY - rect.top;
    const { wx, wy } = viewport.screenToWorldWrapped(screenX, screenY);

    // STATUSBAR HOVER READOUT (§8.18) — the map key for the layers that have no
    // legend. De-duplicated by CELL and time-throttled, so a fast sweep costs one
    // IPC every 60 ms rather than one per pixel; the sequence guard stops a slow
    // reply from overwriting a newer cell's answer.
    if (e.buttons === 0) {
      const cx = Math.floor(wx);
      const cy = Math.floor(wy);
      const key = `${cx},${cy}`;
      const now = performance.now();
      if (key !== lastHoverCellRef.current && now - lastHoverAtRef.current > 60) {
        lastHoverCellRef.current = key;
        lastHoverAtRef.current = now;
        const seq = ++hoverSeqRef.current;
        getCellInfo(cx, cy)
          .then((info) => { if (seq === hoverSeqRef.current) setHoverInfo(info); })
          .catch(() => { /* off-grid or mid-regeneration: leave the last reading */ });
      }
    }

    // Hover shine: highlight the settlement under the cursor so it's clear what a
    // click will select (inspection tools, not while painting/panning).
    const hoverTool = activeToolRef.current === "select" || activeToolRef.current === "pan";
    if (hoverTool && !isPaintingRef.current && e.buttons === 0) {
      // Hit-test the SAME unified list the map draws, so the shine lands on the dot
      // under the cursor (live hub OR sub-cap village), never a farther live hub.
      const cands = clickCandidatesRef.current;
      const thresh = Math.max(6, m.grid_width * 0.012);
      let best = -1; let bx = 0; let by = 0; let bestD = thresh * thresh;
      for (const h of cands) {
        let dx = Math.abs(h.x + 0.5 - wx);
        if (dx > m.grid_width / 2) dx = m.grid_width - dx;
        const dy = h.y + 0.5 - wy;
        const d = dx * dx + dy * dy;
        if (d < bestD) { bestD = d; best = h.id; bx = h.x; by = h.y; }
      }
      if (best !== hoveredHubRef.current) {
        hoveredHubRef.current = best;
        overlayManagerRef.current?.setHoverPoint(best >= 0 ? { x: bx, y: by } : null);
        if (containerRef.current) containerRef.current.style.cursor = best >= 0 ? "pointer" : "";
        requestRender();
      }
    }

    if (wx >= 0 && wx < m.grid_width && wy >= 0 && wy < m.grid_height) {
      const tool = activeToolRef.current;
      const erasing = e.shiftKey;
      const modeLabel = tool === "ridge"
        ? (erasing ? "Ridge erase (flatten)" : `Ridge ~${Math.round(ridgeParamsRef.current.height * 8848)}m, width ${ridgeParamsRef.current.width}`)
        : tool === "elevation"
        ? (erasing ? "Erase elevation" : `Elevation ${Math.round(elevationValueRef.current * 8848)}m`)
        : (erasing ? "Erase (sea)" : "Paint land");
      setStatus(`Cell: (${wx}, ${wy}) | ${modeLabel}`);
    }

    const tool = activeToolRef.current;
    if (tool === "paint" || tool === "elevation" || tool === "shelf" || tool === "provinceBrush") {
      if (!isPaintingRef.current) {
        drawCursorRing(screenX, screenY, getBrushScreenRadius(), getPaintMode(e.shiftKey));
      }
    } else {
      clearPaintOverlay();
    }

    if (activeToolRef.current === "pan" || e.buttons === 4) {
      viewport.updatePan(e.clientX, e.clientY);
      refreshTiles();
      requestRender();
      return;
    }

    if (isPaintingRef.current) {
      if (activeToolRef.current === "ridge") {
        // Extend the in-progress ridge line (throttle by ~1 cell of movement).
        const draft = ridgeDraftRef.current;
        const last = draft[draft.length - 1];
        if (!last || Math.abs(last[0] - wx) + Math.abs(last[1] - wy) >= 1) {
          draft.push([wx, wy]);
          const p = ridgeParamsRef.current;
          const line = { points: [...draft], width: p.width, height: p.height, character: p.character, noise: p.noise, erase: isErasingRef.current };
          overlayManagerRef.current?.setRidgeSketch([...ridgeLinesRef.current, line]);
          requestRender();
        }
      } else if (activeToolRef.current === "lasso") {
        // Extend the in-progress lasso polygon (throttle by ~1 cell of movement).
        const draft = lassoDraftRef.current;
        const last = draft[draft.length - 1];
        if (!last || Math.abs(last[0] - wx) + Math.abs(last[1] - wy) >= 1) {
          draft.push([wx, wy]);
          overlayManagerRef.current?.setLassoSketch([...draft]);
          requestRender();
        }
      } else {
        applyBrush(e);
      }
    }
  }, [applyBrush, refreshTiles, setStatus, getBrushScreenRadius, getPaintMode, requestRender]);

  const onPointerUp = useCallback(async (e?: React.PointerEvent) => {
    const viewport = viewportRef.current;
    if (viewport) viewport.endPan();
    clearPaintOverlay();

    // A click (not a drag) on the political map picks the province under it and opens
    // the realm window on it — its realm, its goods, its people.
    const down = politicalDownRef.current;
    politicalDownRef.current = null;
    if (down && e && Math.hypot(e.clientX - down.x, e.clientY - down.y) < 5) {
      const pid = realmLayerRef.current.provinceAt(down.wx, down.wy);
      const ui = useUIStore.getState();
      ui.setSelectedProvince(pid);
      if (pid !== null) { ui.setShowProvinceInspector(false); ui.setShowStates(true); ui.setRealmFocus(pid); }
    }

    // Ridge tool: commit the drawn polyline to the store (transient — one-shot).
    if (isPaintingRef.current && activeToolRef.current === "ridge") {
      isPaintingRef.current = false;
      const draft = ridgeDraftRef.current;
      ridgeDraftRef.current = [];
      if (draft.length >= 1) {
        const p = ridgeParamsRef.current;
        addRidgeLine({ points: draft, width: p.width, height: p.height, character: p.character, noise: p.noise, erase: isErasingRef.current });
      } else {
        overlayManagerRef.current?.setRidgeSketch(ridgeLinesRef.current);
        requestRender();
      }
      isErasingRef.current = false;
      return;
    }

    // Lasso tool: commit the drawn polygon to the store (transient — replaces
    // any previous lasso; the area-tools panel reads it and clears it after use).
    if (isPaintingRef.current && activeToolRef.current === "lasso") {
      isPaintingRef.current = false;
      const draft = lassoDraftRef.current;
      lassoDraftRef.current = [];
      if (draft.length >= 3) {
        setLassoPolygon(draft);
      } else {
        overlayManagerRef.current?.setLassoSketch([]);
        requestRender();
      }
      return;
    }

    if (isPaintingRef.current && pendingCellsRef.current.size > 0) {
      isPaintingRef.current = false;

      const gw = metaRef.current?.grid_width ?? 1;
      const cells: [number, number][] = [];
      for (const key of pendingCellsRef.current) {
        cells.push([key % gw, Math.floor(key / gw)]);
      }
      pendingCellsRef.current.clear();

      const erasing = isErasingRef.current;
      let value: PaintValue;
      const tool = activeToolRef.current;

      // Border edit: the painted land cells go to the province being edited.
      if (tool === "provinceBrush") {
        const target = useUIStore.getState().provinceEditTarget;
        isPaintingRef.current = false; isErasingRef.current = false;
        if (target !== null) {
          try {
            const res = await simPaintProvinceCells(target, cells);
            useWorldStore.getState().setProvinces(res.provinces, decodeProvinceRaster(res));
            setStatus("Border updated");
          } catch (err) { setStatus(`Border edit failed: ${err}`); }
        }
        return;
      }

      if (tool === "elevation") {
        value = { type: "elevation", value: erasing ? 0.0 : elevationValueRef.current };
      } else if (tool === "shelf") {
        value = { type: "shelf", value: erasing ? 0 : 1 };
      } else {
        value = { type: "terrain", value: erasing ? 0 : 1 };
      }

      try {
        const modifiedTiles = await paintStroke(cells, value);
        tileManagerRef.current?.invalidate(modifiedTiles);
        refreshTiles();
      } catch (err) {
        console.error("Paint failed:", err);
      }
    }

    isPaintingRef.current = false;
    isErasingRef.current = false;
  }, [refreshTiles, addRidgeLine, setLassoPolygon, requestRender]);

  const getCursor = (): string => {
    if (activeTool === "pan") return "grab";
    if (activeTool === "select") return "pointer";
    if (activeTool === "paint" || activeTool === "elevation" || activeTool === "shelf" || activeTool === "provinceBrush") return "none";
    if (activeTool === "ridge" || activeTool === "lasso") return "crosshair";
    return "crosshair";
  };

  return (
    <div
      ref={paneRef}
      style={{
        width: "100%", height: "100%",
        display: "flex", alignItems: "center", justifyContent: "center",
        background: "#15110b", overflow: "hidden",
      }}
    >
    <div
      ref={containerRef}
      style={{
        width: box ? `${box.w}px` : "100%",
        height: box ? `${box.h}px` : "100%",
        position: "relative", cursor: getCursor(),
      }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerLeave={() => { clearPaintOverlay(); onPointerUp(); }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {mapStyle === "game" && (
        // Game Board mist: the map's edges fade, like fog over unexplored land.
        <div aria-hidden style={{
          position: "absolute", inset: 0, pointerEvents: "none", zIndex: 5,
          background: "radial-gradient(ellipse at center, rgba(0,0,0,0) 58%, rgba(214,220,214,0.22) 84%, rgba(200,206,200,0.42) 100%)",
        }} />
      )}
      {compareLayer && (
        <div
          onPointerDown={(e) => {
            e.currentTarget.setPointerCapture(e.pointerId);
            e.stopPropagation();
          }}
          onPointerMove={(e) => {
            if (e.buttons === 0) return;
            const r = containerRef.current?.getBoundingClientRect();
            if (r) setComparePos((e.clientX - r.left) / Math.max(1, r.width));
            e.stopPropagation();
          }}
          onPointerUp={(e) => e.stopPropagation()}
          title={`Drag to compare — ${compareLayer} on the right`}
          style={{
            position: "absolute", top: 0, bottom: 0,
            left: `${comparePos * 100}%`, width: 14, marginLeft: -7,
            cursor: "ew-resize", zIndex: 20, touchAction: "none",
            display: "flex", alignItems: "center", justifyContent: "center",
          }}
        >
          <div style={{ position: "absolute", top: 0, bottom: 0, width: 2, background: "#cfe2f6", opacity: 0.85 }} />
          <div style={{
            width: 14, height: 34, borderRadius: 7, background: "#221c12",
            border: "1px solid #c09b5a", display: "flex", alignItems: "center",
            justifyContent: "center", color: "#dbc7a4", fontSize: 9, letterSpacing: -1,
          }}>◀▶</div>
        </div>
      )}

      {initError && (
        <div style={{
          position: "absolute", top: 10, left: 10, right: 10,
          background: "#3a1010", border: "1px solid #802020",
          borderRadius: 6, padding: 12, zIndex: 50, color: "#ff8080",
          fontSize: 12, fontFamily: "monospace", whiteSpace: "pre-wrap",
        }}>
          Init failed: {initError}
        </div>
      )}
      {initStatus !== "ready" && !initError && (
        <div style={{
          position: "absolute", top: "50%", left: "50%",
          transform: "translate(-50%, -50%)",
          color: "#796a51", fontSize: 13,
        }}>
          {initStatus}
        </div>
      )}
    </div>
    </div>
  );
}
