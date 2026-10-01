import { useEffect, useRef } from "react";
import { drawBust, drawFigure, type Person, type Occasion } from "@ui/campaign/cultureDress";

/** A pixel-art bust of one `Person` (face genome), drawn at 2× device scale. */
export function PixelBust({ person, size, occasion = "national", cols, bg, style }: {
  person: Person; size: number; occasion?: Occasion; cols?: number; bg?: string; style?: React.CSSProperties;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const el = ref.current; if (!el) return;
    const d = 2; el.width = size * d; el.height = size * d;
    const ctx = el.getContext("2d"); if (!ctx) return;
    ctx.clearRect(0, 0, el.width, el.height);
    if (bg) { ctx.fillStyle = bg; ctx.fillRect(0, 0, el.width, el.height); }
    drawBust(ctx, 0, 0, size * d, person.kit, { person, occasion, cols: cols ?? (size >= 90 ? 52 : 44) });
  }, [person, size, occasion, cols, bg]);
  return <canvas ref={ref} style={{ width: size, height: size, display: "block", flex: "none", imageRendering: "pixelated", ...style }} />;
}

/** A pixel-art standing figure of one `Person`. Height is 2.1 × `w`. */
export function PixelFigure({ person, w, occasion = "national", cols = 30, style }: {
  person: Person; w: number; occasion?: Occasion; cols?: number; style?: React.CSSProperties;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  const h = Math.round(w * 2.1);
  useEffect(() => {
    const el = ref.current; if (!el) return;
    const d = 2; el.width = w * d; el.height = h * d;
    const ctx = el.getContext("2d"); if (!ctx) return;
    ctx.clearRect(0, 0, el.width, el.height);
    drawFigure(ctx, 0, 0, w * d, person.kit, { person, occasion, cols });
  }, [person, w, h, occasion, cols]);
  return <canvas ref={ref} style={{ width: w, height: h, display: "block", flex: "none", imageRendering: "pixelated", ...style }} />;
}
