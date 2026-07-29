import { useMemo, useState } from "react";
import type { PBuilding, PSettlement, Province, ProvinceLand, ProvinceLandSample } from "@types";
import type { ProvinceRaster } from "@state/worldStore";

// The province SURVEY PLATE: a stack of toggleable layers over one province's
// footprint, in the tradition of an estate map or a geological sheet — the same idiom
// §8.12 established for the biome hatching on the main map.
//
// One thing to be honest about in the rendering. The campaign holds land use and
// tenure as SHARES of a province, not as a spatial layout — there is no per-cell land
// register, and inventing one would be a lie dressed as detail. So the land-use and
// tenure plates DITHER: each sampled cell is assigned a class by a stable per-cell
// hash against the cumulative shares. That is truthful (the mosaic's proportions are
// exactly the model's shares), it is stable (the same cell keeps its class between
// years, so scrubbing the time slider shows land genuinely converting rather than
// reshuffling), and it reads as a cartographic fill rather than as false precision.

// ── Custom minimalist building icons (tiny SVG, no emoji). kind: 0 estate ·
//    1 manufactory · 2 warehouse · 3 bank · 4 mint. Drawn centred on (0,0). ──
const B_COLOR = ["#7fb069", "#d98c40", "#5a9bd4", "#e3c14a", "#c9a24a"];
const B_LABEL = ["Estate", "Manufactory", "Depot", "Bank", "Mint"];
function BuildingGlyph({ kind, s }: { kind: number; s: number }) {
  const c = B_COLOR[kind] ?? "#aaa";
  switch (kind) {
    case 0: // estate — filled field square
      return <rect x={-s} y={-s} width={2 * s} height={2 * s} rx={1} fill={c} stroke="#0a1620" strokeWidth={0.6} />;
    case 1: // manufactory — roof/triangle
      return <path d={`M0 ${-s * 1.2} L ${s} ${s} L ${-s} ${s} Z`} fill={c} stroke="#0a1620" strokeWidth={0.6} />;
    case 2: // warehouse — crate (outlined square with a bar)
      return <g stroke="#0a1620" strokeWidth={0.6}>
        <rect x={-s} y={-s} width={2 * s} height={2 * s} rx={0.5} fill={c} />
        <line x1={-s} y1={0} x2={s} y2={0} stroke="#0a1620" strokeWidth={0.7} />
      </g>;
    case 3: // bank — diamond
      return <path d={`M0 ${-s * 1.3} L ${s * 1.1} 0 L 0 ${s * 1.3} L ${-s * 1.1} 0 Z`} fill={c} stroke="#0a1620" strokeWidth={0.6} />;
    case 4: // mint — coin (circle + dot)
      return <g stroke="#0a1620" strokeWidth={0.6}><circle r={s} fill={c} /><circle r={s * 0.35} fill="#0a1620" /></g>;
    default:
      return <circle r={s} fill={c} />;
  }
}

/** The plates, bottom-up. `relief` is the ground everything else reads against. */
export type PlateKey = "relief" | "water" | "landuse" | "tenure" | "holdings" | "borders";
export const PLATE_LABEL: Record<PlateKey, string> = {
  relief: "relief", water: "water", landuse: "land use",
  tenure: "tenure", holdings: "holdings", borders: "borders",
};
export const DEFAULT_PLATES: PlateKey[] = ["relief", "water", "landuse", "holdings", "borders"];

/** Land-use classes, with the colour each dithered cell takes. */
const LANDUSE = [
  { key: "arable", label: "arable", color: "#c8a94e" },
  { key: "pasture", label: "pasture", color: "#7fa05a" },
  { key: "forest", label: "woodland", color: "#2f6b46" },
  { key: "waste", label: "moor & waste", color: "#6b6a5c" },
] as const;

/** Tenure classes. The house share is drawn in the holding family's OWN colour where
 *  there is exactly one; otherwise a generic private tint. */
const TENURE = [
  { key: "civic", label: "civic & crown", color: "#5a7fa8" },
  { key: "house", label: "house & noble", color: "#b06a4a" },
  { key: "temple", label: "temple", color: "#9b7fc0" },
  { key: "common", label: "common land", color: "#6f8f6a" },
] as const;

/** Stable 0..1 hash of a cell. Keeps a cell's dithered class fixed across years and
 *  across renders, which is what makes the time slider show CONVERSION not reshuffling. */
function cellHash(rx: number, ry: number, salt: number): number {
  let h = (rx * 374761393 + ry * 668265263 + salt * 1442695041) | 0;
  h = (h ^ (h >>> 13)) * 1274126177 | 0;
  h = h ^ (h >>> 16);
  return ((h >>> 8) & 0xffff) / 0xffff;
}

/** Pick a class index for a cell given cumulative shares. */
function ditherClass(shares: number[], t: number): number {
  const total = shares.reduce((a, b) => a + Math.max(0, b), 0);
  if (total <= 0) return shares.length - 1;
  let acc = 0;
  const x = t * total;
  for (let i = 0; i < shares.length; i++) {
    acc += Math.max(0, shares[i]);
    if (x <= acc) return i;
  }
  return shares.length - 1;
}

interface Hover { x: number; y: number; title: string; rows: [string, string][] }

export function ProvinceMiniMap({
  province, raster, settlements, buildings,
  land, sample, plates = DEFAULT_PLATES, width = 240, riverCells,
}: {
  province: Province;
  raster: ProvinceRaster | null;
  settlements: PSettlement[];
  buildings: PBuilding[];
  /** Live land state — required for the land-use and tenure plates. */
  land?: ProvinceLand | null;
  /** A historical year's sample: when present the land plates show THAT year. */
  sample?: ProvinceLandSample | null;
  plates?: PlateKey[];
  width?: number;
  /** How many of the province's cells carry a river (drives the water plate's density). */
  riverCells?: number;
}) {
  const [hover, setHover] = useState<Hover | null>(null);

  // Extract the province's footprint from the raster → an SVG viewBox. To keep the
  // SHAPE (and its proportions) faithful for a small province as well as a huge one,
  // the sample stride is sized to THIS province's bounding box, not the whole map:
  // pass 1 finds the bbox coarsely, pass 2 re-samples it at a per-province stride so
  // every province renders at a similar ~130-cell fidelity instead of a coarse blob.
  const geo = useMemo(() => {
    if (!raster) return null;
    const { data, w, h, gridW, gridH } = raster;
    // Pass 1 — coarse bbox.
    const coarse = Math.max(1, Math.round(Math.max(w, h) / 300));
    let minx = w, miny = h, maxx = -1, maxy = -1;
    for (let ry = 0; ry < h; ry += coarse) {
      for (let rx = 0; rx < w; rx += coarse) {
        if (data[ry * w + rx] !== province.id) continue;
        if (rx < minx) minx = rx; if (rx > maxx) maxx = rx;
        if (ry < miny) miny = ry; if (ry > maxy) maxy = ry;
      }
    }
    if (maxx < 0) return null;
    // Widen by the coarse step so the fine pass doesn't clip the edges.
    minx = Math.max(0, minx - coarse); miny = Math.max(0, miny - coarse);
    maxx = Math.min(w - 1, maxx + coarse); maxy = Math.min(h - 1, maxy + coarse);
    // Pass 2 — fine scan within the bbox at a province-relative stride.
    const bw = maxx - minx + 1, bh = maxy - miny + 1;
    const stride = Math.max(1, Math.round(Math.max(bw, bh) / 130));
    const cells: [number, number][] = [];
    for (let ry = miny; ry <= maxy; ry += stride) {
      for (let rx = minx; rx <= maxx; rx += stride) {
        if (data[ry * w + rx] === province.id) cells.push([rx, ry]);
      }
    }
    if (cells.length === 0) return null;
    // Edge cells — a footprint cell with at least one non-member 4-neighbour. This is
    // what the borders plate outlines, so the frontier reads as a drawn line rather
    // than as the accidental edge of a mosaic.
    const member = new Set(cells.map(([x, y]) => y * w + x));
    const edge = cells.filter(([x, y]) =>
      !member.has(y * w + (x - stride)) || !member.has(y * w + (x + stride))
      || !member.has((y - stride) * w + x) || !member.has((y + stride) * w + x));
    const pad = stride;
    const ox = minx - pad, oy = miny - pad;
    const vw = bw + stride + 2 * pad, vh = bh + stride + 2 * pad;
    // world-cell → raster-cell → local viewBox coords
    const toLocal = (x: number, y: number): [number, number] => [
      (x * w) / gridW - ox, (y * h) / gridH - oy,
    ];
    return { cells, edge, ox, oy, vw, vh, toLocal, stride };
  }, [raster, province.id]);

  if (!geo) return <div style={{ opacity: 0.5, padding: 8 }}>map unavailable</div>;

  const { cells, edge, ox, oy, vw, vh, toLocal, stride } = geo;
  const on = (k: PlateKey) => plates.includes(k);
  const boxW = width, boxH = Math.max(120, Math.round((width * vh) / vw));
  // Settlement dot radius proportional to the province footprint so a 4-cell micro-
  // province doesn't have dots bigger than itself, and a 500-cell province still
  // shows legible markers. Clamped to a narrow range so they never disappear either.
  const dotScale = Math.min(1.0, Math.max(0.35, 1.8 / Math.sqrt(cells.length)));
  // City size: marker scales with population (√, so a metropolis reads bigger than a
  // hamlet without dwarfing it), normalised to the largest settlement in the province.
  const maxPop = Math.max(1, ...settlements.map((s) => s.population || 1));
  const popScale = (pop: number) => 0.55 + 1.1 * Math.sqrt((pop || 1) / maxPop);

  // The land figures the plates draw: a scrubbed historical sample when the slider is
  // off "today", else the live state.
  const lu = sample ?? land ?? null;
  const luShares = lu
    ? [lu.arable, lu.pasture, lu.forest,
       Math.max(0, 1 - lu.arable - lu.pasture - lu.forest)]
    : null;
  const tenure = land?.tenure ?? null;
  // A single holding family colours the house share in its own colour — the tenure
  // plate then says WHO, not merely "private".
  const soleHolder = land && land.holders.length === 1 ? land.holders[0] : null;
  const tenureColor = (i: number) =>
    i === 1 && soleHolder ? soleHolder.color : TENURE[i].color;

  // Water: the model gives a river CELL COUNT, not a course, so the plate marks a
  // proportional scatter of watered cells rather than drawing a fictional channel.
  const waterFrac = riverCells !== undefined && cells.length > 0
    ? Math.min(0.5, riverCells / Math.max(1, province.cells))
    : 0;

  return (
    <div style={{ display: "flex", gap: 10, position: "relative", flexWrap: "wrap" }}>
      {/* The plate */}
      <svg width={boxW} height={boxH} viewBox={`0 0 ${vw} ${vh}`}
        style={{ background: "#0a1620", border: "1px solid #1c3242", borderRadius: 6, flexShrink: 0 }}
        onMouseLeave={() => setHover(null)}>
        {/* 1 · relief — the ground. Also the fallback when no land layer exists. */}
        {(on("relief") || !luShares) && cells.map(([rx, ry], i) => (
          <rect key={`r${i}`} x={rx - ox} y={ry - oy} width={stride * 1.05} height={stride * 1.05}
            fill="#3f6d55" opacity={on("landuse") && luShares ? 0.28 : 0.55} />
        ))}

        {/* 3 · land use — the plate that CHANGES over five centuries. */}
        {on("landuse") && luShares && cells.map(([rx, ry], i) => {
          const c = LANDUSE[ditherClass(luShares, cellHash(rx, ry, 1))];
          return (
            <rect key={`u${i}`} x={rx - ox} y={ry - oy} width={stride * 1.05} height={stride * 1.05}
              fill={c.color} opacity={0.75} />
          );
        })}

        {/* 4 · tenure — who actually holds the land. */}
        {on("tenure") && tenure && cells.map(([rx, ry], i) => {
          const k = ditherClass([...tenure], cellHash(rx, ry, 7));
          return (
            <rect key={`t${i}`} x={rx - ox} y={ry - oy} width={stride * 1.05} height={stride * 1.05}
              fill={tenureColor(k)} opacity={on("landuse") ? 0.45 : 0.7} />
          );
        })}

        {/* 2 · water — a proportional scatter, honest about carrying no course. */}
        {on("water") && waterFrac > 0 && cells.map(([rx, ry], i) =>
          cellHash(rx, ry, 23) < waterFrac ? (
            <rect key={`w${i}`} x={rx - ox + stride * 0.2} y={ry - oy + stride * 0.35}
              width={stride * 0.7} height={stride * 0.28} rx={stride * 0.14}
              fill="#4a90c4" opacity={0.8} />
          ) : null)}

        {/* 6 · borders — the frontier as a drawn line. */}
        {on("borders") && edge.map(([rx, ry], i) => (
          <rect key={`e${i}`} x={rx - ox} y={ry - oy} width={stride * 1.05} height={stride * 1.05}
            fill="none" stroke="#d8c9a0" strokeWidth={stride * 0.22} opacity={0.55} />
        ))}

        {/* 5 · holdings — settlements */}
        {on("holdings") && settlements.map((s, i) => {
          const [lx, ly] = toLocal(s.x, s.y);
          const psc = popScale(s.population);
          const r = (s.seat ? 1.6 : 1.1) * dotScale * psc;
          return (
            <g key={`s${i}`} transform={`translate(${lx} ${ly})`} style={{ cursor: "pointer" }}
              onMouseEnter={(e) => setHover({
                x: e.nativeEvent.offsetX, y: e.nativeEvent.offsetY,
                title: `${s.seat ? "★ " : ""}${s.name}${s.seat ? " (seat)" : ""}`,
                rows: [["Population", s.population.toLocaleString()],
                       ["Class", ["ordinary", "trade hub", "entrepôt"][s.hub_class] ?? "—"]],
              })}
              onMouseLeave={() => setHover(null)}>
              {s.seat
                ? <path transform={`scale(${dotScale * psc})`}
                    d="M0 -2.2 L0.7 -0.7 L2.2 -0.7 L1 0.4 L1.4 2 L0 1 L-1.4 2 L-1 0.4 L-2.2 -0.7 L-0.7 -0.7 Z"
                    fill="#fff" stroke="#0a1620" strokeWidth={0.4 / (dotScale * psc)} />
                : <circle r={r} fill="#e8eef4" stroke="#0a1620" strokeWidth={0.4} />}
            </g>
          );
        })}
        {/* 5 · holdings — buildings, custom minimalist icons */}
        {on("holdings") && buildings.map((b, i) => {
          const [lx, ly] = toLocal(b.x, b.y);
          return (
            <g key={`b${i}`} transform={`translate(${lx} ${ly})`} style={{ cursor: "pointer" }}
              onMouseEnter={(e) => setHover({
                x: e.nativeEvent.offsetX, y: e.nativeEvent.offsetY,
                title: `${B_LABEL[b.kind] ?? "Building"} · ${b.name}`,
                rows: b.stats.map((st) => [st.label, st.value] as [string, string]),
              })}
              onMouseLeave={() => setHover(null)}>
              <BuildingGlyph kind={b.kind} s={1.5} />
            </g>
          );
        })}
      </svg>

      {/* Legend for whichever land plate is on top, else the holdings list. */}
      <div style={{ flex: 1, minWidth: 120, fontSize: 12, maxHeight: boxH, overflowY: "auto" }}>
        {on("tenure") && tenure ? (
          <>
            {TENURE.map((t, i) => (
              <LegendRow key={t.key} color={tenureColor(i)}
                label={i === 1 && soleHolder ? soleHolder.name : t.label}
                value={`${Math.round(tenure[i] * 100)}%`} />
            ))}
            {land && land.holders.length > 1 && (
              <div style={{ opacity: 0.6, fontSize: 11, marginTop: 3 }}>
                {land.holders.length} families hold estates here
              </div>
            )}
          </>
        ) : on("landuse") && luShares ? (
          <>
            {LANDUSE.map((c, i) => (
              <LegendRow key={c.key} color={c.color} label={c.label}
                value={`${Math.round(luShares[i] * 100)}%`} />
            ))}
            {lu && (
              <div style={{ opacity: 0.6, fontSize: 11, marginTop: 3 }}>
                soil in {soilWord(lu.soil)} heart
                {lu.irrigated > 0.01 ? ` · ${Math.round(lu.irrigated * 100)}% watered` : ""}
              </div>
            )}
          </>
        ) : (
          <>
            {settlements.map((s, i) => (
              <div key={`ls${i}`} style={{ padding: "1px 0", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                {s.seat ? "★ " : "· "}{s.name} <span style={{ opacity: 0.55 }}>{s.population.toLocaleString()}</span>
              </div>
            ))}
            {buildings.length > 0 && <div style={{ height: 4 }} />}
            {buildings.map((b, i) => (
              <div key={`lb${i}`} style={{ display: "flex", gap: 5, alignItems: "center", padding: "1px 0" }}>
                <svg width={12} height={12} viewBox="-3 -3 6 6"><BuildingGlyph kind={b.kind} s={1.6} /></svg>
                <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{b.name}</span>
              </div>
            ))}
            {settlements.length === 0 && buildings.length === 0 &&
              <div style={{ opacity: 0.5 }}>frontier — no holdings</div>}
          </>
        )}
      </div>

      {/* Hover tooltip */}
      {hover && (
        <div style={{
          position: "absolute", left: Math.min(hover.x + 12, boxW - 40), top: hover.y + 8,
          background: "#10202c", border: "1px solid #2a4a5a",
          borderRadius: 6, padding: "6px 8px", pointerEvents: "none", zIndex: 60,
          font: "11px/1.35 system-ui", color: "#dceaf4", boxShadow: "0 4px 14px rgba(0,0,0,.5)",
          maxWidth: 220,
        }}>
          <div style={{ fontWeight: 600, marginBottom: 3 }}>{hover.title}</div>
          {hover.rows.map(([k, v], i) => (
            <div key={i} style={{ display: "flex", justifyContent: "space-between", gap: 10 }}>
              <span style={{ opacity: 0.6 }}>{k}</span><span>{v}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function LegendRow({ color, label, value }: { color: string; label: string; value: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6, padding: "1px 0" }}>
      <span style={{ width: 9, height: 9, borderRadius: 2, background: color, flex: "0 0 auto" }} />
      <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
        {label}
      </span>
      <span style={{ opacity: 0.7 }}>{value}</span>
    </div>
  );
}

export function soilWord(soil: number): string {
  if (soil >= 0.8) return "excellent";
  if (soil >= 0.62) return "good";
  if (soil >= 0.45) return "fair";
  if (soil >= 0.32) return "poor";
  return "exhausted";
}

/** The plate's layer toggles — exported so the panels share one control. */
export function PlateToggles({ plates, setPlates, disabled = [] }: {
  plates: PlateKey[];
  setPlates: (p: PlateKey[]) => void;
  /** Plates with no data behind them, shown greyed rather than hidden. */
  disabled?: PlateKey[];
}) {
  const keys: PlateKey[] = ["relief", "water", "landuse", "tenure", "holdings", "borders"];
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 3 }}>
      {keys.map((k) => {
        const off = disabled.includes(k);
        const active = plates.includes(k) && !off;
        return (
          <span key={k} onClick={() => {
            if (off) return;
            setPlates(active ? plates.filter((p) => p !== k) : [...plates, k]);
          }} title={off ? "no data for this layer yet" : `Toggle the ${PLATE_LABEL[k]} plate`}
            style={{
              fontSize: 10, padding: "1px 6px", borderRadius: 3, userSelect: "none",
              cursor: off ? "default" : "pointer",
              opacity: off ? 0.35 : 1,
              color: active ? "#0a1620" : "#9ab0c8",
              background: active ? "#8fb89a" : "transparent",
              border: `1px solid ${active ? "#8fb89a" : "#2a4436"}`,
            }}>
            {active ? "☑" : "☐"} {PLATE_LABEL[k]}
          </span>
        );
      })}
    </div>
  );
}
