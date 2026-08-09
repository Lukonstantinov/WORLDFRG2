import { useMemo, useState } from "react";
import type { PBuilding, PSettlement, Province, ProvinceLand, ProvinceLandSample, ProvinceTerrainCrop, RiverData } from "@types";
import type { ProvinceRaster } from "@state/worldStore";
import { GOOD_DEFS } from "@goods";

/** Hypsometric relief shading from a REAL elevation sample (§2.3) — sea a flat
 *  blue (elevation isn't meaningful for sea depth here), land a low-to-high ramp
 *  anchored at the map's old flat placeholder green so the plate doesn't jump in
 *  hue at zero elevation, rising through olive/tan/brown to a pale peak. */
function reliefColor(elev: number, isLand: boolean): string {
  if (!isLand) return "#1e3a52";
  const stops: [number, string][] = [
    [0.00, "#3f6d55"], [0.15, "#5c7a4a"], [0.35, "#8a8248"],
    [0.55, "#8a6a4a"], [0.75, "#8a8078"], [1.00, "#e8e4dc"],
  ];
  const e = Math.max(0, Math.min(1, elev));
  let lo = stops[0], hi = stops[stops.length - 1];
  for (let i = 0; i < stops.length - 1; i++) {
    if (e >= stops[i][0] && e <= stops[i + 1][0]) { lo = stops[i]; hi = stops[i + 1]; break; }
  }
  const span = hi[0] - lo[0];
  const t = span > 1e-6 ? (e - lo[0]) / span : 0;
  const mix = (a: string, b: string, t: number) => {
    const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
    const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
    const c = pa.map((v, i) => Math.round(v + (pb[i] - v) * t));
    return `#${c.map((v) => v.toString(16).padStart(2, "0")).join("")}`;
  };
  return mix(lo[1], hi[1], t);
}

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
//
// §2.4 · the LAND-USE plate's placement (not tenure — who holds a field doesn't
// correlate with its altitude) is further biased by real elevation: woodland and
// waste favour higher ground, arable and pasture favour the flat, via a RANK
// transform (`landUsePercentile`) rather than a direct threshold shift, so the
// province's overall shares stay exact regardless of the bias — only the ORDER
// cells are assigned to correlates with elevation.

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
export type PlateKey = "relief" | "water" | "landuse" | "tenure" | "holdings" | "borders" | "goods" | "deposits";
export const PLATE_LABEL: Record<PlateKey, string> = {
  relief: "relief", water: "water", landuse: "land use",
  tenure: "tenure", holdings: "holdings", borders: "borders",
  // Two DISTINCT plates: "goods" = the surface/belt produce (best-quality areas),
  // "deposits" = the ore/mineral workings where they actually sit.
  goods: "goods", deposits: "deposits",
};
export const DEFAULT_PLATES: PlateKey[] = ["relief", "water", "landuse", "holdings", "borders"];

/** Land-use classes, with the colour each dithered cell takes. */
const LANDUSE = [
  // Value separation matters more than hue here: the first palette put a mid-olive
  // arable against a mid-green wood, and at plate size the two were indistinguishable
  // so a province that had cleared half its woodland looked unchanged. Arable is now
  // a light stubble-gold and woodland a deep green — a clear light/dark split, which
  // is how a printed land-use sheet separates them too.
  { key: "arable", label: "arable", color: "#e0c46a" },
  { key: "pasture", label: "pasture", color: "#8fae5e" },
  { key: "forest", label: "woodland", color: "#245a3a" },
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

/** Stable 0..1 hash of a PATCH of cells. Keeps a cell's dithered class fixed across
 *  years and across renders, which is what makes the time slider show CONVERSION not
 *  reshuffling.
 *
 *  `patch` quantises the coordinates so neighbouring cells share a class. Hashing each
 *  cell independently (patch = 1) was the first cut and it rendered as television
 *  static — technically the right proportions, visually unreadable, and nothing like a
 *  survey sheet. Real land use comes in fields, so the dither is grouped into blocks a
 *  few sampled cells across. */
function cellHash(rx: number, ry: number, salt: number, patch = 1): number {
  const px = Math.floor(rx / patch), py = Math.floor(ry / patch);
  let h = (px * 374761393 + py * 668265263 + salt * 1442695041) | 0;
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
  terrain, rivers, deposits = [], beltGoods = [],
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
  /** Real cropped elevation/land grid (§2.3) — the relief plate's true base layer.
   *  Falls back to the old flat fill when absent (an older world, or still loading). */
  terrain?: ProvinceTerrainCrop | null;
  /** The world's full river geometry — clipped here to this province's own raster
   *  mask, so a drawn course is honestly this province's own reach of the river. */
  rivers?: RiverData[];
  width?: number;
  /** How many of the province's cells carry a river (drives the water plate's density). */
  riverCells?: number;
  /** #9 · ore workings in this province (world cell coords) for the deposits plate. */
  deposits?: { good: string; x: number; y: number; grade: number; depth: number }[];
  /** #9 · untapped surface/belt goods (no single cell) — drawn as quality-weighted
   *  squares dithered across the province footprint on the deposits plate. */
  beltGoods?: { name: string; quality: number }[];
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

  // Dither patch size, in raster units: a few SAMPLED cells across (cells are
  // `stride` apart), so the land-use/tenure mosaic reads as fields regardless of
  // how big the province is. Computed here (not after the `!geo` return below)
  // because the elevation-bias rank transform below is a hook and needs it too.
  const patchSize = geo ? geo.stride * 3 : 1;

  // Real elevation/land samples (§2.3) — the relief plate's true base layer,
  // positioned by the SAME world→local transform the settlements/buildings use.
  const terrainSamples = useMemo(() => {
    if (!geo || !terrain || terrain.cols <= 0 || terrain.rows <= 0) return [];
    const out: { lx: number; ly: number; color: string }[] = [];
    for (let r = 0; r < terrain.rows; r++) {
      for (let c = 0; c < terrain.cols; c++) {
        const wx = terrain.ox + c * terrain.stride;
        const wy = terrain.oy + r * terrain.stride;
        const i = r * terrain.cols + c;
        const [lx, ly] = geo.toLocal(wx, wy);
        out.push({ lx, ly, color: reliefColor(terrain.elevation[i], terrain.land[i] === 1) });
      }
    }
    return out;
  }, [geo, terrain]);
  const terrainRectSize = raster && terrain
    ? Math.max((terrain.stride * raster.w) / raster.gridW, (terrain.stride * raster.h) / raster.gridH)
    : 0;

  // Real river courses, clipped to this province's own raster mask — a run of
  // points is kept whenever it (or an adjacent point) belongs to the province, so
  // the drawn line reaches the border rather than stopping a cell short.
  const riverSegments = useMemo(() => {
    if (!geo || !raster || !rivers || rivers.length === 0) return [];
    const { data, w: rw2, h: rh2, gridW, gridH } = raster;
    const inProvince = (wx: number, wy: number): boolean => {
      const rx = Math.min(rw2 - 1, Math.max(0, Math.round((wx * rw2) / gridW)));
      const ry = Math.min(rh2 - 1, Math.max(0, Math.round((wy * rh2) / gridH)));
      return data[ry * rw2 + rx] === province.id;
    };
    const segs: [number, number][][] = [];
    for (const rv of rivers) {
      const pts = rv.points;
      if (!pts || pts.length < 2) continue;
      let cur: [number, number][] = [];
      for (let i = 0; i < pts.length; i++) {
        const [wx, wy] = pts[i];
        const here = inProvince(wx, wy);
        const prevHere = i > 0 && inProvince(pts[i - 1][0], pts[i - 1][1]);
        const nextHere = i < pts.length - 1 && inProvince(pts[i + 1][0], pts[i + 1][1]);
        if (here || prevHere || nextHere) {
          cur.push(geo.toLocal(wx, wy));
        } else if (cur.length > 1) {
          segs.push(cur); cur = [];
        } else {
          cur = [];
        }
      }
      if (cur.length > 1) segs.push(cur);
    }
    return segs;
  }, [geo, raster, rivers, province.id]);

  // Elevation-biased placement for the land-use dither (§2.4): woodland/waste
  // favour higher elevation, arable/pasture favour flat land. Cells are grouped
  // into the SAME patch the dither already quantises to (so fields read as
  // blocks, not static), each patch gets one composite score (elevation + a
  // little noise for texture within a band), and patches are RANKED rather than
  // thresholded directly — so the set of percentiles handed to `ditherClass` is
  // exactly a permutation of (0,1) and the province's overall shares stay exact
  // regardless of the bias. Only the ORDER correlates with elevation; the class
  // cutoffs still come from this year's real shares, which is what lets a cell's
  // class change as the land actually converts (rule 17).
  const landUsePercentile = useMemo(() => {
    const m = new Map<string, number>();
    if (!geo || geo.cells.length === 0) return m;
    const worldOf = (rx: number, ry: number): [number, number] =>
      raster ? [(rx * raster.gridW) / raster.w, (ry * raster.gridH) / raster.h] : [rx, ry];
    const elevAt = (wx: number, wy: number): number => {
      if (!terrain || terrain.cols === 0 || terrain.rows === 0) return 0.3; // neutral default
      const c = Math.min(terrain.cols - 1, Math.max(0, Math.round((wx - terrain.ox) / terrain.stride)));
      const r = Math.min(terrain.rows - 1, Math.max(0, Math.round((wy - terrain.oy) / terrain.stride)));
      const i = r * terrain.cols + c;
      return terrain.land[i] === 1 ? terrain.elevation[i] : 0.3;
    };
    const patchKeys = new Set<string>();
    for (const [rx, ry] of geo.cells) {
      patchKeys.add(`${Math.floor(rx / patchSize)},${Math.floor(ry / patchSize)}`);
    }
    const scored = [...patchKeys].map((key) => {
      const [px, py] = key.split(",").map(Number);
      const rx = px * patchSize, ry = py * patchSize;
      const [wx, wy] = worldOf(rx, ry);
      const noise = cellHash(rx, ry, 1, 1); // already patch-quantised via rx/ry above
      return { key, score: terrain ? elevAt(wx, wy) * 0.8 + noise * 0.2 : noise };
    });
    scored.sort((a, b) => a.score - b.score);
    scored.forEach((s, i) => m.set(s.key, (i + 0.5) / scored.length));
    return m;
  }, [geo, raster, terrain, patchSize]);

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
        {/* 1 · relief — the ground. Real elevation/land when the crop has loaded
               (§2.3); the old flat fill is the fallback while it loads or on a
               world saved before this existed. */}
        {(on("relief") || !luShares) && terrainSamples.length > 0 && (
          <g opacity={on("landuse") && luShares ? 0.5 : 0.95}>
            {terrainSamples.map((s, i) => (
              <rect key={`rt${i}`} x={s.lx - terrainRectSize / 2} y={s.ly - terrainRectSize / 2}
                width={terrainRectSize * 1.05} height={terrainRectSize * 1.05} fill={s.color} />
            ))}
          </g>
        )}
        {(on("relief") || !luShares) && terrainSamples.length === 0 && cells.map(([rx, ry], i) => (
          <rect key={`r${i}`} x={rx - ox} y={ry - oy} width={stride * 1.05} height={stride * 1.05}
            fill="#3f6d55" opacity={on("landuse") && luShares ? 0.28 : 0.55} />
        ))}

        {/* 3 · land use — the plate that CHANGES over five centuries. Placement is
               the elevation-biased rank (§2.4), falling back to the plain patch
               hash when a cell's patch key somehow isn't in the map. */}
        {on("landuse") && luShares && cells.map(([rx, ry], i) => {
          const key = `${Math.floor(rx / patchSize)},${Math.floor(ry / patchSize)}`;
          const t = landUsePercentile.get(key) ?? cellHash(rx, ry, 1, patchSize);
          const c = LANDUSE[ditherClass(luShares, t)];
          return (
            <rect key={`u${i}`} x={rx - ox} y={ry - oy} width={stride * 1.05} height={stride * 1.05}
              fill={c.color} opacity={0.75} />
          );
        })}

        {/* 4 · tenure — who actually holds the land. Pure noise (not elevation-
               biased — who holds a field doesn't correlate with its altitude). */}
        {on("tenure") && tenure && cells.map(([rx, ry], i) => {
          const k = ditherClass([...tenure], cellHash(rx, ry, 7, patchSize));
          return (
            <rect key={`t${i}`} x={rx - ox} y={ry - oy} width={stride * 1.05} height={stride * 1.05}
              fill={tenureColor(k)} opacity={on("landuse") ? 0.45 : 0.7} />
          );
        })}

        {/* 2 · water — a REAL course when the world's river geometry is available
               (§2.3), clipped to this province's own raster mask. Falls back to a
               proportional scatter (honest about carrying no course) otherwise. */}
        {on("water") && riverSegments.length > 0 && riverSegments.map((seg, i) => (
          <polyline key={`wr${i}`} points={seg.map(([lx, ly]) => `${lx},${ly}`).join(" ")}
            fill="none" stroke="#4a90c4" strokeWidth={Math.max(0.6, stride * 0.35)}
            strokeLinecap="round" strokeLinejoin="round" opacity={0.85} />
        ))}
        {on("water") && riverSegments.length === 0 && waterFrac > 0 && cells.map(([rx, ry], i) =>
          cellHash(rx, ry, 23, stride) < waterFrac ? (
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
        {/* 6a · belt/surface goods (#9) — the GOODS plate. IMPORTANT: the model holds ONE
            quality for the WHOLE province (a belt good has no sub-province location), so we
            must NOT paint an area — that would invent a spatial layout the model doesn't
            hold (rule 17), and isolating a good would fill the entire province. Instead we
            place a few SYMBOLIC markers reading "produced here", quality-faded. Isolating a
            good shows a spread of just that good; showing all lays one marker per good like
            a legend on the map. Real per-cell locations only exist for DEPOSITS (plate 6b). */}
        {on("goods") && beltGoods.length > 0 && cells.length > 0 && (() => {
          const isolated = beltGoods.length === 1;
          const shown = beltGoods.slice(0, isolated ? 1 : 12);
          const fs = Math.max(6, Math.min(14, stride * 2.6));
          // A stable, well-spread cell for the k-th marker (Knuth multiplicative hash).
          const pickCell = (k: number) => cells[Math.floor((k * 2654435761) % cells.length)];
          const out: React.ReactNode[] = [];
          const marker = (g: { name: string; quality: number }, key: string, rx: number, ry: number) => {
            const emoji = GOOD_DEFS.find((d) => d.name === g.name)?.emoji ?? "•";
            return (
              <text key={key} x={rx - ox} y={ry - oy} fontSize={fs} textAnchor="middle"
                dominantBaseline="central" opacity={0.5 + 0.5 * Math.max(0, Math.min(1, g.quality))}
                style={{ paintOrder: "stroke", stroke: "#0a1620", strokeWidth: 0.6 }}>
                <title>{g.name} · quality {(g.quality * 100).toFixed(0)}%</title>{emoji}
              </text>
            );
          };
          if (isolated) {
            const g = shown[0];
            const marks = Math.max(3, Math.round(4 + 5 * g.quality));
            for (let i = 0; i < marks; i++) {
              const [rx, ry] = pickCell(i * 37 + 5);
              out.push(marker(g, `bg${i}`, rx, ry));
            }
          } else {
            shown.forEach((g, i) => {
              const [rx, ry] = pickCell(i * 53 + 9);
              out.push(marker(g, `bg${i}`, rx, ry));
            });
          }
          return out;
        })()}
        {/* 6b · deposits (#9) — the separate DEPOSITS plate: ore workings where they
            actually sit, coloured by good and sized by grade, richest reading loudest. */}
        {on("deposits") && deposits.map((d, i) => {
          const [lx, ly] = toLocal(d.x, d.y);
          const col = GOOD_DEFS.find((g) => g.name === d.good)?.color ?? "#56c8d8";
          const r = geo ? (0.9 + 1.6 * Math.min(1, Math.max(0, d.grade))) * Math.max(0.5, geo.stride * 0.5) : 2;
          return (
            <g key={`d${i}`} transform={`translate(${lx} ${ly})`} style={{ cursor: "pointer" }}
              onMouseEnter={(e) => setHover({
                x: e.nativeEvent.offsetX, y: e.nativeEvent.offsetY,
                title: `${d.good} deposit`,
                rows: [["grade", `${Math.round(d.grade * 100)}%`], ["depth", ["surface", "shallow", "deep", "flooded"][Math.max(0, Math.min(3, d.depth))]]],
              })}
              onMouseLeave={() => setHover(null)}>
              <circle r={r} fill={col} stroke="#0a1620" strokeWidth={0.5} opacity={0.9} />
              <circle r={r * 0.4} fill="#0a1620" opacity={0.55} />
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
  const keys: PlateKey[] = ["relief", "water", "landuse", "tenure", "holdings", "borders", "goods", "deposits"];
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
