import { useEffect, useRef } from "react";
import { useGoodsStore } from "@state/goodsStore";
import { goodIconCanvas } from "@canvas/goodIconCache";

export interface GoodIconProps {
  /** Backend good identifier (`GOOD_DEFS[i].name`). */
  name: string;
  /** Displayed square, CSS px. 24/48 give a whole-pixel sprite at 1× DPI. */
  size?: number;
  /** Override the good's own tint (defaults to the goods-store metadata). */
  color?: string;
  title?: string;
  style?: React.CSSProperties;
}

/** One trade good as its 24×24 pixel sprite (`canvas/goodArt.ts`). An unknown
 *  or custom good draws a crate stencilled in its own tint. Backed at device
 *  resolution so the sprite's integer scaling lands on real pixels. */
export function GoodIcon({ name, size = 22, color, title, style }: GoodIconProps) {
  const meta = useGoodsStore((s) => s.meta);
  const tint = color ?? meta(name).color ?? "#cccccc";
  const ref = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const src = goodIconCanvas(name, tint, size, window.devicePixelRatio || 1);
    el.width = src.width; el.height = src.height;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, el.width, el.height);
    ctx.drawImage(src, 0, 0);
  }, [name, tint, size]);

  return (
    <canvas ref={ref} title={title ?? name}
      style={{ width: size, height: size, flex: "0 0 auto", display: "block", imageRendering: "pixelated", ...style }} />
  );
}
