import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { useViewportStore } from "@state/viewportStore";
import { getCellInfo, getPlateMotion } from "@bridge";
import type { PlateMotionArrow } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";
import { Panel, PanelHeader, PanelBody, Meter } from "@ui/kit";
import { T, FZ } from "@ui/campaign/chronicleTheme";
import { plateName } from "./plateNames";

/** Click a cell while the Plates layer is on and this opens that plate: its generated
 *  name, what it is made of and how it behaves — measured from the generated world
 *  (cells, land share, boundary mix) plus its drift. */
export function PlateInspector() {
  const activeLayer = useUIStore((s) => s.activeLayer);
  const inspected = useUIStore((s) => s.inspectedCell);
  const selectedPlate = useUIStore((s) => s.selectedPlate);
  const setSelectedPlate = useUIStore((s) => s.setSelectedPlate);
  const setInspectedCell = useUIStore((s) => s.setInspectedCell);
  const meta = useWorldStore((s) => s.meta);
  const tileVersion = useViewportStore((s) => s.tileVersion);
  const [plates, setPlates] = useState<PlateMotionArrow[]>([]);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.province);

  const on = activeLayer === "plates";
  useEffect(() => {
    if (!on) return;
    getPlateMotion().then(setPlates).catch(() => setPlates([]));
  }, [on, meta?.name, tileVersion]);

  // A clicked cell names its plate.
  useEffect(() => {
    if (!on || !inspected) return;
    let alive = true;
    getCellInfo(inspected.wx, inspected.wy).then((c) => { if (alive) setSelectedPlate(c.plate_index ?? null); }).catch(() => {});
    return () => { alive = false; };
  }, [on, inspected, setSelectedPlate]);

  if (!on || selectedPlate === null) return null;
  const p = plates.find((x) => x.id === selectedPlate);
  if (!p) return null;
  const worldKey = meta?.name ?? "world";
  const close = () => { setSelectedPlate(null); setInspectedCell(null); };

  const bTotal = p.convergent + p.divergent + p.transform;
  const share = (n: number) => (bTotal > 0 ? n / bTotal : 0);
  const dominant = bTotal === 0 ? "no marked boundary"
    : p.convergent >= p.divergent && p.convergent >= p.transform ? "mostly colliding — mountain belts and trenches"
    : p.divergent >= p.transform ? "mostly pulling apart — rifts, young crust, ridges"
    : "mostly sliding past a neighbour — long straight faults";
  const kind = p.is_oceanic ? "Oceanic" : p.land_frac > 0.35 ? "Continental" : "Continental, mostly drowned";
  const area = `${(p.area_frac * 100).toFixed(1)}% of the world`;

  return (
    <Panel onPointerDown={onPointerDown} width={300} style={{ top: 70, left: 270, zIndex: 41, ...rootStyle }}>
      <PanelHeader icon="🧭" title={plateName(p.id, p.is_oceanic, worldKey)} onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ padding: "6px 10px 10px" }}>
        <Row k="Kind" v={kind} />
        <Row k="Size" v={`${area} · ${p.cells.toLocaleString()} cells`} />
        <Row k="Land" v={`${(p.land_frac * 100).toFixed(0)}% above water`} />
        <Row k="Drift" v={`${compass(p.vx, p.vy)} · ${p.speed < 0.02 ? "barely moving" : p.speed < 0.08 ? "slow" : "fast"} (${p.speed.toFixed(2)})`} />
        <div style={{ color: T.inkDim, fontSize: FZ.tiny, margin: "8px 0 3px", textTransform: "uppercase", letterSpacing: 0.5 }}>Edge character</div>
        {([["Convergent", p.convergent, "#d9694a"], ["Divergent", p.divergent, "#5a9ad8"], ["Transform", p.transform, "#c9b04a"]] as const).map(([label, n, color]) => (
          <div key={label} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
            <span style={{ width: 70, color: T.inkMid, fontSize: FZ.small }}>{label}</span>
            <div style={{ flex: 1 }}><Meter value={share(n)} max={1} color={color} /></div>
            <span style={{ width: 34, textAlign: "right", color: T.ink, fontSize: FZ.small }}>{Math.round(share(n) * 100)}%</span>
          </div>
        ))}
        <div style={{ color: T.inkMid, fontSize: FZ.small, marginTop: 6, lineHeight: 1.45 }}>This plate is {dominant}.</div>
      </PanelBody>
    </Panel>
  );
}

function Row({ k, v }: { k: string; v: string }) {
  return (
    <div style={{ display: "flex", gap: 8, padding: "2px 0", fontSize: FZ.body }}>
      <span style={{ width: 70, color: T.inkDim }}>{k}</span>
      <span style={{ color: T.ink, flex: 1 }}>{v}</span>
    </div>
  );
}

/** Screen-space drift (x east, y south) as a compass word. */
function compass(vx: number, vy: number): string {
  if (Math.hypot(vx, vy) < 1e-4) return "stationary";
  const a = (Math.atan2(vy, vx) * 180) / Math.PI;
  const dirs = ["east", "south-east", "south", "south-west", "west", "north-west", "north", "north-east"];
  return `towards the ${dirs[Math.round(((a + 360) % 360) / 45) % 8]}`;
}
