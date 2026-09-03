import { useEffect, useRef, useState } from "react";
import { drawBust, drawFigure, resolveKit, REGISTERS, creoleKit, type Occasion, type DressKit } from "@ui/campaign/cultureDress";

const OCCASION_TITLE: Record<Occasion, string> = {
  everyday: "Working dress — plainer cloth, no finery",
  national: "National costume",
  ceremonial: "Ceremonial finery — mantle, jewels, richer trim",
};

/** Authored at 2× and shown at half — the pixel treatment's one-pixel edge only
 *  survives an integer upscale with smoothing off. */
const PLATE_SCALE = 2;

/** A people's DRESS PLATE: the portrait bust beside the full costume figure.
 *  Headwear silhouette and neckline are per-people, which is what actually reads
 *  at portrait size; a worldgen hearth with no preset kit gets a derived one and
 *  a creole gets a composite of its two parents, so nothing renders blank. */
export function CultureFigures({ name, kit, kit2, color }: {
  name: string; kit?: number; kit2?: number; color?: [number, number, number];
}) {
  const [occasion, setOccasion] = useState<Occasion>("national");
  const bustRef = useRef<HTMLCanvasElement | null>(null);
  const figRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    let K: DressKit;
    if (kit2 != null && kit2 >= 0 && kit2 !== kit) K = creoleKit(name, kit ?? name, kit2);
    else K = resolveKit(kit != null && kit >= 0 ? kit : name, { region: "" });

    const paint = (el: HTMLCanvasElement | null, w: number, h: number, draw: (c: CanvasRenderingContext2D) => void) => {
      if (!el) return;
      el.width = w * PLATE_SCALE; el.height = h * PLATE_SCALE;
      el.style.width = w + "px"; el.style.height = h + "px";
      const ctx = el.getContext("2d");
      if (!ctx) return;
      ctx.clearRect(0, 0, el.width, el.height);
      draw(ctx);
    };
    paint(bustRef.current, BUST, BUST, (c) => drawBust(c, 0, 0, BUST * PLATE_SCALE, K, { occasion }));
    paint(figRef.current, FIG_W, FIG_H, (c) => drawFigure(c, 0, 0, FIG_W * PLATE_SCALE, K, { occasion }));
  }, [name, kit, kit2, occasion]);

  const plate = color
    ? `linear-gradient(180deg, rgba(${color[0]},${color[1]},${color[2]},0.12), rgba(${color[0]},${color[1]},${color[2]},0.02))`
    : "rgba(255,255,255,0.03)";

  return (
    <div>
      <div style={{
        display: "flex", alignItems: "flex-end", justifyContent: "center", gap: 12,
        height: 132, background: plate, border: "1px solid #16202e", borderRadius: 6, padding: "4px 8px",
      }}>
        <canvas ref={bustRef} style={{ display: "block" }} />
        <canvas ref={figRef} style={{ display: "block" }} />
      </div>
      <div style={{ display: "flex", justifyContent: "center", marginTop: 5 }}>
        <div style={{ display: "flex", border: "1px solid rgba(180,190,205,0.25)", borderRadius: 12, overflow: "hidden" }}>
          {REGISTERS.map((o) => (
            <button key={o} onClick={() => setOccasion(o)} title={OCCASION_TITLE[o]}
              style={{
                fontSize: 9.5, padding: "2px 10px", cursor: "pointer", border: "none", textTransform: "capitalize",
                background: occasion === o ? "rgba(180,150,90,0.28)" : "transparent",
                color: occasion === o ? "#e6cf9a" : "#8b93a0", fontWeight: occasion === o ? 700 : 400,
              }}>
              {o}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

const BUST = 92, FIG_W = 56, FIG_H = Math.round(56 * 2.1);
