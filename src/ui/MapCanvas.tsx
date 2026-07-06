import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { initPixiApp, type MapApp } from "../canvas/PixiApp";
import { TileViewport } from "../canvas/TileViewport";
import { TileManager } from "../canvas/TileManager";
import { OverlayManager, type ColonyMarker } from "../canvas/OverlayManager";
import { createPaintOverlay, drawCursorRing, paintStamp, clearPaintOverlay } from "../canvas/PaintOverlay";
import { useWorldStore } from "../state/worldStore";
import { useViewportStore } from "../state/viewportStore";
import { useUIStore } from "../state/uiStore";
import { useGoodsStore } from "../state/goodsStore";
import { useCampaignStore } from "../state/campaignStore";
import { useSettingsStore } from "../state/settingsStore";
import { paintStroke, undoAction, redoAction, computeOverlays, computeStormZones, computeMonsoonZones, computeCultureRegions, computeTradeRoutes, computeTradeMatrix, computePolitical, getEconomy, campaignMerchantRoutes, campaignFuturesLanes, campaignGetSpeculation, campaignGetTradeFlow, campaignCoinUsage, campaignGetBanks, campaignGetEpidemics, campaignGetGuilds, campaignGetFigures, campaignGetLandmarks, campaignGetDynasties, campaignGetTradeBasins, campaignGetGoodHeat, campaignGetCultures, campaignCultureHubs, campaignGetMigrationRoutes } from "../bridge/tauri";
import type { MerchantRoute, FuturesLane } from "../types";
import { goodOverlayKey, GOOD_DEFS } from "../goods";
import type { PaintValue, EconChain, Settlement, CampaignHubBrief } from "../types";

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
  const economy = useWorldStore((s) => s.economy);
  const setEconomy = useWorldStore((s) => s.setEconomy);
  const activeLayer = useUIStore((s) => s.activeLayer);
  const activeTool = useUIStore((s) => s.activeTool);
  const brushRadius = useUIStore((s) => s.brushRadius);
  const elevationValue = useUIStore((s) => s.elevationValue);
  const tileVersion = useViewportStore((s) => s.tileVersion);
  const focusTarget = useViewportStore((s) => s.focusTarget);
  const searchPin = useViewportStore((s) => s.searchPin);
  const overlayVisibility = useUIStore((s) => s.overlayVisibility);
  const lineColors = useSettingsStore((s) => s.lineColors);
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
  const futuresLanesRef = useRef<FuturesLane[]>([]);
  const selectedChain = useUIStore((s) => s.selectedChain);
  const selectedHub = useUIStore((s) => s.selectedHub);
  const migrationMode = useUIStore((s) => s.migrationMode);
  const openRoads = useUIStore((s) => s.openRoads);
  const selectedExport = useUIStore((s) => s.selectedExport);
  const setSelectedGood = useUIStore((s) => s.setSelectedGood);
  const hubDisplay = useUIStore((s) => s.hubDisplay);
  const reachGood = useUIStore((s) => s.reachGood);

  // Stable identity for the loaded world. Heavy effects (tile reloads + sim IPC)
  // key off this string rather than the `meta` object, so latitude-slider edits
  // — which no longer touch `meta` at all, but even a persisted change keeps the
  // same dimensions/name — don't retrigger tile-cache rebuilds or sim queries.
  const worldKey = meta ? `${meta.name}|${meta.grid_width}|${meta.grid_height}` : null;

  const metaRef = useRef(meta);
  metaRef.current = meta;
  const stretchToFitRef = useRef(stretchToFit);
  stretchToFitRef.current = stretchToFit;
  const activeLayerRef = useRef(activeLayer);
  activeLayerRef.current = activeLayer;
  const activeToolRef = useRef(activeTool);
  activeToolRef.current = activeTool;
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
  // Good regions kept for click hit-testing (click a good belt → good-flow panel).
  const goodRegionsRef = useRef<import("../types").GoodRegion[]>([]);
  const overlayVisibilityRef = useRef(overlayVisibility);
  overlayVisibilityRef.current = overlayVisibility;

  /** Mark canvas as needing a repaint */
  const requestRender = useCallback(() => {
    needsRenderRef.current = true;
  }, []);

  /** Render the map to the 2D canvas */
  const renderFrame = useCallback(() => {
    const mapApp = appRef.current;
    const viewport = viewportRef.current;
    const tileManager = tileManagerRef.current;
    const overlayManager = overlayManagerRef.current;
    if (!mapApp || !viewport || !tileManager) return;

    const { ctx, canvas } = mapApp;
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.width / dpr;
    const h = canvas.height / dpr;

    // Clear
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = "#080e18";
    ctx.fillRect(0, 0, w, h);

    // Debug: draw a visible border so we know the canvas renders
    ctx.strokeStyle = "#304060";
    ctx.lineWidth = 2;
    ctx.strokeRect(1, 1, w - 2, h - 2);

    // Apply viewport transform (independent X/Y scale so the world fills the
    // canvas with no letterbox bars).
    ctx.setTransform(dpr * viewport.scaleX, 0, 0, dpr * viewport.scaleY, dpr * viewport.x, dpr * viewport.y);
    ctx.imageSmoothingEnabled = false;

    // Clip to the logical world bounds. Tiles are 128×128, so the world grid
    // (e.g. 3600×1800) is covered by tiles that extend past the edges (29×15 →
    // 3712×1920); the last partial tile row/column is default-sea and otherwise
    // bleeds in as a thin ocean strip at the bottom/right edge. Clipping to the
    // exact grid hides that padding so the map ends precisely at the template.
    const m = metaRef.current;
    ctx.save();
    if (m) {
      ctx.beginPath();
      ctx.rect(0, 0, m.grid_width, m.grid_height);
      ctx.clip();
    }

    // Draw tiles (only those within the visible tile range).
    tileManager.draw(ctx, viewport.getVisibleTileRange(w, h));

    // Draw overlays
    if (overlayManager) {
      overlayManager.render(ctx);
    }

    ctx.restore();
    // Reset transform
    ctx.setTransform(1, 0, 0, 1, 0, 0);
  }, []);

  const refreshTiles = useCallback(() => {
    const viewport = viewportRef.current;
    const tileManager = tileManagerRef.current;
    const el = containerRef.current;
    const m = metaRef.current;
    if (!viewport || !tileManager || !el || !m) return;

    const { txMin, txMax, tyMin, tyMax } = viewport.getVisibleTileRange(
      el.clientWidth, el.clientHeight
    );

    // viewport.scale picks the LOD: zoomed out → coarser supertiles, so the
    // request size stays bounded no matter how much of the world is visible.
    tileManager.loadVisibleTiles(
      txMin, txMax, tyMin, tyMax,
      activeLayerRef.current, m.grid_width, m.grid_height,
      viewport.scale,
    ).then(() => requestRender());
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
        viewport.onWheel(e);
        overlayManager.updateScale(viewport.scale);
        refreshTiles();
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

      // Render loop
      const loop = () => {
        if (destroyed) return;
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
  }, [activeLayer, refreshTiles]);

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
    // Estates (ids >= 100000) are INTERNAL to their parent city — never drawn as
    // their own map dots.
    const hubs = campaignSnapshot?.active ? campaignSnapshot.hubs.filter((h) => h.id < 100000) : null;
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
  // vectors + streamlines, fishery banks, shark/shipworm/reef/storm danger zones
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
      om.drawSharkZones(o.shark_zones);
      om.drawShipwormZones(o.shipworm_zones);
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
    // Peoples / culture territories (organic hearth map) for the Peoples overlay.
    computeCultureRegions().then((regions) => {
      om.drawCultureRegions(regions);
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
      requestRender();
    }).catch(() => {});
  }, [overlayVisibility.merchantRoutes, campaignSnapshot?.active, campaignSnapshot?.clock.tick, meta, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

  // Trade ▸ Flows highlight — glowing arrows from a settlement to its partners,
  // set by the Flows subtab (purely frontend, redraws when the selection changes).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    om.setFlowHighlight(flowHighlight, meta.grid_width);
    requestRender();
  }, [flowHighlight, meta, requestRender]);

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
    campaignGetTradeFlow(rivers.map((r) => ({ points: r.points })), bioParams.tradeReach, bioParams.maxCrossing)
      .then((trunks) => {
        om.drawDynamicFlow(trunks, meta.grid_width);
        requestRender();
      }).catch(() => {});
  }, [overlayVisibility.dynamicFlow, campaignSnapshot?.active, Math.floor((campaignSnapshot?.clock.tick ?? 0) / 365), meta, rivers, bioParams.tradeReach, bioParams.maxCrossing, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

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
      om.setVisible(type, visible);
    }
    requestRender();
  }, [overlayVisibility, requestRender]);

  // Merchant-family control overlay — which settlements each house holds. Driven
  // by the live campaign houses (updates dynamically as the campaign advances).
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.drawHouseControl(houses, meta?.grid_width ?? 0, selectedHouseIdx);
    requestRender();
  }, [houses, meta, selectedHouseIdx, requestRender]);

  // Draw latitude lines. Driven by the live `latConfig` slice (not `meta`) so
  // dragging the Latitude Frame sliders repaints ONLY this overlay — no tile
  // reloads, no sim IPC.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    om.drawLatLines(meta.grid_width, meta.grid_height, latConfig.equatorOffset, latConfig.latScale, latConfig.lineRatio);
    requestRender();
  }, [worldKey, latConfig, requestRender]); // eslint-disable-line react-hooks/exhaustive-deps

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

  // Coin-usage overlay: when a coin is selected in the Currencies panel, fetch the
  // per-city settlement snapshot and tint the map by use of that coin.
  const coinOverlayHub = useUIStore((s) => s.coinOverlayHub);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    if (coinOverlayHub == null || !campaignSnapshot?.active) {
      om.setCoinUsage([], null); requestRender(); return;
    }
    let alive = true;
    campaignCoinUsage()
      .then((u) => { if (alive) { om.setCoinUsage(u, coinOverlayHub); requestRender(); } })
      .catch(() => {});
    return () => { alive = false; };
  }, [coinOverlayHub, campaignSnapshot?.active, campaignSnapshot?.clock.tick, requestRender]);

  // #23 · the chosen itinerary route, pushed from the Itinerary panel via uiStore.
  const travelRoute = useUIStore((s) => s.travelRoute);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.drawTravelRoute(travelRoute ?? []);
    requestRender();
  }, [travelRoute, requestRender]);

  // 🌊 Hydrology · the selected river system's subtree glows on the map.
  const riverHighlight = useUIStore((s) => s.riverHighlight);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.setRiverHighlight(riverHighlight);
    requestRender();
  }, [riverHighlight, requestRender]);

  // #26 · geographic toponym labels, pushed from the world store.
  const toponyms = useWorldStore((s) => s.toponyms);
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.drawToponyms(toponyms);
    requestRender();
  }, [toponyms, requestRender]);

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
    const emoji: Record<string, string> = Object.fromEntries(GOOD_DEFS.map((g) => [g.name, g.emoji]));
    campaignGetGuilds().then((guilds) => {
      if (!alive) return;
      om.setGuilds(guilds.map((g) => ({ x: g.x, y: g.y, emoji: emoji[g.good_name] ?? "🏭", label: g.exceptional ? g.brand : "" })));
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

    const { wx, wy } = viewport.screenToWorld(e.clientX - rect.left, e.clientY - rect.top);
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

    if (e.button === 2 || e.altKey) {
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const { wx, wy } = viewport.screenToWorld(e.clientX - rect.left, e.clientY - rect.top);
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
          const { wx, wy } = viewport.screenToWorld(e.clientX - rect.left, e.clientY - rect.top);
          const thresh = Math.max(6, m.grid_width * 0.012);
          // A bank icon takes click priority when bank icons are shown → open the Bank panel.
          if (useUIStore.getState().showBankIcons) {
            const bi = overlayManagerRef.current?.bankIconAt(wx, wy, thresh) ?? -1;
            if (bi >= 0) {
              useUIStore.getState().setSelectedBankIdx(bi);
              useUIStore.getState().setShowBank(true);
              return;
            }
          }
          // Hit-test the LIVE campaign hubs when a campaign is running (they include the
          // in-campaign colonies/outposts the frozen economy snapshot lacks); otherwise
          // the economy hubs. A click on a colony/outpost also opens the Colonial Office.
          const snap = campaignSnapshotRef.current;
          const hubList: { id: number; x: number; y: number; colony_kind?: number; is_estate?: boolean }[] =
            snap?.active ? snap.hubs.filter((h) => !h.is_estate) : econ.hubs;
          let best = -1; let bestD = thresh * thresh; let bestColony = 0;
          for (const h of hubList) {
            // Markers are DRAWN at the cell CENTRE (h.x+0.5), so hit-test the centre
            // too — comparing against the cell corner offset every click half a cell
            // up-left ("clicking slightly left").
            let dx = Math.abs(h.x + 0.5 - wx);
            if (dx > m.grid_width / 2) dx = m.grid_width - dx;
            const dy = h.y + 0.5 - wy;
            const d = dx * dx + dy * dy;
            if (d < bestD) { bestD = d; best = h.id; bestColony = h.colony_kind ?? 0; }
          }
          // Fallback: the campaign sim only keeps the strongest ~250 settlements as
          // LIVE hubs, but the map still draws EVERY worldgen settlement. Those extra
          // dots were pure visuals — unclickable — because the hit-test scanned only
          // the live hubs. Also scan the full frozen `econ.hubs` so every visible
          // settlement is clickable; HubPanel renders their worldgen data even without
          // a live campaign detail. Live hubs keep priority (checked first / smaller D).
          if (best < 0 && snap?.active) {
            for (const h of econ.hubs) {
              let dx = Math.abs(h.x + 0.5 - wx);
              if (dx > m.grid_width / 2) dx = m.grid_width - dx;
              const dy = h.y + 0.5 - wy;
              const d = dx * dx + dy * dy;
              if (d < bestD) { bestD = d; best = h.id; bestColony = 0; }
            }
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
            if (route) { setSelectedMerchantRoute(route); return; }
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
        const { wx, wy } = viewport.screenToWorld(e.clientX - rect.left, e.clientY - rect.top);
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
    }

    if (activeToolRef.current === "pan" || e.button === 1) {
      viewport.startPan(e.clientX, e.clientY);
      return;
    }

    if (activeToolRef.current === "select") {
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const { wx, wy } = viewport.screenToWorld(e.clientX - rect.left, e.clientY - rect.top);
        if (wx >= 0 && wx < m.grid_width && wy >= 0 && wy < m.grid_height) {
          setInspectedCell({ wx, wy });
        }
      }
      return;
    }

    const tool = activeToolRef.current;
    // A finalized (locked) map is read-only — geography edits are disabled in the
    // UI (the backend also rejects them via ensure_unfrozen).
    if (m.frozen && (tool === "paint" || tool === "elevation" || tool === "shelf" || tool === "volcano")) {
      return;
    }
    if (tool === "paint" || tool === "elevation" || tool === "shelf") {
      isPaintingRef.current = true;
      isErasingRef.current = e.shiftKey;
      pendingCellsRef.current.clear();
      clearPaintOverlay();
      applyBrush(e);
    } else if (tool === "volcano") {
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const { wx, wy } = viewport.screenToWorld(e.clientX - rect.left, e.clientY - rect.top);
        if (wx >= 0 && wx < m.grid_width && wy >= 0 && wy < m.grid_height) {
          const wrappedX = ((wx % m.grid_width) + m.grid_width) % m.grid_width;
          const val: PaintValue = { type: "volcanic", value: e.shiftKey ? 0 : 1 };
          paintStroke([[wrappedX, wy]], val).then((modified) => {
            tileManagerRef.current?.invalidate(modified);
            refreshTiles();
          }).catch(console.error);
        }
      }
    }
  }, [applyBrush, setInspectedCell, setSelectedHub, setSelectedGood, refreshTiles]);

  const onPointerMove = useCallback((e: React.PointerEvent) => {
    const viewport = viewportRef.current;
    const m = metaRef.current;
    if (!viewport || !m) return;

    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;

    const screenX = e.clientX - rect.left;
    const screenY = e.clientY - rect.top;
    const { wx, wy } = viewport.screenToWorld(screenX, screenY);

    // Hover shine: highlight the settlement under the cursor so it's clear what a
    // click will select (inspection tools, not while painting/panning).
    const hoverTool = activeToolRef.current === "select" || activeToolRef.current === "pan";
    if (hoverTool && !isPaintingRef.current && e.buttons === 0) {
      const snap = campaignSnapshotRef.current;
      const econ = economyRef.current;
      const hubList = snap?.active ? snap.hubs.filter((h) => !h.is_estate) : (econ?.hubs ?? []);
      const thresh = Math.max(6, m.grid_width * 0.012);
      let best = -1; let bx = 0; let by = 0; let bestD = thresh * thresh;
      for (const h of hubList) {
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
      const modeLabel = tool === "elevation"
        ? (erasing ? "Erase elevation" : `Elevation ${Math.round(elevationValueRef.current * 8848)}m`)
        : (erasing ? "Erase (sea)" : "Paint land");
      setStatus(`Cell: (${wx}, ${wy}) | ${modeLabel}`);
    }

    const tool = activeToolRef.current;
    if (tool === "paint" || tool === "elevation" || tool === "shelf") {
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
      applyBrush(e);
    }
  }, [applyBrush, refreshTiles, setStatus, getBrushScreenRadius, getPaintMode, requestRender]);

  const onPointerUp = useCallback(async () => {
    const viewport = viewportRef.current;
    if (viewport) viewport.endPan();
    clearPaintOverlay();

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
  }, [refreshTiles]);

  const getCursor = (): string => {
    if (activeTool === "pan") return "grab";
    if (activeTool === "select") return "pointer";
    if (activeTool === "paint" || activeTool === "elevation" || activeTool === "shelf") return "none";
    return "crosshair";
  };

  return (
    <div
      ref={paneRef}
      style={{
        width: "100%", height: "100%",
        display: "flex", alignItems: "center", justifyContent: "center",
        background: "#080e18", overflow: "hidden",
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
          color: "#4a6080", fontSize: 13,
        }}>
          {initStatus}
        </div>
      )}
    </div>
    </div>
  );
}
