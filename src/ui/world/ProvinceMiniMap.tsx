import { useMemo, useState } from "react";
import type { PBuilding, PSettlement, Province, ProvinceGoodMask, ProvinceLand, ProvinceLandSample, ProvinceLocalityDot, ProvinceTerrainCrop, RiverData } from "@types";
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

/** Darken/lighten a `#rrggbb` colour by a multiplicative factor (>1 lightens). */
function shadeColor(hex: string, factor: number): string {
  const c = [1, 3, 5].map((i) => Math.round(Math.max(0, Math.min(255, parseInt(hex.slice(i, i + 2), 16) * factor))));
  return `#${c.map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}

/** A single directional hillshade (NW light, one lamp — never a second/fill light;
 *  §8.21's own lesson is that a fill light washes out the exact shadows that carry
 *  the relief on a shaded DEM). Central-difference slope from the four orthogonal
 *  neighbours in the terrain grid, clamped to a modest readable swing so the plate
 *  still reads as height-tinted land, not a raw shading study. */
function hillshadeFactor(
  terrain: ProvinceTerrainCrop, r: number, c: number,
): number {
  const { cols, rows, elevation, land } = terrain;
  const at = (rr: number, cc: number): number => {
    const rc = Math.max(0, Math.min(rows - 1, rr)), cc2 = Math.max(0, Math.min(cols - 1, cc));
    const i = rc * cols + cc2;
    return land[i] === 1 ? elevation[i] : elevation[r * cols + c];
  };
  const dx = at(r, c + 1) - at(r, c - 1);
  const dy = at(r + 1, c) - at(r - 1, c);
  // NW light: brighten where the surface faces north/west (negative dx/dy slope
  // toward the light), darken the opposite (SE-facing) slope.
  const light = -(dx + dy) * 6.0;
  return Math.max(0.72, Math.min(1.28, 1.0 + light));
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
  // F5 (CLAUDE.md §4 step 7a + §7 (ports/junctions, shipped) slice 2) merged the old separate
  // "goods"/"quality" toggles into one plate — coverage AND the belt's own absolute
  // value shaded into it are the same layer now, so there is nothing left to toggle
  // independently. `deposits` stays its own plate: different geology, different symbol.
  goods: "goods", deposits: "deposits",
};
export const DEFAULT_PLATES: PlateKey[] = ["relief", "water", "holdings", "borders", "goods"];

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

/** The equatorial circumference the whole grid spans — the same figure the province
 *  partition and the biome layer use to turn km into cells. Lets a locality's
 *  `radius_km` become plate units without needing the world's cell size passed in:
 *  `local = radius_km · raster.w / KM_EQUATOR`. */
const KM_EQUATOR = 40075;
/** D4 · how far outside the province footprint the viewBox may widen to show an
 *  offshore locality, in raster cells. Bounded so a locality that landed across the
 *  X seam (this component has never unwrapped X) cannot blow the plate open. */
const MAX_SEA_PAD = 14;
/** The belt byte at/above which a cell counts as covered — mirrors the renderer's own
 *  `COVERAGE_MIN_U8` (`query_commands::overlays`) so the plate and the map agree on
 *  where a belt begins. */
const GOODS_COVERAGE_MIN = 5;

export function ProvinceMiniMap({
  province, raster, settlements, buildings,
  land, sample, plates = DEFAULT_PLATES, width = 240, riverCells,
  terrain, rivers, deposits = [], localities = [], goodMasks = [],
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
  /** #9 · the province's surface/belt goods (name + quality). Retained for the goods
   *  LEGEND upstream; the plate itself now draws belt areas from `goodMasks`, not from
   *  this per-province summary. */
  beltGoods?: { name: string; quality: number; marine?: boolean }[];
  /** The belt COVERAGE + QUALITY for each shown good, sampled to this province's raster
   *  (`province_good_belt_masks`). This is what lets the plate draw goods as AREAS +
   *  a quality wash — the same reading the main map gives — on ANY world, since it
   *  reads the goods tile column rather than needing per-locality positions. */
  goodMasks?: ProvinceGoodMask[];
  /** CLAUDE.md §8.19 (goods localities, shipped) Slice 6 (F3 · D4) · the REAL terroir localities inside
   *  (or, for a marine good, in the water off) this province, at their own cell
   *  coordinates. This is the data F3 said the "squares of goods" feature was blocked
   *  on: unlike land use and tenure — which are SHARES with no spatial extent and so
   *  must dither (rule 17) — a locality genuinely has a position, so it is drawn at
   *  it. A `sea` locality is an annotation in the adjacent water and confers NO
   *  maritime territory (D4). */
  localities?: ProvinceLocalityDot[];
}) {
  const [hover, setHover] = useState<Hover | null>(null);

  // Extract the province's footprint from the raster → an SVG viewBox. To keep the
  // SHAPE (and its proportions) faithful for a small province as well as a huge one,
  // the sample stride is sized to THIS province's bounding box, not the whole map:
  // pass 1 finds the bbox coarsely, pass 2 re-samples it at a per-province stride so
  // every province renders at a similar ~130-cell fidelity instead of a coarse blob.
  // The offshore (marine) localities' world positions, as a memo with a stable
  // identity, so the frame below widens for them without recomputing every render.
  const seaAnchors = useMemo(
    () => localities.filter((l) => l.sea).map((l) => [l.x, l.y] as [number, number]),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [localities.filter((l) => l.sea).map((l) => `${l.x},${l.y}`).join("|")],
  );

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
    let ox = minx - pad, oy = miny - pad;
    let vw = bw + stride + 2 * pad, vh = bh + stride + 2 * pad;
    // D4 · widen the FRAME (never the footprint) so an offshore locality attached to
    // this coast is actually on the plate. `cells`/`edge`/`stride` are untouched, so
    // the province's own shape, mosaic fidelity and border line are unaffected — this
    // only decides how much water is visible around it.
    let sx0 = ox, sy0 = oy, sx1 = ox + vw, sy1 = oy + vh;
    for (const l of seaAnchors) {
      const rx = (l[0] * w) / gridW, ry = (l[1] * h) / gridH;
      sx0 = Math.min(sx0, rx - 1); sx1 = Math.max(sx1, rx + 1);
      sy0 = Math.min(sy0, ry - 1); sy1 = Math.max(sy1, ry + 1);
    }
    sx0 = Math.max(sx0, ox - MAX_SEA_PAD); sx1 = Math.min(sx1, ox + vw + MAX_SEA_PAD);
    sy0 = Math.max(sy0, oy - MAX_SEA_PAD); sy1 = Math.min(sy1, oy + vh + MAX_SEA_PAD);
    ox = sx0; oy = sy0; vw = sx1 - sx0; vh = sy1 - sy0;
    // world-cell → raster-cell → local viewBox coords
    const toLocal = (x: number, y: number): [number, number] => [
      (x * w) / gridW - ox, (y * h) / gridH - oy,
    ];
    return { cells, edge, ox, oy, vw, vh, toLocal, stride };
    // `seaAnchors` is a stable string key of the offshore positions, so the frame is
    // recomputed only when they actually move (not on every parent render).
  }, [raster, province.id, seaAnchors]);

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
        const isLand = terrain.land[i] === 1;
        const base = reliefColor(terrain.elevation[i], isLand);
        // Real terrain, not just a height tint: a single NW hillshade (§8.21's own
        // "one lamp, never a fill light" discipline) so ridges and valleys actually
        // read, the same idiom the main map's relief_at/AO uses. Sea stays flat —
        // this crop carries no bathymetry, only a land/sea flag + elevation.
        const color = isLand ? shadeColor(base, hillshadeFactor(terrain, r, c)) : base;
        out.push({ lx, ly, color });
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

        {/* 6a · GOODS — coverage AND quality as ONE plate (F5 / slice 2 merged the old
               separate toggles: the belt's own absolute value already carries "can it
               grow here" at v=0 and "is it fine here" at v>0, so shading by it is
               strictly more informative than a flat coverage fill on its own).
               F1 / slice 1: sampled at the SAME world-cell fidelity the relief crop
               uses (`m.ox/oy/stride/cols/rows`), not the coarse province raster, so a
               belt's edge reads against the real coastline under it instead of a
               24×-coarser block grid. Drawn UNDER the borders/holdings so the survey
               marks stay legible on top. */}
        {on("goods") && goodMasks.map((m) => {
          const col = GOOD_DEFS.find((d) => d.name === m.good)?.color ?? "#56c8d8";
          // THREE WAYS THIS LAYER USED TO DISAPPEAR WITHOUT SAYING SO, all fixed
          // here — together they are the reported "province no longer shows areas
          // of goods, only small dots":
          //   · it was gated on `geo` (the terrain crop). A crop that has not
          //     arrived, or a province the crop query declines, dropped every belt
          //     area while the locality cores below still drew — dots, no areas.
          //     `toLocal` works without it, so the gate bought nothing.
          //   · `rectSize` fell back to **0** when `raster` was absent, so every
          //     cell drew as a 0×0 rect: present in the DOM, invisible on screen.
          //   · a zero/NaN `raster.gridW` produced the same, via a divide.
          // The fallback is now the plate's own cell size, which is the honest
          // reading when the raster's scale is unknown.
          const scale = raster && raster.gridW > 0 && raster.gridH > 0
            ? Math.max((m.stride * raster.w) / raster.gridW, (m.stride * raster.h) / raster.gridH)
            : 0;
          const rectSize = Number.isFinite(scale) && scale > 0.05 ? scale : stride;
          const pts: { lx: number; ly: number; v: number }[] = [];
          for (let r = 0; r < m.rows; r++) {
            for (let c = 0; c < m.cols; c++) {
              const v = m.q[r * m.cols + c];
              if (v < GOODS_COVERAGE_MIN) continue;
              const wx = m.ox + c * m.stride, wy = m.oy + r * m.stride;
              // `geo.toLocal` when the crop is loaded (it matches the relief plate's
              // own sampling exactly); the plate's own `toLocal` otherwise, which is
              // the same projection at slightly coarser fidelity — a belt drawn a
              // little coarsely beats a belt not drawn at all.
              const [lx, ly] = geo ? geo.toLocal(wx, wy) : toLocal(wx, wy);
              pts.push({ lx, ly, v });
            }
          }
          return (
            <g key={`gc-${m.good}`}>
              {pts.map((p, i) => (
                <rect key={`gc${i}`} x={p.lx - rectSize / 2} y={p.ly - rectSize / 2}
                  width={rectSize * 1.05} height={rectSize * 1.05}
                  fill={col} opacity={0.18 + 0.55 * (p.v / 255)} />
              ))}
            </g>
          );
        })}

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
        {/* 6a · the GOODS plate, Slice 6 (F3 · D4). REAL locality squares, at the
            cells the generator actually placed them on.

            This is what F3 said the feature was blocked on. Land use and tenure are
            SHARES with no spatial extent, so they must dither (rule 17) — but a
            locality is not a share: `GoodLocality.x/y` is a real cell and
            `radius_km` a real span, so both are drawn as they are. The square is
            CLIPPED to the province footprint, which is also what keeps a 900 km
            staple region (D6's chernozem case) honest: it tints as much of the
            province as it genuinely covers and no more.

            A MARINE locality is drawn in the adjacent water instead, dashed and
            outside the clip — an annotation, never territory. The province gains no
            maritime extent from it (D4): nothing here or downstream counts it toward
            land use, tenure, the harvest or revenue.

            Opacity carries `grade`; hue is the good's own from GOOD_DEFS, the same
            colour language the rest of the app already uses for a good. */}
        {on("goods") && localities.length > 0 && raster && (() => {
          const clipId = `pmm-prov-${province.id}`;
          // F5 (slice 2) · the true-to-scale LAND square is gone. The size ladder is
          // in km (a staple region is 900 km, CLAUDE.md §8.19) while a
          // province is 200-400 km across, so a land square always covered the whole
          // plate and the province read as one flat block. Every land locality is now
          // a CORE marker at its real cell, plus its name — the same reduction slice 6
          // already applied wherever a belt mask happened to be present; F1's fix
          // (masks now sample at real world-cell fidelity) makes that the honest
          // reading everywhere, not just when a mask query happened to return one.
          // A SEA one has no province footprint to clip to (it is outside the
          // province by definition, D4), so it stays a small dashed annotation,
          // capped well below its true span — the real span is stated in the tooltip.
          const half = (l: ProvinceLocalityDot) => {
            const s = Math.max(stride * 0.7, (l.radius_km * raster.w) / KM_EQUATOR);
            return Math.min(s, stride * 4);
          };
          const colorOf = (good: string) =>
            GOOD_DEFS.find((d) => d.name === good)?.color ?? "#56c8d8";
          // WHICH GOODS ACTUALLY GOT AN AREA DRAWN ABOVE. Reducing a locality to a
          // small core marker is only honest when plate 6a really drew the belt
          // underneath it; the reduction was made UNCONDITIONAL on the premise that
          // it always would, and when the mask is absent — a deposit good (they are
          // filtered out of the mask fetch by design), a good whose province belt
          // sits at the absent floor, an older world, or a fetch that has not landed
          // — the result is a bare dot standing for a whole region, with no area at
          // all. That is the regression this restores: no mask ⇒ draw the square.
          const covered = new Set(goodMasks
            .filter((m) => m.q.some((v) => v >= GOODS_COVERAGE_MIN))
            .map((m) => m.good));
          const sea = localities.filter((l) => l.sea);
          // Cores: the locality's real position, drawn on top of plate 6a's belt
          // area. Small, fixed-size and unclipped-by-scale, so it reads as a survey
          // mark on the belt rather than as a second area.
          const cores = localities.filter((l) => !l.sea);
          const square = (l: ProvinceLocalityDot, key: string, dashed: boolean) => {
            const [lx, ly] = toLocal(l.x, l.y);
            const s = half(l);
            const g = Math.max(0, Math.min(1, l.grade));
            return (
              <rect key={key} x={lx - s} y={ly - s} width={s * 2} height={s * 2}
                fill={colorOf(l.good)} opacity={0.22 + 0.5 * g}
                stroke={colorOf(l.good)} strokeWidth={stride * 0.25}
                strokeDasharray={dashed ? `${stride * 0.8} ${stride * 0.6}` : undefined}
                strokeOpacity={0.85} style={{ cursor: "pointer" }}
                onMouseEnter={(e) => setHover({
                  x: e.nativeEvent.offsetX, y: e.nativeEvent.offsetY,
                  title: `${l.name ? `${l.name} — ` : ""}${l.good}`,
                  rows: [
                    ["grade", `${Math.round(l.grade * 100)}%`],
                    ["span", `${Math.round(l.radius_km)} km`],
                    ["extent", ["a pocket", "small", "broad", "a great homeland"][Math.max(0, Math.min(3, l.extent))]],
                    ...(l.river_fed ? [["watered", "river-fed"] as [string, string]] : []),
                    ...(l.sea ? [["note", "offshore — not province territory"] as [string, string]] : []),
                  ],
                })}
                onMouseLeave={() => setHover(null)} />
            );
          };
          return (
            <g key="loc">
              {/* The footprint itself, as the clip for every LAND locality. */}
              <clipPath id={clipId}>
                {cells.map(([rx, ry], i) => (
                  <rect key={`c${i}`} x={rx - ox} y={ry - oy} width={stride * 1.05} height={stride * 1.05} />
                ))}
              </clipPath>
              <g clipPath={`url(#${clipId})`}>
                {/* LAND localities never draw the km-scale square (a real user
                    report: "squares appear on every good there is"). Plate 6a's
                    belt-mask layer just above already draws the real coverage
                    AREA at full cell fidelity — that IS the province-sized goods
                    layer the square fallback was invented to stand in for when a
                    mask hadn't arrived. A square drawn on top of that duplicated
                    the reading in a coarser, less honest shape. What's kept below
                    is the small diamond CORE marker — the "explicit zone, more
                    saturated" spot request — at the locality's real cell,
                    regardless of whether a belt mask exists for it. */}
                {cores.map((l, i) => {
                  const [lx, ly] = toLocal(l.x, l.y);
                  const r = stride * 0.9;
                  const g = Math.max(0, Math.min(1, l.grade));
                  return (
                    <g key={`lc${i}`} style={{ cursor: "pointer" }}
                      onMouseEnter={(e) => setHover({
                        x: e.nativeEvent.offsetX, y: e.nativeEvent.offsetY,
                        title: `${l.name ? `${l.name} — ` : ""}${l.good}`,
                        rows: [
                          ["grade", `${Math.round(l.grade * 100)}%`],
                          ["span", `${Math.round(l.radius_km)} km`],
                          ["extent", ["a pocket", "small", "broad", "a great homeland"][Math.max(0, Math.min(3, l.extent))]],
                          ...(l.river_fed ? [["watered", "river-fed"] as [string, string]] : []),
                          [covered.has(l.good) ? "note" : "shown as",
                           covered.has(l.good) ? "core of the belt drawn above"
                                               : "its own area — no belt mask for this good"] as [string, string],
                        ],
                      })}
                      onMouseLeave={() => setHover(null)}>
                      {/* A diamond, so a locality core never reads as an ore
                          working (a circle, plate 6b) or a belt cell (a square). */}
                      <path d={`M ${lx} ${ly - r} L ${lx + r} ${ly} L ${lx} ${ly + r} L ${lx - r} ${ly} Z`}
                        fill={colorOf(l.good)} opacity={0.55 + 0.45 * g}
                        stroke="#0a1620" strokeWidth={Math.max(0.4, stride * 0.18)} />
                    </g>
                  );
                })}
              </g>
              {/* Unclipped, dashed: the water off this coast (D4). */}
              {sea.map((l, i) => square(l, `ls${i}`, true))}
            </g>
          );
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
  const keys: PlateKey[] = ["relief", "water", "holdings", "borders", "goods", "deposits"];
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
