import { useMemo, useState } from "react";
import { cultureFigureSVG, cultureSeed } from "./cultureFigure";

/** Full-body figures of a people in national dress — a man and a woman, drawn as flat
 *  vector "costume plates". Appearance + garments come from the culture's kit and are
 *  RANDOMISED per individual, so ↻ re-rolls two fresh people of the same culture. A
 *  creole blends both parent kits. Purely deterministic; no raster assets. */
export function CultureFigures({ name, kit, kit2, color }: {
  name: string; kit?: number; kit2?: number; color?: [number, number, number];
}) {
  const [roll, setRoll] = useState(0);
  const figs = useMemo(() => {
    if (kit == null || kit < 0) return null;
    const base = cultureSeed(name);
    return {
      m: cultureFigureSVG({ kit, sex: "m", seed: base, kit2, variant: roll }),
      f: cultureFigureSVG({ kit, sex: "f", seed: base ^ 0x9e37, kit2, variant: roll }),
    };
  }, [name, kit, kit2, roll]);
  if (!figs) return null;
  const plate = color
    ? `linear-gradient(180deg, rgba(${color[0]},${color[1]},${color[2]},0.12), rgba(${color[0]},${color[1]},${color[2]},0.02))`
    : "rgba(255,255,255,0.03)";
  return (
    <div>
      <div style={{ display: "flex", gap: 10 }}>
        <Plate label="♂ Man" svg={figs.m} bg={plate} />
        <Plate label="♀ Woman" svg={figs.f} bg={plate} />
      </div>
      <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 5 }}>
        <button onClick={() => setRoll((r) => r + 1)} title="Show two other people of this culture"
          style={{ fontSize: 10, padding: "2px 10px", borderRadius: 12, cursor: "pointer",
            border: "1px solid rgba(180,190,205,0.3)", background: "rgba(255,255,255,0.05)", color: "#aeb6c0" }}>
          ↻ another
        </button>
      </div>
    </div>
  );
}

function Plate({ label, svg, bg }: { label: string; svg: string; bg: string }) {
  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ height: 172, background: bg, border: "1px solid rgba(180,190,205,0.18)",
        borderRadius: 8, overflow: "hidden", display: "flex", alignItems: "flex-end", justifyContent: "center" }}
        dangerouslySetInnerHTML={{ __html: svg }} />
      <div style={{ textAlign: "center", fontSize: 9.5, color: "#8b93a0", marginTop: 3, letterSpacing: 0.3 }}>{label}</div>
    </div>
  );
}
