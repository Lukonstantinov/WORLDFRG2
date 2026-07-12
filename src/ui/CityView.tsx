import { useEffect, useRef, useState } from "react";
import type { HubDetail, BuildingInfo } from "../types";

// ── Isometric city view ─────────────────────────────────────────────────────
// Every settlement generates a bird's-eye ISOMETRIC plan of itself, deterministic
// from its name: land is partitioned into QUARTERS each owned by a faction (a
// resident house / the civic commons / a diaspora fondaco) and washed in that
// owner's heraldic colour; the city's real buildings are placed as marked
// LANDMARKS; coastal cities get water + a harbour. Everything recolours live with
// the campaign (owners come straight from `detail.buildings`).
//
// The buildings are drawn here as procedural iso BLOCKS. Each block is emitted
// through `drawTile`, the single sprite-slot: swapping in the Kenney CC0 iso
// tiles later means only replacing that one function — the generator + layout are
// unchanged.

const CIVIC = "#7a8aa0";
// Tile size is DYNAMIC: the plan is fit to a target width, so a small town gets
// chunky tiles and a metropolis packs smaller ones — both fill the panel nicely.
const REF_TW = 38;      // reference tile width the landmark heights are tuned to
const TARGET_W = 320;   // target on-screen plan width (px) to fit the panel
const TW_MIN = 16, TW_MAX = 48;
/** Dynamic iso tile dims + height scale for a grid of `N` tiles across. */
function tileDims(N: number) {
  const tw = Math.max(TW_MIN, Math.min(TW_MAX, Math.round(TARGET_W / (N + 1))));
  const th = Math.max(8, Math.round(tw / 2));
  return { tw, th, hs: tw / REF_TW }; // hs scales building heights with tile size
}

// ── PNG SPRITE PACK support ─────────────────────────────────────────────────
// Drop an isometric building pack into `public/city-sprites/` as one PNG per
// building type (see the map below). When a sprite is present it is blitted in
// place of the procedural iso block; a missing sprite falls back to the drawn
// building, so the view always renders. Owner colour keeps reading via the ground
// wash + the heraldic flag, so fixed-palette sprites still show who controls what.
const SPRITE_BASE = "/city-sprites/"; // public/city-sprites/<stem>.svg|png (like /fish/)
const SPRITE_EXTS = [".svg", ".png"];  // shipped templates are SVG; a PNG pack also works
/** Building label → sprite file stem in public/city-sprites/<stem>.png.
 *  Edit these to match the filenames in your pack. */
const SPRITE_MAP: Record<string, string> = {
  Guildhall: "guildhall", Workshop: "workshop", Granary: "granary", Warehouse: "warehouse",
  Shipyard: "shipyard", Fondaco: "fondaco", Cathedral: "cathedral", Temple: "temple",
  Citadel: "citadel", Palace: "palace", "Council Hall": "council_hall", Mint: "mint",
  Bank: "bank", Harbor: "harbor", house: "house",
};
type SpriteState = HTMLImageElement | "loading" | "error";
const spriteCache = new Map<string, SpriteState>();
/** Return a ready sprite image, or null while it loads / if it's absent. Calls
 *  `onReady` once when a fresh image finishes loading so the canvas can redraw. */
function loadSprite(stem: string, onReady: () => void): HTMLImageElement | null {
  const cur = spriteCache.get(stem);
  if (cur instanceof HTMLImageElement) return cur;
  if (cur === "loading" || cur === "error") return null;
  spriteCache.set(stem, "loading");
  const img = new Image();
  let ext = 0;
  img.onload = () => { spriteCache.set(stem, img); onReady(); };
  img.onerror = () => {
    ext += 1;
    if (ext < SPRITE_EXTS.length) { img.src = `${SPRITE_BASE}${stem}${SPRITE_EXTS[ext]}`; }
    else { spriteCache.set(stem, "error"); }
  };
  img.src = `${SPRITE_BASE}${stem}${SPRITE_EXTS[0]}`;
  return null;
}

/** Per-type landmark height → each building reads as a distinct silhouette
 *  (a guildhall towers, a granary squats, a shipyard hugs the shore). */
function landmarkHeight(label: string): number {
  const H: Record<string, number> = {
    Cathedral: 42, Temple: 42, Citadel: 36, Palace: 36, Guildhall: 34,
    "Council Hall": 30, Mint: 28, Fondaco: 27, Workshop: 22, Bank: 24,
    Warehouse: 17, Granary: 15, Harbor: 13, Shipyard: 13,
  };
  return H[label] ?? 24;
}

function hstr(s: string): number {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619) >>> 0; }
  return h >>> 0;
}
function mkRng(seed: number) {
  let s = (seed || 1) >>> 0;
  return () => { s = (Math.imul(s, 1664525) + 1013904223) >>> 0; return s / 4294967296; };
}
function toRgb(hex: string): [number, number, number] {
  const h = (hex || CIVIC).replace("#", "");
  return [parseInt(h.slice(0, 2), 16) || 122, parseInt(h.slice(2, 4), 16) || 138, parseInt(h.slice(4, 6), 16) || 160];
}
function shade([r, g, b]: [number, number, number], f: number): string {
  const c = (v: number) => Math.max(0, Math.min(255, Math.round(v * f)));
  return `rgb(${c(r)},${c(g)},${c(b)})`;
}
function mix(a: [number, number, number], b: [number, number, number], t: number): [number, number, number] {
  return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
}

interface Ward { ci: number; cj: number; owner: string; color: string; label: string }
interface Cell { i: number; j: number; water: boolean; ward: number; building?: BuildingInfo; height: number }

const QUARTER_LABELS = ["Patrician", "Merchant", "Artisan", "Harbour", "Temple", "Commons"];

/** Build the deterministic plan: grid size from population, quarters from the
 *  city's factions, buildings dropped into their owner's quarter. */
function generate(detail: HubDetail) {
  const seed = hstr(detail.name || "city");
  const rand = mkRng(seed);
  const pop = Math.max(200, detail.population || 500);
  // Grid scales with population (a hamlet ~6, a metropolis ~13 across).
  const N = Math.max(6, Math.min(13, Math.round(6 + Math.log10(pop / 400) * 2.4)));
  const coastal = !!detail.coastal;

  // Factions present = the distinct owners of the city's buildings (+ civic floor).
  const owners = new Map<string, string>(); // name -> colour
  owners.set("Civic", CIVIC);
  for (const b of detail.buildings || []) owners.set(b.owner, b.color);
  const factions = [...owners.entries()];
  // Fewer, chunkier quarters → each coloured district reads as a large area.
  const K = Math.max(2, Math.min(4, factions.length));

  // Ward seeds — one per faction, scattered; each carries a quarter label.
  const wards: Ward[] = [];
  for (let k = 0; k < K; k++) {
    const [owner, color] = factions[k % factions.length];
    wards.push({
      ci: Math.floor(rand() * N), cj: Math.floor(rand() * N),
      owner, color, label: QUARTER_LABELS[k % QUARTER_LABELS.length],
    });
  }

  // Water band on one coastal edge.
  const waterEdge = coastal ? Math.floor(rand() * 4) : -1;
  const isWater = (i: number, j: number) => {
    if (waterEdge < 0) return false;
    if (waterEdge === 0) return j >= N - 1;
    if (waterEdge === 1) return i >= N - 1;
    if (waterEdge === 2) return j <= 0;
    return i <= 0;
  };

  // Assign every cell to its nearest ward (Voronoi → organic coloured quarters).
  const cells: Cell[] = [];
  for (let j = 0; j < N; j++) {
    for (let i = 0; i < N; i++) {
      let best = 0, bd = Infinity;
      for (let k = 0; k < wards.length; k++) {
        const d = (wards[k].ci - i) ** 2 + (wards[k].cj - j) ** 2;
        if (d < bd) { bd = d; best = k; }
      }
      cells.push({ i, j, water: isWater(i, j), ward: best, height: 0 });
    }
  }
  const at = (i: number, j: number) => cells[j * N + i];

  // Place LANDMARK buildings — each into a land tile of its owner's quarter,
  // preferring tiles near that ward's seed; harbour goes on the coast.
  const land = cells.filter((c) => !c.water);
  const used = new Set<number>();
  const buildings = detail.buildings || [];
  for (const b of buildings) {
    const wantHarbour = b.label === "Shipyard" || b.label === "Fondaco";
    // Score each free tile ONCE (lower = better): same-owner quarter preferred,
    // harbour buildings pulled to the coast, with a stable per-tile jitter.
    let pick: Cell | undefined; let bestScore = Infinity;
    for (const c of land) {
      if (used.has(c.j * N + c.i)) continue;
      const jitter = ((hstr(b.label + c.i + "," + c.j) >>> 0) % 1000) / 1000 * 0.9;
      const score = (wards[c.ward].owner === b.owner ? 0 : 5)
        + (wantHarbour ? nearWater(c, isWater, N) : 0) + jitter;
      if (score < bestScore) { bestScore = score; pick = c; }
    }
    if (pick) { used.add(pick.j * N + pick.i); pick.building = b; pick.height = landmarkHeight(b.label); }
  }

  // Fill the rest of the land with common houses — kept LOW (well below any
  // landmark) so the marked buildings clearly dominate.
  for (const c of land) {
    if (c.building) continue;
    if (rand() < 0.24) continue; // gaps → streets/yards
    c.height = 4 + Math.floor(rand() * 5);
  }

  const walled = pop > 12000;
  return { N, cells, wards, at, walled, waterEdge };
}

function nearWater(c: Cell, isWater: (i: number, j: number) => boolean, N: number): number {
  for (const [di, dj] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
    const ni = c.i + di, nj = c.j + dj;
    if (ni < 0 || nj < 0 || ni >= N || nj >= N || isWater(ni, nj)) return -2;
  }
  return 0;
}

export function CityView({ detail }: { detail: HubDetail }) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [hover, setHover] = useState<BuildingInfo | null>(null);
  const [spriteTick, setSpriteTick] = useState(0); // bumped when a pack sprite loads
  const bumpRef = useRef(() => setSpriteTick((t) => t + 1));
  const planRef = useRef<ReturnType<typeof generate> | null>(null);
  const geomRef = useRef<{ ox: number; oy: number; tw: number; th: number } | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !detail) return;
    const plan = generate(detail);
    planRef.current = plan;
    const { N, cells, wards } = plan;
    const { tw, th, hs } = tileDims(N); // ← dynamic with city size

    const dpr = Math.min(2, window.devicePixelRatio || 1);
    const cssW = (N + 1) * tw;
    const cssH = (N + 1) * th + Math.round(46 * hs);
    canvas.width = cssW * dpr; canvas.height = cssH * dpr;
    canvas.style.width = cssW + "px"; canvas.style.height = cssH + "px";
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cssW, cssH);

    const ox = cssW / 2;
    const oy = Math.round(28 * hs) + 4;
    geomRef.current = { ox, oy, tw, th };
    const iso = (i: number, j: number): [number, number] => [ox + (i - j) * tw / 2, oy + (i + j) * th / 2];

    const getImg = (stem: string) => loadSprite(stem, bumpRef.current);
    // Painter's order: far tiles (small i+j) first.
    const order = [...cells].sort((a, b) => (a.i + a.j) - (b.i + b.j));
    for (const c of order) {
      const [cx, cy] = iso(c.i, c.j);
      const wardCol = toRgb(wards[c.ward].color);
      if (c.water) { drawGround(ctx, cx, cy, tw, th, "#1c3a4a", "#173040"); continue; }
      // Ground washed strongly toward the ward owner's colour → bold quarters.
      const g = mix([58, 74, 60], wardCol, 0.44);
      drawGround(ctx, cx, cy, tw, th, shade(g, 1.0), shade(g, 0.78));
      if (c.height > 0) drawTile(ctx, cx, cy, c.height * hs, tw, th, wardCol, c.building, getImg);
    }
  }, [detail, spriteTick]);

  const onMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const plan = planRef.current, geom = geomRef.current, canvas = canvasRef.current;
    if (!plan || !geom || !canvas) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left, my = e.clientY - rect.top;
    // Inverse iso (approximate to the tile under the cursor's base).
    const a = (mx - geom.ox) / (geom.tw / 2), b = (my - geom.oy) / (geom.th / 2);
    const i = Math.round((a + b) / 2), j = Math.round((b - a) / 2);
    if (i < 0 || j < 0 || i >= plan.N || j >= plan.N) { setHover(null); return; }
    setHover(plan.at(i, j).building ?? null);
  };

  // Quarter legend from the plan.
  const wards = planRef.current?.wards ?? [];
  const seen = new Set<string>();
  const legend = wards.filter((w) => { if (seen.has(w.owner)) return false; seen.add(w.owner); return true; });

  return (
    <div>
      <div style={{ overflowX: "auto", background: "#0a121c", border: "1px solid #24405e", borderRadius: 8, padding: "4px 0" }}>
        <canvas ref={canvasRef} onMouseMove={onMove} onMouseLeave={() => setHover(null)}
          style={{ display: "block", margin: "0 auto", imageRendering: "auto" }} />
      </div>
      {hover && (
        <div style={{ fontSize: 11, color: "#cfe0f4", marginTop: 4, background: "#0a121c",
          border: "1px solid #24405e", borderRadius: 6, padding: "5px 8px" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span style={{ fontSize: 13 }}>{hover.emoji}</span>
            <span style={{ color: "#cdbb88", fontWeight: 700 }}>{hover.label}</span>
            <span style={{ flex: 1 }} />
            <span style={{ width: 9, height: 9, borderRadius: 2, background: hover.color, display: "inline-block" }} />
            <span style={{ color: hover.color }}>{hover.owner}</span>
          </div>
          {BUILDING_INFO[hover.label] && (
            <div style={{ color: "#9ab0c8", marginTop: 2, lineHeight: 1.4 }}>{BUILDING_INFO[hover.label]}</div>
          )}
          <div style={{ color: "#7fbf9a", marginTop: 2 }}>{hover.effect}</div>
        </div>
      )}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, fontSize: 9, color: "#9ab0c8", marginTop: 5 }}>
        {legend.map((w) => (
          <span key={w.owner} style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
            <span style={{ width: 8, height: 8, borderRadius: 2, background: w.color, display: "inline-block" }} />
            {w.owner} <span style={{ color: "#6a86a6" }}>quarter</span>
          </span>
        ))}
      </div>
    </div>
  );
}

// ── Drawing primitives (the sprite-slot) ────────────────────────────────────
/** A single iso ground diamond. */
function drawGround(ctx: CanvasRenderingContext2D, cx: number, cy: number, tw: number, th: number, top: string, edge: string) {
  ctx.beginPath();
  ctx.moveTo(cx, cy - th / 2);
  ctx.lineTo(cx + tw / 2, cy);
  ctx.lineTo(cx, cy + th / 2);
  ctx.lineTo(cx - tw / 2, cy);
  ctx.closePath();
  ctx.fillStyle = top; ctx.fill();
  ctx.strokeStyle = edge; ctx.lineWidth = 0.5; ctx.stroke();
}

type Pt = [number, number];
function poly(ctx: CanvasRenderingContext2D, pts: Pt[], fill: string) {
  ctx.beginPath();
  ctx.moveTo(pts[0][0], pts[0][1]);
  for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i][0], pts[i][1]);
  ctx.closePath();
  ctx.fillStyle = fill; ctx.fill();
}
// Roof material palettes (silhouette + colour differ per building type).
const ROOF_TILE: [number, number, number] = [156, 91, 69];   // terracotta pitch
const ROOF_GOLD: [number, number, number] = [201, 162, 74];   // temple dome
const ROOF_STONE: [number, number, number] = [112, 118, 124]; // citadel battlement
const ROOF_SLATE: [number, number, number] = [92, 102, 112];  // flat storage roof
type Arche = "pitch" | "dome" | "crenel" | "flat";
function archetypeOf(label: string): Arche {
  if (["Cathedral", "Temple", "Mint", "Bank"].includes(label)) return "dome";
  if (["Citadel", "Palace", "Council Hall"].includes(label)) return "crenel";
  if (["Granary", "Warehouse", "Harbor", "Shipyard"].includes(label)) return "flat";
  return "pitch";
}

/** Draw ONE building: a PNG sprite from the pack if one is present, otherwise the
 *  procedural iso structure. Owner colour still reads via the ground wash + flag. */
function drawTile(ctx: CanvasRenderingContext2D, cx: number, cy: number, h: number,
  tw: number, th: number, wardCol: [number, number, number],
  building: BuildingInfo | undefined, getImg: (stem: string) => HTMLImageElement | null) {
  const stem = building ? SPRITE_MAP[building.label] : SPRITE_MAP.house;
  const img = stem ? getImg(stem) : null;
  if (img) { drawSprite(ctx, img, cx, cy, tw, th, building); return; }
  drawProcedural(ctx, cx, cy, h, tw, th, wardCol, building);
}

/** Blit a pack sprite anchored on the tile's front, sized to the tile. A landmark
 *  keeps its heraldic flag + emoji chip so ownership and identity still read. */
function drawSprite(ctx: CanvasRenderingContext2D, img: HTMLImageElement, cx: number, cy: number,
  tw: number, th: number, building?: BuildingInfo) {
  const scale = building ? 2.2 : 1.35;              // landmarks bigger than houses
  const w = tw * scale;
  const hgt = w * (img.height / Math.max(1, img.width));
  const dx = cx - w / 2;
  const dy = (cy + th / 2) - hgt + th * 0.12;       // sit the base on the tile front
  ctx.drawImage(img, dx, dy, w, hgt);
  if (building) {
    const topY = dy + hgt * 0.06;
    const poleH = Math.max(7, tw * 0.32), pw = Math.max(5, tw * 0.22);
    ctx.strokeStyle = "#1b2833"; ctx.lineWidth = 1.1;
    ctx.beginPath(); ctx.moveTo(cx, topY); ctx.lineTo(cx, topY - poleH); ctx.stroke();
    poly(ctx, [[cx, topY - poleH], [cx + pw, topY - poleH + 3], [cx, topY - poleH + 6]], building.color);
    const chipR = Math.max(6.5, Math.min(10, tw * 0.24));
    const fontPx = Math.max(9, Math.min(14, Math.round(tw * 0.34)));
    const ey = topY - poleH - chipR - 1;
    ctx.beginPath(); ctx.arc(cx, ey, chipR, 0, Math.PI * 2);
    ctx.fillStyle = "rgba(9,14,20,0.85)"; ctx.fill();
    ctx.strokeStyle = building.color; ctx.lineWidth = 1.2; ctx.stroke();
    ctx.font = `${fontPx}px system-ui, sans-serif`;
    ctx.textAlign = "center"; ctx.textBaseline = "middle";
    ctx.fillText(building.emoji || "🏛️", cx, ey + 0.5);
  }
}

/** The procedural iso building (fallback when no pack sprite is present) — walls in
 *  the owner's colour, a roof whose SHAPE reads the building type, and a flag. */
function drawProcedural(ctx: CanvasRenderingContext2D, cx: number, cy: number, h: number,
  tw: number, th: number, wardCol: [number, number, number], building?: BuildingInfo) {
  const isLandmark = !!building;
  const s = isLandmark ? 1.0 : 0.62;
  const base = isLandmark ? toRgb(building!.color) : mix([120, 116, 108], wardCol, 0.3);
  const leftC = shade(base, isLandmark ? 0.55 : 0.5);
  const rightC = shade(base, isLandmark ? 0.78 : 0.66);
  const hw = (tw / 2) * s, hh = (th / 2) * s;
  // Eave corners (roof base at wall height h) + ground corners.
  const T: Pt = [cx, cy - hh - h], R: Pt = [cx + hw, cy - h], B: Pt = [cx, cy + hh - h], L: Pt = [cx - hw, cy - h];
  const Rg: Pt = [cx + hw, cy], Bg: Pt = [cx, cy + hh], Lg: Pt = [cx - hw, cy];
  // Walls (the two front faces).
  poly(ctx, [L, B, Bg, Lg], leftC);
  poly(ctx, [R, B, Bg, Rg], rightC);

  const arche: Arche = isLandmark ? archetypeOf(building!.label) : "pitch";
  const rh = Math.max(5, tw * 0.42);
  let flagTopY = cy - hh - h; // where the flag pole roots (roof apex)
  if (arche === "pitch") {
    const A: Pt = [cx, cy - hh - h - rh];
    const rl = shade(ROOF_TILE, 0.78), rr = shade(ROOF_TILE, 1.0);
    poly(ctx, [A, T, L], rl); poly(ctx, [A, T, R], rr);   // back slopes
    poly(ctx, [A, L, B], rl); poly(ctx, [A, R, B], rr);   // front slopes
    flagTopY = A[1];
  } else if (arche === "flat") {
    poly(ctx, [T, R, B, L], shade(ROOF_SLATE, 1.0));
    poly(ctx, [T, R, B, L], "rgba(0,0,0,0)");
    ctx.strokeStyle = shade(ROOF_SLATE, 0.6); ctx.lineWidth = 1;
    ctx.beginPath(); ctx.moveTo(T[0], T[1]); ctx.lineTo(R[0], R[1]); ctx.lineTo(B[0], B[1]); ctx.lineTo(L[0], L[1]); ctx.closePath(); ctx.stroke();
  } else if (arche === "crenel") {
    poly(ctx, [T, R, B, L], shade(ROOF_STONE, 1.0));
    const m = Math.max(2.5, tw * 0.12), mh = rh * 0.55;
    for (const c of [L, B, R] as Pt[]) {
      const [x, y] = c;
      poly(ctx, [[x - m, y - mh], [x + m, y - mh], [x + m, y], [x - m, y]], shade(ROOF_STONE, 1.15)); // merlon face
      poly(ctx, [[x - m, y - mh], [x, y - mh - m * 0.5], [x + m, y - mh], [x, y - mh + m * 0.5]], shade(ROOF_STONE, 1.3)); // cap
    }
    flagTopY = cy - hh - h - rh * 0.55;
  } else { // dome
    poly(ctx, [T, R, B, L], shade(base, 0.9)); // drum
    ctx.beginPath();
    ctx.ellipse(cx, cy - h - hh * 0.1, hw * 0.82, hh + rh, 0, 0, Math.PI * 2);
    ctx.fillStyle = shade(ROOF_GOLD, 1.0); ctx.fill();
    ctx.strokeStyle = shade(ROOF_GOLD, 0.65); ctx.lineWidth = 1; ctx.stroke();
    flagTopY = cy - h - hh * 0.1 - (hh + rh);
  }

  if (isLandmark) {
    // Owner flag: a short pole + a pennant in the owner's heraldic colour.
    const poleH = Math.max(7, tw * 0.32);
    const pw = Math.max(5, tw * 0.22);
    ctx.strokeStyle = "#1b2833"; ctx.lineWidth = 1.1;
    ctx.beginPath(); ctx.moveTo(cx, flagTopY); ctx.lineTo(cx, flagTopY - poleH); ctx.stroke();
    poly(ctx, [[cx, flagTopY - poleH], [cx + pw, flagTopY - poleH + 3], [cx, flagTopY - poleH + 6]], building!.color);
    // Small emoji chip above the flag for at-a-glance identification.
    const chipR = Math.max(6.5, Math.min(10, tw * 0.24));
    const fontPx = Math.max(9, Math.min(14, Math.round(tw * 0.34)));
    const ey = flagTopY - poleH - chipR - 1;
    ctx.beginPath(); ctx.arc(cx, ey, chipR, 0, Math.PI * 2);
    ctx.fillStyle = "rgba(9,14,20,0.85)"; ctx.fill();
    ctx.strokeStyle = building!.color; ctx.lineWidth = 1.2; ctx.stroke();
    ctx.font = `${fontPx}px system-ui, sans-serif`;
    ctx.textAlign = "center"; ctx.textBaseline = "middle";
    ctx.fillText(building!.emoji || "🏛️", cx, ey + 0.5);
  }
}

/** One-line lore/role for each building type — shown on hover + in the ward grid. */
export const BUILDING_INFO: Record<string, string> = {
  Guildhall: "Seat of the merchant guild; lowers freight on goods leaving the city.",
  Workshop: "Artisans' works — raises the city's output of manufactured goods.",
  Granary: "Public grain store; buffers famine and lifts food output.",
  Warehouse: "Bonded storage that smooths supply and adds a little output.",
  Shipyard: "Builds and berths the resident house's ships.",
  Fondaco: "A foreign merchants' quarter and trade house — a diaspora enclave.",
  Cathedral: "The city's great sanctuary; draws pilgrims and steadies civic mood.",
  Temple: "A holy precinct; its festivals lift the people and draw the faithful.",
  Citadel: "The fortified seat of power; defends the city and resists takeover.",
  Palace: "The ruling house's grand residence and court.",
  "Council Hall": "Where the polis council sits and sets tariff, mint and law.",
  Mint: "Strikes the polis's own coin.",
  Bank: "A counting-house extending credit across the trade network.",
  Harbor: "Docks and quays working the city's sea trade.",
};
