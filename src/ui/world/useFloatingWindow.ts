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
} as const;

/** Anything that must keep its own pointer behaviour — a drag started on one of these
 *  is a CLICK on the control, never a window move. Sliders/scrollers matter most: a
 *  range input is dragged horizontally, which is exactly the gesture that would
 *  otherwise carry the whole window off. `[data-no-drag]` stays the manual escape
 *  hatch for anything else (a custom chart the panel wants to keep interactive). */
const INTERACTIVE =
  "button,a,input,textarea,select,option,label,summary," +
  "[data-no-drag],[contenteditable=''],[contenteditable='true'],[role='button'],[role='tab'],[role='slider']";

/** Pixels the pointer must travel before a press turns into a window drag. Below it
 *  the gesture is still a plain click, so drag-anywhere never eats a button press on
 *  a row, tab or list item that isn't one of the `INTERACTIVE` tags. */
const DRAG_THRESHOLD = 4;

/** Make a floating panel DRAGGABLE FROM ANYWHERE and TINTED so it's distinct.
 *
 *  Usage:
 *    const { rootStyle, onPointerDown, dragRoot } = useFloatingWindow(PANEL_TINTS.coin);
 *    <div data-draggable {...dragRoot} style={{ ...panel, ...rootStyle }}>
 *      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>…
 *        <span data-no-drag onClick={close}>✕</span>
 *
 *  `dragRoot` goes on the window ROOT: pressing anywhere in the window body starts a
 *  drag once the pointer has moved `DRAG_THRESHOLD` px, so a press that doesn't move
 *  is still an ordinary click. Controls matching `INTERACTIVE` (and anything marked
 *  `data-no-drag`) are excluded, and text selection inside the panel is off — that is
 *  the trade made for being able to grab a window by any part of it.
 *
 *  `onPointerDown` is the HEADER handler and is unchanged: the title bar drags
 *  immediately, with no threshold. Until the first drag the panel keeps its CSS
 *  position; the first drag seeds from the live bounding box so it doesn't jump. */
export function useFloatingWindow(tint: string) {
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);
  const drag = useRef<{ sx: number; sy: number; ox: number; oy: number } | null>(null);

  /** Shared drag start. `threshold` px of travel are required before the window
   *  actually moves (0 for the header, which drags on contact). */
  const begin = useCallback((e: React.PointerEvent<HTMLElement>, threshold: number) => {
    const target = e.target as HTMLElement;
    if (target.closest(INTERACTIVE)) return;
    const root = (e.currentTarget as HTMLElement).closest("[data-draggable]") as HTMLElement | null;
    const r = root?.getBoundingClientRect();
    const start = {
      sx: e.clientX, sy: e.clientY,
      ox: pos?.x ?? r?.left ?? 0, oy: pos?.y ?? r?.top ?? 0,
    };
    let armed = threshold <= 0;
    if (armed) drag.current = start;
    const move = (ev: PointerEvent) => {
      if (!armed) {
        const dx = ev.clientX - start.sx;
        const dy = ev.clientY - start.sy;
        if (dx * dx + dy * dy < threshold * threshold) return;
        armed = true;
        drag.current = start;
      }
      if (!drag.current) return;
      setPos({
        x: Math.max(0, drag.current.ox + ev.clientX - drag.current.sx),
        y: Math.max(0, drag.current.oy + ev.clientY - drag.current.sy),
      });
    };
    const up = () => {
      drag.current = null;
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    // Only the header suppresses the default immediately; the body must not, or the
    // press would never reach the row/tab the user was actually clicking.
    if (threshold <= 0) e.preventDefault();
  }, [pos]);

  const onPointerDown = useCallback(
    (e: React.PointerEvent<HTMLElement>) => begin(e, 0), [begin]);
  const onRootPointerDown = useCallback(
    (e: React.PointerEvent<HTMLElement>) => begin(e, DRAG_THRESHOLD), [begin]);

  const rootStyle: React.CSSProperties = {
    background: tint,
    // Drag-anywhere and text selection are mutually exclusive: a selection drag would
    // otherwise paint a highlight under the moving window.
    userSelect: "none",
    WebkitUserSelect: "none",
    ...(pos ? { left: pos.x, top: pos.y, right: "auto", bottom: "auto" } : {}),
  };
  /** Spread onto the window root to make the whole surface a drag handle. */
  const dragRoot = { onPointerDown: onRootPointerDown };
  return { rootStyle, onPointerDown, onRootPointerDown, dragRoot };
}
