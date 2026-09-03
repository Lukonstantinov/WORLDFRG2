import { useEffect, useRef } from "react";
import { useGoodsStore } from "@state/goodsStore";
import { goodIconCanvas, type GoodTreatment } from "@canvas/goodIconCache";

export interface GoodIconProps {
  /** Backend good identifier (`GOOD_DEFS[i].name`). */
  name: string;
  /** Displayed square, CSS px. */
  size?: number;
  /** pixel = dense in-app panels · victorian = the ledger/trade screens. */
  treatment?: GoodTreatment;
  /** Override the good's own tint (defaults to the goods-store metadata). */
  color?: string;
  title?: string;
  style?: React.CSSProperties;
}

/** One trade good, drawn from its own illustration recipe. Replaces the emoji
 *  glyph in list views. Rasterised once per (good, size, treatment, tint) and
 *  blitted, so a list of every good costs 85 draws, not 85 re-renders. */
export function GoodIcon({ name, size = 22, treatment = "pixel", color, title, style }: GoodIconProps) {
  const meta = useGoodsStore((s) => s.meta);
  const tint = color ?? meta(name).color ?? "#cccccc";
  const ref = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const src = goodIconCanvas(name, tint, size, treatment);
    el.width = src.width; el.height = src.height;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, el.width, el.height);
    ctx.imageSmoothingEnabled = treatment === "victorian";
    ctx.drawImage(src, 0, 0);
  }, [name, tint, size, treatment]);

  return (
    <canvas ref={ref} title={title ?? name}
      style={{ width: size, height: size, flex: "0 0 auto", display: "block", ...style }} />
  );
}
