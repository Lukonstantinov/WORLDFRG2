import { useMemo } from "react";
import { cultureFigureSVG, cultureSeed } from "./cultureFigure";

/** Two full-body figures — a man and a woman of a people in their national dress —
 *  drawn as flat vector "costume plates". Appearance + garments come from the culture's
 *  kit (a creole blends both parent kits). Purely deterministic; no raster assets. */
export function CultureFigures({ name, kit, kit2, color }: {
  name: string; kit?: number; kit2?: number; color?: [number, number, number];
}) {
  const figs = useMemo(() => {
    if (kit == null || kit < 0) return null;
    const seed = cultureSeed(name);
    return {
      m: cultureFigureSVG({ kit, sex: "m", seed, kit2 }),
      f: cultureFigureSVG({ kit, sex: "f", seed: seed ^ 0x9e37, kit2 }),
    };
  }, [name, kit, kit2]);
  if (!figs) return null;
  const plate = color
    ? `linear-gradient(180deg, rgba(${color[0]},${color[1]},${color[2]},0.10), rgba(${color[0]},${color[1]},${color[2]},0.02))`
    : "rgba(255,255,255,0.03)";
  return (
    <div style={{ display: "flex", gap: 10 }}>
      <Plate label="♂ Man" svg={figs.m} bg={plate} />
      <Plate label="♀ Woman" svg={figs.f} bg={plate} />
    </div>
  );
}

function Plate({ label, svg, bg }: { label: string; svg: string; bg: string }) {
  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ height: 150, background: bg, border: "1px solid rgba(180,190,205,0.18)",
        borderRadius: 8, overflow: "hidden", display: "flex", alignItems: "flex-end", justifyContent: "center", padding: "6px 0 0" }}
        dangerouslySetInnerHTML={{ __html: svg }} />
      <div style={{ textAlign: "center", fontSize: 9.5, color: "#8b93a0", marginTop: 3, letterSpacing: 0.3 }}>{label}</div>
    </div>
  );
}
