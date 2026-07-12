// Isometric city-building sprite generator (uniform-grid templates).
//
// Every building is drawn on the SAME cell: a fixed iso footprint diamond centred
// in a 128×220 canvas, base bottom-vertex at (64,190). The city view blits each
// sprite so that cell maps exactly onto one grid tile — so buildings are all the
// same footprint and tile the grid cleanly (no overlap, no mash-up). Only HEIGHT
// varies (a cathedral rises, a house squats), which is how you tell them apart.
//
// Walls are neutral stone/timber; the owning faction's colour reads via the
// panel's ground wash + the heraldic flag it draws at each roof. Re-run:
//   node scripts/gen_city_sprites.mjs
//
// The renderer (src/ui/CityView.tsx) shares the CELL constants + SPRITE_PEAK map;
// keep them in sync if you change geometry here.

import { mkdirSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const OUT = join(dirname(fileURLToPath(import.meta.url)), "..", "public", "city-sprites");
mkdirSync(OUT, { recursive: true });

// ── the CELL (shared with CityView) ─────────────────────────────────────────
const VB_W = 128, VB_H = 220;
const CX = 64, CY = 160;          // footprint centre
const FHW = 50, FHH = 25;         // building footprint half-extents (fills the cell)

const r = (n) => Math.round(n * 10) / 10;
const P = (x, y) => `${r(x)},${r(y)}`;
function shade(hex, f) {
  const h = hex.replace("#", "");
  const c = (i) => Math.max(0, Math.min(255, Math.round(parseInt(h.substr(i, 2), 16) * f)));
  return `#${[0, 2, 4].map((i) => c(i).toString(16).padStart(2, "0")).join("")}`;
}
const poly = (pts, fill, extra = "") => `<polygon points="${pts.map(([x, y]) => P(x, y)).join(" ")}" fill="${fill}"${extra}/>`;
const line = (a, b, stroke, w = 1.4) => `<line x1="${r(a[0])}" y1="${r(a[1])}" x2="${r(b[0])}" y2="${r(b[1])}" stroke="${stroke}" stroke-width="${w}" stroke-linecap="round"/>`;
const circle = (x, y, rad, fill, extra = "") => `<circle cx="${r(x)}" cy="${r(y)}" r="${r(rad)}" fill="${fill}"${extra}/>`;

// A box of wall-height `wh`, footprint scaled by sx,sy (1 = full cell), centred.
function corners(wh, sx = 1, sy = 1) {
  const hw = FHW * sx, hh = FHH * sy;
  return {
    L: [CX - hw, CY], T: [CX, CY - hh], R: [CX + hw, CY], B: [CX, CY + hh],
    L2: [CX - hw, CY - wh], T2: [CX, CY - hh - wh], R2: [CX + hw, CY - wh], B2: [CX, CY + hh - wh],
  };
}
const walls = (c, wall) => poly([c.L, c.B, c.B2, c.L2], shade(wall, 0.72)) + poly([c.R, c.B, c.B2, c.R2], shade(wall, 0.9));
const topFace = (c, fill) => poly([c.L2, c.T2, c.R2, c.B2], fill);
function hipRoof(c, rh, roof) {
  const A = [CX, c.T2[1] - rh];
  return poly([A, c.T2, c.L2], shade(roof, 0.72)) + poly([A, c.T2, c.R2], shade(roof, 0.9))
    + poly([A, c.L2, c.B2], shade(roof, 0.82)) + poly([A, c.R2, c.B2], shade(roof, 1.04));
}
function gableRoof(c, rh, roof) {
  const rl = [c.L2[0], c.L2[1] - rh], rr = [c.R2[0], c.R2[1] - rh];
  return poly([rl, rr, c.R2, c.L2], shade(roof, 0.72)) + poly([rl, rr, c.B2], shade(roof, 1.04))
    + poly([c.L2, rl, c.T2], shade(roof, 0.82)) + poly([c.R2, rr, c.T2], shade(roof, 0.92));
}
function crenel(c, roof) {
  let s = topFace(c, shade(roof, 1.0));
  const m = 6, mh = 8;
  for (const p of [c.L2, c.B2, c.R2]) {
    s += poly([[p[0] - m, p[1] - mh], [p[0] + m, p[1] - mh], [p[0] + m, p[1]], [p[0] - m, p[1]]], shade(roof, 1.14));
    s += poly([[p[0] - m, p[1] - mh], [p[0], p[1] - mh - 3], [p[0] + m, p[1] - mh], [p[0], p[1] - mh + 3]], shade(roof, 1.3));
  }
  return s;
}
// door on the lit (front-right) wall
function door(c, w = 11, hgt = 18, col = "#3c2c1e") {
  const mx = (c.R[0] + c.B[0]) / 2, my = (c.R[1] + c.B[1]) / 2;
  return poly([[mx - w / 2, my - 2], [mx + w / 2, my - 2 - w * 0.28], [mx + w / 2, my - hgt], [mx - w / 2, my - hgt + w * 0.28]], col);
}
function windows(c, n, wh, col = "#33465a") {
  let s = "";
  for (let i = 0; i < n; i++) {
    const t = (i + 1) / (n + 1);
    const x = c.B[0] + (c.R[0] - c.B[0]) * t, y = c.B[1] + (c.R[1] - c.B[1]) * t;
    const wy = y - wh * 0.5;
    s += poly([[x - 2.6, wy], [x + 2.6, wy - 1.4], [x + 2.6, wy - 8], [x - 2.6, wy - 6.6]], col);
  }
  return s;
}
// a column colonnade along the lit front edge
function colonnade(c, n, h, col = "#e0d6bc") {
  let s = "";
  for (let i = 0; i <= n; i++) {
    const t = i / n;
    const x = c.B[0] + (c.R[0] - c.B[0]) * t, y = c.B[1] + (c.R[1] - c.B[1]) * t;
    s += poly([[x - 2.2, y - 3], [x + 2.2, y - 4.1], [x + 2.2, y - h], [x - 2.2, y - h + 1.1]], col);
  }
  return s;
}

const STONE = "#cdbfa6", TIMBER = "#b8a582", WOOD = "#8a6a46", FORT = "#9aa0a6";
const TERRA = "#b0603f", SLATE = "#606b78", MARBLE = "#dcd2ba", GOLD = "#caa24a";
const svg = (inner) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${VB_W} ${VB_H}" width="${VB_W}" height="${VB_H}">${inner}</svg>\n`;

// ── templates (uniform footprint; only height/roof/features differ) ──────────
const B = {};

B.house = () => { const c = corners(30, 0.82, 0.82); return svg(walls(c, TIMBER) + hipRoof(c, 16, TERRA) + door(c, 8, 12)); };

B.guildhall = () => { const c = corners(52); return svg(walls(c, STONE) + hipRoof(c, 22, TERRA) + windows(c, 3, 52) + door(c, 12, 22, "#33241a")); };

B.workshop = () => { const c = corners(40); const ch = [CX + 22, c.R2[1] - 4];
  return svg(walls(c, WOOD) + gableRoof(c, 18, TERRA) + door(c, 15, 20, "#2a1c12")
    + poly([[ch[0] - 3, ch[1]], [ch[0] + 3, ch[1] - 1.5], [ch[0] + 3, ch[1] - 15], [ch[0] - 3, ch[1] - 13.5]], "#595046")
    + circle(ch[0] + 1, ch[1] - 19, 3.6, "#8a929a") + circle(ch[0] + 4, ch[1] - 24, 4.4, "#9aa2aa")); };

B.granary = () => { const c = corners(52, 0.82, 0.82); let v = "";
  for (let i = 0; i < 3; i++) v += poly([[CX - 1.8, c.B2[1] + 9 + i * 11], [CX + 1.8, c.B2[1] + 8 + i * 11], [CX + 1.8, c.B2[1] + 4 + i * 11], [CX - 1.8, c.B2[1] + 5 + i * 11]], "#3a2c1e");
  return svg(walls(c, "#c8b48c") + hipRoof(c, 20, "#a86b3e") + v + door(c, 9, 13)); };

B.warehouse = () => { const c = corners(38); return svg(walls(c, "#b7a888") + gableRoof(c, 15, SLATE) + door(c, 20, 22, "#241a12") + windows(c, 2, 38, "#2c3a48")); };

B.shipyard = () => { const c = corners(26, 0.9, 0.9);
  const slip = poly([[c.B[0] - 4, c.B[1] + 3], [c.R[0] - 2, c.R[1] + 4], [c.R[0] - 6, c.R[1] + 9], [c.B[0] - 8, c.B[1] + 8]], "#6b5942");
  const hull = poly([[CX + 10, CY + 20], [CX + 24, CY + 25], [CX + 19, CY + 29], [CX + 7, CY + 24]], "#4a3626");
  return svg(slip + walls(c, WOOD) + gableRoof(c, 12, "#7a5238") + door(c, 14, 15, "#221610") + hull
    + line([CX + 15, CY + 24], [CX + 15, CY + 8], "#3a2a1c", 1.4)); };

B.fondaco = () => { const c = corners(48); let arc = "";
  for (let i = 0; i < 3; i++) { const t = (i + 1) / 4; const x = c.B[0] + (c.R[0] - c.B[0]) * t, y = c.B[1] + (c.R[1] - c.B[1]) * t;
    arc += `<path d="M ${r(x - 4)} ${r(y - 2)} l 0 -8 q 4 -6 8 0 l 0 8 z" fill="#2e281e"/>`; }
  return svg(walls(c, "#c2b79a") + hipRoof(c, 16, "#a56a4a") + arc
    + poly([c.L2, c.T2, c.R2, c.B2], "none", ` stroke="#8a7a58" stroke-width="1.2"`)); };

B.cathedral = () => { const nave = corners(58, 1.0, 1.0);
  const tower = corners(128, 0.34, 0.38);            // central crossing tower
  const spireBase = tower.T2[1];                      // apex top of tower walls region
  const spire = poly([tower.L2, [CX, spireBase + FHH * 0.38 - 128 + 128], tower.R2, [CX, tower.T2[1] - 34]], shade("#7a4a5a", 0.95));
  const rose = circle(CX, CY - 34, 5.5, "#3a4a6a") + circle(CX, CY - 34, 5.5, "none", ` stroke="#caa24a" stroke-width="1.3"`);
  const portal = `<path d="M ${r((nave.R[0] + nave.B[0]) / 2 - 6)} ${r((nave.R[1] + nave.B[1]) / 2 - 2)} l 0 -13 q 6 -9 12 0 l 0 13 z" fill="#241820"/>`;
  const belfry = poly([[CX - 4, tower.T2[1] + 22], [CX + 4, tower.T2[1] + 24], [CX + 4, tower.T2[1] + 10], [CX - 4, tower.T2[1] + 8]], "#241a22");
  return svg(walls(nave, "#c6b59a") + gableRoof(nave, 22, "#7a4a5a") + rose + portal
    + walls(tower, "#cabb9e") + belfry + poly([tower.L2, [CX, tower.T2[1] - 34], tower.R2, [CX, tower.T2[1] + FHH * 0.38]], shade("#7a4a5a", 0.98))); };

B.temple = () => { const c = corners(40);
  const steps = poly([[c.L[0] - 4, c.L[1] + 4], [CX, c.B[1] + 8], [c.R[0] + 4, c.R[1] + 4], [CX, c.T[1]]], shade(MARBLE, 0.9));
  const entab = poly([c.L2, c.T2, c.R2, c.B2], MARBLE);
  const frieze = poly([c.B2, c.R2, [c.R2[0], c.R2[1] - 5], [c.B2[0], c.B2[1] - 5]], shade(MARBLE, 1.08));
  return svg(steps + walls(c, MARBLE) + colonnade(c, 4, 34) + frieze + entab + gableRoof(corners(46), 12, "#b8442f")); };

B.citadel = () => { const wall = corners(46);
  const keep = corners(118, 0.44, 0.48);
  return svg(walls(wall, FORT) + crenel(wall, FORT) + door(wall, 12, 20, "#2a2e33") + windows(wall, 3, 46, "#20262c")
    + walls(keep, shade(FORT, 1.06)) + crenel(keep, shade(FORT, 1.06))); };

B.palace = () => { const c = corners(48);
  const portico = poly([[CX - 9, c.B[1] - 2], [CX + 9, c.B[1] - 2], [CX + 9, c.B[1] - 24], [CX - 9, c.B[1] - 24]], MARBLE)
    + line([CX - 7, c.B[1] - 3], [CX - 7, c.B[1] - 22], "#c8bca0", 2.4) + line([CX + 7, c.B[1] - 3], [CX + 7, c.B[1] - 22], "#c8bca0", 2.4);
  return svg(walls(c, "#cdc0a4") + hipRoof(c, 20, TERRA) + windows(c, 4, 48) + portico + door(c, 9, 16, "#33241a")); };

B.council_hall = () => { const c = corners(46);
  const belfry = corners(96, 0.24, 0.28);
  const bspire = poly([belfry.L2, [CX, belfry.T2[1] + FHH * 0.28], belfry.R2, [CX, belfry.T2[1] - 22]], shade(TERRA, 0.95));
  const clock = circle((belfry.R2[0] + belfry.B2[0]) / 2, belfry.R2[1] - 12, 3.4, "#e8ddc4") + circle((belfry.R2[0] + belfry.B2[0]) / 2, belfry.R2[1] - 12, 3.4, "none", ` stroke="#33241a" stroke-width="0.9"`);
  return svg(walls(c, "#cdc0a4") + hipRoof(c, 18, TERRA) + windows(c, 3, 46) + door(c, 13, 20, "#33241a")
    + walls(belfry, shade("#cdc0a4", 1.04)) + clock + bspire); };

B.mint = () => { const c = corners(40); const dx = (c.R[0] + c.B[0]) / 2, dy = (c.R[1] + c.B[1]) / 2;
  return svg(walls(c, "#c4b89c") + hipRoof(c, 16, SLATE) + door(c, 11, 18, "#2a2016")
    + circle(dx, dy - 24, 5, GOLD) + circle(dx, dy - 24, 5, "none", ` stroke="#8a6a30" stroke-width="1"`)); };

B.bank = () => { const c = corners(42);
  const frieze = poly([c.B2, c.R2, [c.R2[0], c.R2[1] - 5], [c.B2[0], c.B2[1] - 5]], shade(MARBLE, 1.06));
  return svg(walls(c, "#cbbfa4") + hipRoof(c, 16, SLATE) + colonnade(c, 3, 26) + frieze
    + circle((c.R[0] + c.B[0]) / 2, (c.R[1] + c.B[1]) / 2 - 30, 4, GOLD)); };

B.harbor = () => { const c = corners(16, 1.0, 1.0);
  let bol = ""; for (const p of [c.L, c.T, c.R]) bol += circle(p[0], p[1] - 3, 2.4, "#3a2f22");
  const cbx = CX - 6, cby = c.T2[1];
  const crane = line([cbx, cby + 6], [cbx, cby - 26], WOOD, 3) + line([cbx, cby - 26], [cbx + 24, cby - 18], WOOD, 2.4)
    + line([cbx + 24, cby - 18], [cbx + 24, cby - 8], "#2a3742", 1);
  return svg(topFace(c, "#9a8f78") + walls(c, "#9a8f78") + bol + crane); };

// Sprite-space Y of each building's flag anchor (roof peak) — mirrored in CityView.
const PEAK = { house: 114, guildhall: 86, workshop: 102, granary: 88, warehouse: 107, shipyard: 122,
  fondaco: 96, cathedral: 20, temple: 96, citadel: 42, palace: 92, council_hall: 42, mint: 104, bank: 102, harbor: 74 };

const names = Object.keys(B);
for (const name of names) writeFileSync(join(OUT, `${name}.svg`), B[name]());
writeFileSync(join(OUT, "_peaks.json"), JSON.stringify(PEAK, null, 0) + "\n");
console.log(`Wrote ${names.length} sprites → ${OUT}\n  ${names.join(", ")}`);
