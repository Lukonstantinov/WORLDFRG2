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

const TW = 30;   // iso tile width
const TH = 15;   // iso tile height (2:1 iso)
const CIVIC = "#7a8aa0";

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
  const K = Math.max(2, Math.min(6, factions.length));

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
    if (pick) { used.add(pick.j * N + pick.i); pick.building = b; pick.height = 15 + Math.min(14, buildings.length); }
  }

  // Fill the rest of the land with common houses (short blocks, jittered height).
  for (const c of land) {
    if (c.building) continue;
    if (rand() < 0.22) continue; // gaps → streets/yards
    c.height = 5 + Math.floor(rand() * 6);
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
  const planRef = useRef<ReturnType<typeof generate> | null>(null);
  const geomRef = useRef<{ ox: number; oy: number } | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !detail) return;
    const plan = generate(detail);
    planRef.current = plan;
    const { N, cells, wards } = plan;

    const dpr = Math.min(2, window.devicePixelRatio || 1);
    const cssW = (N + 1) * TW;
    const cssH = (N + 1) * TH + 40;
    canvas.width = cssW * dpr; canvas.height = cssH * dpr;
    canvas.style.width = cssW + "px"; canvas.style.height = cssH + "px";
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cssW, cssH);

    const ox = cssW / 2;
    const oy = 24;
    geomRef.current = { ox, oy };
    const iso = (i: number, j: number): [number, number] => [ox + (i - j) * TW / 2, oy + (i + j) * TH / 2];

    // Painter's order: far tiles (small i+j) first.
    const order = [...cells].sort((a, b) => (a.i + a.j) - (b.i + b.j));
    for (const c of order) {
      const [cx, cy] = iso(c.i, c.j);
      const wardCol = toRgb(wards[c.ward].color);
      if (c.water) { drawGround(ctx, cx, cy, "#1c3a4a", "#173040"); continue; }
      // Ground washed toward the ward owner's colour.
      const g = mix([58, 74, 60], wardCol, 0.28);
      drawGround(ctx, cx, cy, shade(g, 1.0), shade(g, 0.8));
      if (c.height > 0) drawTile(ctx, cx, cy, c.height, wardCol, c.building);
    }

    // Endpoint-of-nothing: draw the quarter legend beneath handled in JSX.
  }, [detail]);

  const onMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const plan = planRef.current, geom = geomRef.current, canvas = canvasRef.current;
    if (!plan || !geom || !canvas) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left, my = e.clientY - rect.top;
    // Inverse iso (approximate to the tile under the cursor's base).
    const a = (mx - geom.ox) / (TW / 2), b = (my - geom.oy) / (TH / 2);
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
        <div style={{ fontSize: 11, color: "#cfe0f4", marginTop: 4, display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ fontSize: 13 }}>{hover.emoji}</span>
          <span style={{ color: "#cdbb88", fontWeight: 700 }}>{hover.label}</span>
          <span style={{ width: 9, height: 9, borderRadius: 2, background: hover.color, display: "inline-block" }} />
          <span style={{ color: hover.color }}>{hover.owner}</span>
          <span style={{ color: "#7fbf9a" }}>· {hover.effect}</span>
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
function drawGround(ctx: CanvasRenderingContext2D, cx: number, cy: number, top: string, edge: string) {
  ctx.beginPath();
  ctx.moveTo(cx, cy - TH / 2);
  ctx.lineTo(cx + TW / 2, cy);
  ctx.lineTo(cx, cy + TH / 2);
  ctx.lineTo(cx - TW / 2, cy);
  ctx.closePath();
  ctx.fillStyle = top; ctx.fill();
  ctx.strokeStyle = edge; ctx.lineWidth = 0.5; ctx.stroke();
}

/** One building block (procedural iso cube). This is the SPRITE-SLOT: replace the
 *  body with a Kenney iso sprite blit and the rest of the system is unchanged. */
function drawTile(ctx: CanvasRenderingContext2D, cx: number, cy: number, h: number,
  wardCol: [number, number, number], building?: BuildingInfo) {
  const isLandmark = !!building;
  // Roof colour: a warm neutral for common houses; the owner's colour for landmarks.
  const base = isLandmark ? toRgb(building!.color) : mix([150, 140, 120], wardCol, 0.35);
  const topC = shade(base, isLandmark ? 1.05 : 0.95);
  const leftC = shade(base, 0.6);
  const rightC = shade(base, 0.78);
  const hw = TW / 2, hh = TH / 2;
  // top face
  ctx.beginPath();
  ctx.moveTo(cx, cy - hh - h);
  ctx.lineTo(cx + hw, cy - h);
  ctx.lineTo(cx, cy + hh - h);
  ctx.lineTo(cx - hw, cy - h);
  ctx.closePath();
  ctx.fillStyle = topC; ctx.fill();
  // left face
  ctx.beginPath();
  ctx.moveTo(cx - hw, cy - h);
  ctx.lineTo(cx, cy + hh - h);
  ctx.lineTo(cx, cy + hh);
  ctx.lineTo(cx - hw, cy);
  ctx.closePath();
  ctx.fillStyle = leftC; ctx.fill();
  // right face
  ctx.beginPath();
  ctx.moveTo(cx + hw, cy - h);
  ctx.lineTo(cx, cy + hh - h);
  ctx.lineTo(cx, cy + hh);
  ctx.lineTo(cx + hw, cy);
  ctx.closePath();
  ctx.fillStyle = rightC; ctx.fill();
  if (isLandmark) {
    // Owner banner (a thin outline in the owner colour) + the building's emoji.
    ctx.strokeStyle = building!.color; ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.moveTo(cx, cy - hh - h); ctx.lineTo(cx + hw, cy - h);
    ctx.lineTo(cx, cy + hh - h); ctx.lineTo(cx - hw, cy - h);
    ctx.closePath(); ctx.stroke();
    ctx.font = "11px system-ui, sans-serif";
    ctx.textAlign = "center"; ctx.textBaseline = "middle";
    ctx.fillText(building!.emoji || "🏛️", cx, cy - h - 1);
  }
}
