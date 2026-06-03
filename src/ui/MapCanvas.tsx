import { useState, useEffect, useRef, useCallback } from "react";
import { initPixiApp, type MapApp } from "../canvas/PixiApp";
import { TileViewport } from "../canvas/TileViewport";
import { TileManager } from "../canvas/TileManager";
import { OverlayManager } from "../canvas/OverlayManager";
import { createPaintOverlay, drawCursorRing, paintStamp, clearPaintOverlay } from "../canvas/PaintOverlay";
import { useWorldStore } from "../state/worldStore";
import { useViewportStore } from "../state/viewportStore";
import { useUIStore } from "../state/uiStore";
import { useGoodsStore } from "../state/goodsStore";
import { paintStroke, undoAction, redoAction, getOverlayVectors, getCurrentStreamlines, computeTradeRoutes, computeFisheryBanks, computeSharkZones, computeShipwormZones, computeStormZones, computeReefZones, computeGoodRegions, computeTradeMatrix, computePolitical } from "../bridge/tauri";
import type { PaintValue } from "../types";

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
  const isErasingRef = useRef(false);
  const pendingCellsRef = useRef<Set<string>>(new Set());
  const initRef = useRef(false);
  const rafRef = useRef(0);
  const needsRenderRef = useRef(true);
  const [initError, setInitError] = useState<string | null>(null);
  const [initStatus, setInitStatus] = useState("waiting");

  const meta = useWorldStore((s) => s.meta);
  const rivers = useWorldStore((s) => s.rivers);
  const lakes = useWorldStore((s) => s.lakes);
  const settlements = useWorldStore((s) => s.settlements);
  const activeLayer = useUIStore((s) => s.activeLayer);
  const activeTool = useUIStore((s) => s.activeTool);
  const brushRadius = useUIStore((s) => s.brushRadius);
  const elevationValue = useUIStore((s) => s.elevationValue);
  const tileVersion = useViewportStore((s) => s.tileVersion);
  const focusTarget = useViewportStore((s) => s.focusTarget);
  const overlayVisibility = useUIStore((s) => s.overlayVisibility);
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
  const elevationValueRef = useRef(elevationValue);
  elevationValueRef.current = elevationValue;

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

    // Draw tiles
    tileManager.draw(ctx, viewport.x, viewport.y, viewport.scale);

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

    tileManager.loadVisibleTiles(
      txMin, txMax, tyMin, tyMax,
      activeLayerRef.current, m.grid_width, m.grid_height
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
      const resizeObserver = new ResizeObserver((entries) => {
        const entry = entries[0];
        if (entry) {
          const { width, height } = entry.contentRect;
          if (width > 0 && height > 0) {
            const dpr = window.devicePixelRatio || 1;
            mapApp.canvas.width = width * dpr;
            mapApp.canvas.height = height * dpr;
            const m2 = metaRef.current;
            if (m2) viewport.fitWorld(m2.grid_width, m2.grid_height, width, height, stretchToFitRef.current);
            refreshTiles();
            requestRender();
          }
        }
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

  // Reload tiles when layer, world, or version changes
  useEffect(() => {
    const tm = tileManagerRef.current;
    if (tm) tm.clear();
    refreshTiles();
  }, [activeLayer, meta, tileVersion, refreshTiles]);

  // Recompute the aspect-locked canvas box when the world (aspect) changes, so
  // a freshly imported template of a different shape re-proportions the canvas.
  useEffect(() => {
    const pane = paneRef.current;
    if (!pane || !meta) return;
    setBox(fitBox(pane.clientWidth, pane.clientHeight, meta.grid_width, meta.grid_height));
  }, [meta]);

  // Fit world when meta changes or the stretch-to-fit option is toggled.
  useEffect(() => {
    const viewport = viewportRef.current;
    const el = containerRef.current;
    if (!viewport || !el || !meta) return;
    viewport.fitWorld(meta.grid_width, meta.grid_height, el.clientWidth, el.clientHeight, stretchToFit);
    const tm = tileManagerRef.current;
    if (tm) tm.clear();
    refreshTiles();
    requestRender();
  }, [meta, stretchToFit, box, refreshTiles, requestRender]);

  // Redraw overlays when data changes
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    om.drawRivers(rivers);
    om.drawLakes(lakes);
    om.drawSettlements(settlements);
    requestRender();
  }, [rivers, lakes, settlements, requestRender]);

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
    ).then((routes) => {
      om.drawTradeRoutes(routes);
      requestRender();
    }).catch(() => {});
  }, [step8Done, settlements, rivers, tileVersion, bioParams.tradeReach, bioParams.maxCrossing, requestRender]);

  // Compute fishery grand-bank zones whenever the fishery data changes.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    computeFisheryBanks().then((banks) => {
      om.drawFisheryBanks(banks);
      requestRender();
    }).catch(() => {});
  }, [meta, tileVersion, requestRender]);

  // Compute shark + shipworm danger zones + trade-good belt regions when biology changes.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    computeSharkZones().then((zones) => {
      om.drawSharkZones(zones);
      requestRender();
    }).catch(() => {});
    computeShipwormZones().then((zones) => {
      om.drawShipwormZones(zones);
      requestRender();
    }).catch(() => {});
    computeReefZones().then((zones) => {
      om.drawReefZones(zones);
      requestRender();
    }).catch(() => {});
    computeGoodRegions().then((regions) => {
      om.drawGoodRegions(regions);
      requestRender();
    }).catch(() => {});
  }, [meta, tileVersion, requestRender]);

  // Storm zones depend on the seasonal month slider, so recompute them on their
  // own when the month (or calendar length) changes — month 0 = combined annual.
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    computeStormZones(stormMonth, calendarMonths).then((zones) => {
      om.drawStormZones(zones);
      requestRender();
    }).catch(() => {});
  }, [meta, tileVersion, stormMonth, calendarMonths, requestRender]);

  // Load the world's editable good specs (default 30 or custom) so overlays/labels
  // use the right icons/colors, including any custom goods.
  useEffect(() => { if (meta) void loadGoodsFromWorld(); }, [meta, tileVersion, loadGoodsFromWorld]);

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
    ).then((matrix) => {
      om.drawTradeTrunks(matrix.trunks, meta.grid_width);
      requestRender();
    }).catch(() => {});
  }, [step8Done, meta, settlements, rivers, tileVersion, bioParams.tradeReach, bioParams.maxCrossing, requestRender]);

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
    ).then((centers) => {
      om.drawPolitical(centers);
      requestRender();
    }).catch(() => {});
  }, [step9Done, meta, settlements, rivers, tileVersion, bioParams.tradeReach, bioParams.maxCrossing, requestRender]);

  // Sync overlay visibility
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om) return;
    for (const [type, visible] of Object.entries(overlayVisibility)) {
      om.setVisible(type, visible);
    }
    requestRender();
  }, [overlayVisibility, requestRender]);

  // Draw latitude lines
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    om.drawLatLines(meta.grid_width, meta.grid_height);
    requestRender();
  }, [meta, requestRender]);

  // Fetch wind/current vectors
  useEffect(() => {
    const om = overlayManagerRef.current;
    if (!om || !meta) return;
    getOverlayVectors().then((data) => {
      om.drawWindArrows(data.wind, meta.grid_width, meta.grid_height);
      requestRender();
    }).catch(() => {});
    getCurrentStreamlines().then((lines) => {
      om.drawCurrentStreamlines(lines);
      requestRender();
    }).catch(() => {});
  }, [meta, tileVersion, requestRender]);

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
        pendingCellsRef.current.add(`${wrappedX},${cy}`);
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
  }, [applyBrush, setInspectedCell, refreshTiles]);

  const onPointerMove = useCallback((e: React.PointerEvent) => {
    const viewport = viewportRef.current;
    const m = metaRef.current;
    if (!viewport || !m) return;

    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;

    const screenX = e.clientX - rect.left;
    const screenY = e.clientY - rect.top;
    const { wx, wy } = viewport.screenToWorld(screenX, screenY);

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

      const cells: [number, number][] = [];
      for (const key of pendingCellsRef.current) {
        const [x, y] = key.split(",").map(Number);
        cells.push([x, y]);
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
