import { useRef, useState, useCallback } from "react";

/** A distinct, faint tint per floating window so the different sub-panels are easy
 *  to tell apart at a glance (they all share the same dark base otherwise). Each is
 *  a near-opaque dark colour with a subtle hue so text stays readable. Mirrored in
 *  docs/mockups/_archive/floating-window-tints.html. */
export const PANEL_TINTS = {
  settlement: "rgba(12,18,26,0.97)", // 🏙 Settlement window — neutral slate (base)
  house: "rgba(28,20,12,0.97)",     // ⚜️ House detail popup — bronze
  coin: "rgba(18,24,14,0.97)",      // 🪙 Coin/Credit — olive-gold
  speculation: "rgba(24,16,28,0.97)", // 📈 Speculation/Poleis — plum
  houses: "rgba(26,18,16,0.97)",    // 🏛 Houses — sienna
  goods: "rgba(14,24,24,0.97)",     // 📦 Goods market — teal
  trade: "rgba(14,22,30,0.97)",     // 🔀 Trade matrix — steel-blue
  ranking: "rgba(24,22,12,0.97)",   // 🏆 City ranking — amber
  warehouses: "rgba(16,22,16,0.97)", // 🏚 Warehouses — moss
  futures: "rgba(22,14,22,0.97)",   // 📜 Futures — mauve
  news: "rgba(12,20,28,0.97)",      // 📰 News feed — slate
  goodflow: "rgba(14,26,20,0.97)",  // 🌊 Good flow — sea-green
  goodbrowser: "rgba(20,18,26,0.97)", // 📒 Goods browser — indigo
  route: "rgba(26,20,14,0.97)",     // 🐫 Merchant route — tan
  bank: "rgba(16,20,28,0.97)",      // 🏦 Bank — deep blue-steel
  plague: "rgba(26,14,14,0.97)",    // Plagues & Epidemics — sickly red
  guild: "rgba(24,20,12,0.97)",     // Guilds & Crafts — guild gold
  figures: "rgba(20,16,26,0.97)",   // Notable Figures — royal violet
  landmarks: "rgba(14,22,22,0.97)", // Landmarks & Sacred Sites — verdigris
  dynasties: "rgba(26,18,20,0.97)", // Dynasties & Alliances — claret
  atlas: "rgba(12,19,27,0.97)",     // 🗺 World Atlas — deep chart-blue
  hydrology: "rgba(10,22,30,0.97)", // 🌊 Hydrology — deep river-teal
  immigration: "rgba(14,20,26,0.97)", // 🧭 Migration & Immigration — dusk indigo
  province: "rgba(16,22,18,0.97)",  // 🏞 Province Inspector — moss-slate
  war: "rgba(26,15,13,0.97)",       // ⚔ War Council — iron-crimson
} as const;

/** Interactive descendants that must NEVER start a window drag — a press on any of
 *  these should do its own thing (click, type, scroll a slider). Everything else in
 *  the window body is fair game to grab, so the panel drags from anywhere. */
const NO_DRAG_SEL =
  "[data-no-drag], button, a, input, select, textarea, label, [role='button'], [contenteditable='true']";

/** Marker set on the native event once a drag has been claimed, so having
 *  `onPointerDown` on BOTH the root and the header (they share the bubbling event)
 *  starts exactly one drag, not two. */
type DragEvt = PointerEvent & { __wfDragClaimed?: boolean };

/** Move this far (px, Manhattan) before a press becomes a DRAG. Under the threshold
 *  the gesture is left alone, so plain clicks, row selection and text selection in the
 *  body still work even though the whole window is draggable. */
const DRAG_THRESHOLD = 4;

/** Make a floating panel DRAGGABLE FROM ANYWHERE on its body and TINTED so it's
 *  distinct.
 *
 *  Usage — put `onPointerDown` on the ROOT (`data-draggable`) node, so a press
 *  anywhere in the window can move it; the title bar keeps a `cursor: move` hint:
 *    const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.coin);
 *    <div data-draggable style={{ ...panel, ...rootStyle }} onPointerDown={onPointerDown}>
 *      <div style={{ ...header, cursor: "move" }}>…<span data-no-drag onClick={close}>✕</span>
 *
 *  Keeping `onPointerDown` on the header too is harmless (the per-event claim guard
 *  dedupes). Presses on interactive controls (`NO_DRAG_SEL`) never start a drag, and
 *  a small movement threshold preserves clicks / selection. Until the first drag the
 *  panel keeps its CSS position; the first drag seeds from the live bounding box so
 *  it doesn't jump. */
export function useFloatingWindow(tint: string) {
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);
  const drag = useRef<{ sx: number; sy: number; ox: number; oy: number } | null>(null);

  const onPointerDown = useCallback((e: React.PointerEvent<HTMLElement>) => {
    const ne = e.nativeEvent as DragEvt;
    if (ne.__wfDragClaimed) return;                       // already handled this event
    if ((e.target as HTMLElement).closest(NO_DRAG_SEL)) return;
    if (e.button !== 0) return;                            // primary button only
    ne.__wfDragClaimed = true;
    const root = (e.currentTarget as HTMLElement).closest("[data-draggable]") as HTMLElement | null;
    const r = root?.getBoundingClientRect();
    drag.current = {
      sx: e.clientX, sy: e.clientY,
      ox: pos?.x ?? r?.left ?? 0, oy: pos?.y ?? r?.top ?? 0,
    };
    let dragging = false;
    const move = (ev: PointerEvent) => {
      const d = drag.current;
      if (!d) return;
      const dx = ev.clientX - d.sx, dy = ev.clientY - d.sy;
      if (!dragging) {
        if (Math.abs(dx) + Math.abs(dy) < DRAG_THRESHOLD) return; // stay a click until moved
        dragging = true;
        document.body.style.userSelect = "none";           // don't select text while dragging
      }
      ev.preventDefault();
      setPos({ x: Math.max(0, d.ox + dx), y: Math.max(0, d.oy + dy) });
    };
    const up = () => {
      drag.current = null;
      document.body.style.userSelect = "";
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }, [pos]);

  const rootStyle: React.CSSProperties = {
    background: tint,
    ...(pos ? { left: pos.x, top: pos.y, right: "auto", bottom: "auto" } : {}),
  };
  return { rootStyle, onPointerDown };
}
