import { useRef, useState, useCallback } from "react";

/** A distinct, faint tint per floating window so the different sub-panels are easy
 *  to tell apart at a glance (they all share the same dark base otherwise). Each is
 *  a near-opaque dark colour with a subtle hue so text stays readable. Mirrored in
 *  docs/mockups/floating-window-tints.html. */
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
} as const;

/** Make a floating panel DRAGGABLE by its header and TINTED so it's distinct.
 *
 *  Usage:
 *    const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.coin);
 *    <div data-draggable style={{ ...panel, ...rootStyle }}>
 *      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>…
 *        <span data-no-drag onClick={close}>✕</span>
 *
 *  Until the first drag the panel keeps its CSS position; the first drag seeds from
 *  the live bounding box so it doesn't jump. Children marked `data-no-drag`
 *  (close buttons, tabs) don't start a drag. */
export function useFloatingWindow(tint: string) {
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);
  const drag = useRef<{ sx: number; sy: number; ox: number; oy: number } | null>(null);

  const onPointerDown = useCallback((e: React.PointerEvent<HTMLElement>) => {
    if ((e.target as HTMLElement).closest("[data-no-drag]")) return;
    const root = (e.currentTarget as HTMLElement).closest("[data-draggable]") as HTMLElement | null;
    const r = root?.getBoundingClientRect();
    drag.current = {
      sx: e.clientX, sy: e.clientY,
      ox: pos?.x ?? r?.left ?? 0, oy: pos?.y ?? r?.top ?? 0,
    };
    const move = (ev: PointerEvent) => {
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
    e.preventDefault();
  }, [pos]);

  const rootStyle: React.CSSProperties = {
    background: tint,
    ...(pos ? { left: pos.x, top: pos.y, right: "auto", bottom: "auto" } : {}),
  };
  return { rootStyle, onPointerDown };
}
