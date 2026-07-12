// Isometric city-building sprite generator (building-designer templates).
//
// Emits one clean iso SVG per building type into public/city-sprites/. The city
// view (src/ui/CityView.tsx) blits these in place of its procedural blocks and
// falls back to the drawing when one is missing. Owner colour is NOT baked in —
// walls are a neutral stone/timber so the panel's ground wash + heraldic flag
// carry faction colour. Re-run:  node scripts/gen_city_sprites.mjs
//
// Design language: true 2:1 isometric; footprint fills the tile with its BASE at
// the bottom-centre; consistent light (top brightest, right lit, left shaded);
// warm stone walls + terracotta roofs, stone-grey for fortifications, gilt for
// temples/mints. viewBox 140×180 (extra height for spires/towers).

import { mkdirSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const OUT = join(dirname(fileURLToPath(import.meta.url)), "..", "public", "city-sprites");
mkdirSync(OUT, { recursive: true });

// ── geometry + colour helpers ───────────────────────────────────────────────
const CX = 70, BASE_Y = 150, HW = 52, HH = 26;
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
// iso corners for a box `wh` tall on a footprint scaled by `sx,sy` (1 = full tile).
function corners(wh, sx = 1, sy = 1, oy = 0) {
  const hw = HW * sx, hh = HH * sy, by = BASE_Y + oy;
  return {
    L: [CX - hw, by], T: [CX, by - hh], R: [CX + hw, by], B: [CX, by + hh],
    L2: [CX - hw, by - wh], T2: [CX, by - hh - wh], R2: [CX + hw, by - wh], B2: [CX, by + hh - wh],
  };
}
// The two visible walls (front-left shaded, front-right lit).
function walls(c, wall) {
  return poly([c.L, c.B, c.B2, c.L2], shade(wall, 0.72)) + poly([c.R, c.B, c.B2, c.R2], shade(wall, 0.9));
}
const topFace = (c, fill) => poly([c.L2, c.T2, c.R2, c.B2], fill);
// Hip (pyramid) roof over the wall-top diamond of `c`, apex `rh` above.
function hipRoof(c, rh, roof) {
  const A = [CX, c.T2[1] - rh];
  const rl = shade(roof, 0.8), rr = shade(roof, 1.02);
  return poly([A, c.T2, c.L2], shade(roof, 0.7)) + poly([A, c.T2, c.R2], shade(roof, 0.88))
    + poly([A, c.L2, c.B2], rl) + poly([A, c.R2, c.B2], rr);
}
// Gable roof: a ridge running L2→R2, raised `rh`; two long slopes + two triangles.
function gableRoof(c, rh, roof) {
  const rl = [c.L2[0], c.L2[1] - rh], rr = [c.R2[0], c.R2[1] - rh];
  return poly([rl, rr, c.R2, c.L2], shade(roof, 0.7))       // far slope
    + poly([rl, rr, c.B2], shade(roof, 1.02))               // near slope (lit)
    + poly([c.L2, rl, c.T2], shade(roof, 0.8))              // left gable
    + poly([c.R2, rr, c.T2], shade(roof, 0.9));             // right gable end
}
function crenel(c, roof) {
  let s = topFace(c, shade(roof, 1.0));
  const m = 7, mh = 9;
  for (const p of [c.L2, c.B2, c.R2]) {
    s += poly([[p[0] - m, p[1] - mh], [p[0] + m, p[1] - mh], [p[0] + m, p[1]], [p[0] - m, p[1]]], shade(roof, 1.12));
    s += poly([[p[0] - m, p[1] - mh], [p[0], p[1] - mh - 3.5], [p[0] + m, p[1] - mh], [p[0], p[1] - mh + 3.5]], shade(roof, 1.28));
  }
  return s;
}
function dome(c, rh, gold) {
  const cy = c.T2[1] + HH * 0.4;
  return topFace(c, shade(gold, 0.7))
    + `<ellipse cx="${CX}" cy="${r(cy)}" rx="${r(HW * 0.62)}" ry="${r(HH + rh)}" fill="${shade(gold, 1.0)}"/>`
    + circle(CX, cy - HH - rh, 2.4, shade(gold, 1.3));
}
// A door on the front-right wall face, a window trio, etc. (front-right = lit face)
function door(c, w = 12, hgt = 20, col = "#4a3526") {
  const mx = (c.R[0] + c.B[0]) / 2, my = (c.R[1] + c.B[1]) / 2;
  return poly([[mx - w / 2, my - 2], [mx + w / 2, my - 2 - w * 0.28], [mx + w / 2, my - hgt], [mx - w / 2, my - hgt + w * 0.28]], col);
}
function windows(c, n, wh, col = "#33465a") {
  let s = ""; const y0 = c.B2[1] + 6;
  for (let i = 0; i < n; i++) {
    const t = (i + 1) / (n + 1);
    const x = c.B[0] + (c.R[0] - c.B[0]) * t, y = c.B[1] + (c.R[1] - c.B[1]) * t;
    const wy = y - wh * 0.5;
    s += poly([[x - 3, wy], [x + 3, wy - 1.6], [x + 3, wy - 9], [x - 3, wy - 7.4]], col);
  }
  return s;
}
const finial = (x, topY, h = 10) => line([x, topY], [x, topY - h], "#2a3742", 1.2) + circle(x, topY - h, 1.8, "#8794a0");

// ── colours ─────────────────────────────────────────────────────────────────
const STONE = "#cdbfa6", TIMBER = "#b8a582", WOOD = "#8a6a46", FORT = "#9aa0a6";
const TERRA = "#b0603f", SLATE = "#5f6b78", GOLD = "#caa24a", DARKW = "#4a3626";

function svg(inner) {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 140 180" width="140" height="180">${inner}</svg>\n`;
}

// ── building templates ──────────────────────────────────────────────────────
const B = {};

B.house = () => { const c = corners(30, 0.62, 0.62, 8); return svg(walls(c, TIMBER) + hipRoof(c, 20, TERRA) + door(c, 8, 13) + finial(CX, c.T2[1] - 20)); };

B.guildhall = () => { const c = corners(50, 0.9, 0.9); return svg(walls(c, STONE) + hipRoof(c, 26, TERRA) + door(c, 13, 24, "#3a2a1c") + windows(c, 3, 50) + finial(CX, c.T2[1] - 26, 12)); };

B.workshop = () => { const c = corners(38, 0.8, 0.8, 4); const ch = [c.R2[0] - 14, c.R2[1] - 6];
  return svg(walls(c, WOOD) + gableRoof(c, 20, TERRA) + door(c, 16, 20, "#2e2015") + poly([[ch[0] - 3, ch[1]], [ch[0] + 3, ch[1] - 1.5], [ch[0] + 3, ch[1] - 16], [ch[0] - 3, ch[1] - 14.5]], "#5a5148") + circle(ch[0] + 1, ch[1] - 20, 4, "#8a929a") + circle(ch[0] + 5, ch[1] - 26, 5, "#9aa2aa")); };

B.granary = () => { const c = corners(50, 0.66, 0.66, 2); let v = ""; for (let i = 0; i < 3; i++) v += poly([[c.B[0] - 2, c.B2[1] + 10 + i * 12], [c.B[0] + 2, c.B2[1] + 9 + i * 12], [c.B[0] + 2, c.B2[1] + 5 + i * 12], [c.B[0] - 2, c.B2[1] + 6 + i * 12]], "#3a2c1e");
  return svg(walls(c, "#c8b48c") + hipRoof(c, 22, "#9a7b46") + v + door(c, 9, 14)); };

B.warehouse = () => { const c = corners(34, 1.0, 1.0); return svg(walls(c, "#b7a888") + topFace(c, shade(SLATE, 1.0)) + door(c, 22, 24, "#2a1e14") + windows(c, 2, 34, "#2c3a48")); };

B.shipyard = () => { const c = corners(26, 0.8, 0.72, 6);
  const ramp = poly([[c.R[0] - 6, c.R[1] + 8], [c.R[0] + 26, c.R[1] + 20], [c.R[0] + 22, c.R[1] + 26], [c.R[0] - 10, c.R[1] + 14]], "#6b5942");
  const hull = poly([[c.R[0] + 6, c.R[1] + 14], [c.R[0] + 22, c.R[1] + 20], [c.R[0] + 16, c.R[1] + 24], [c.R[0] + 4, c.R[1] + 19]], "#4a3626");
  const mast = line([c.R[0] + 12, c.R[1] + 18], [c.R[0] + 12, c.R[1] + 2], "#3a2a1c", 1.4);
  return svg(`<ellipse cx="90" cy="172" rx="48" ry="9" fill="#1c3a4a"/>` + ramp + walls(c, WOOD) + gableRoof(c, 14, "#7a5238") + door(c, 18, 16, "#241812") + hull + mast); };

B.fondaco = () => { const c = corners(46, 0.92, 0.92); let arc = ""; for (let i = 0; i < 3; i++) { const t = (i + 1) / 4; const x = c.B[0] + (c.R[0] - c.B[0]) * t, y = c.B[1] + (c.R[1] - c.B[1]) * t; arc += `<path d="M ${r(x - 4)} ${r(y - 3)} q 4 -9 8 0 z" fill="#2e3a2c"/>`; }
  return svg(walls(c, "#c2b9a0") + topFace(c, shade("#8a7f66", 1.0)) + poly([c.L2, c.T2, c.R2, c.B2].map(([x, y]) => [x, y - 5]), shade("#8a7f66", 0.7)) + arc + `<rect x="30" y="118" width="80" height="4" fill="#7a8a4a" opacity="0.85" transform="skewX(-20)"/>`); };

B.cathedral = () => { const c = corners(56, 0.62, 0.9); // narrow tall nave
  const naveRoof = gableRoof(c, 24, "#7a4a5a");
  // bell tower on the right
  const t = corners(70, 0.28, 0.34, 0); t.L = [c.R[0] - 6, c.R[1]]; // shift tower to right side
  const tx = c.R[0] + 2; const tw = 18, tbY = c.R[1] + 4;
  const towerL = poly([[tx - tw, tbY], [tx, tbY + 9], [tx, tbY + 9 - 74], [tx - tw, tbY - 74]], shade("#c6b59a", 0.74));
  const towerR = poly([[tx + tw, tbY], [tx, tbY + 9], [tx, tbY + 9 - 74], [tx + tw, tbY - 74]], shade("#c6b59a", 0.92));
  const spire = poly([[tx - tw, tbY - 74], [tx, tbY - 74 + 9], [tx + tw, tbY - 74], [tx, tbY - 74 - 22]], shade("#7a4a5a", 0.95));
  const belfry = poly([[tx - 5, tbY - 40], [tx + 5, tbY - 38], [tx + 5, tbY - 52], [tx - 5, tbY - 54]], "#2a2028");
  const rose = circle((c.B[0] + c.T[0]) / 2, (c.B2[1] + c.T2[1]) / 2 - 4, 6, "#3a4a6a") + circle((c.B[0] + c.T[0]) / 2, (c.B2[1] + c.T2[1]) / 2 - 4, 6, "none", ` stroke="#caa24a" stroke-width="1.4"`);
  const portal = `<path d="M ${r((c.R[0] + c.B[0]) / 2 - 6)} ${r((c.R[1] + c.B[1]) / 2 - 2)} l 0 -14 q 6 -10 12 0 l 0 14 z" fill="#2a1e2a"/>`;
  return svg(walls(c, "#c6b59a") + naveRoof + rose + portal + towerL + towerR + belfry + spire + finial(tx, tbY - 74 - 22, 8)); };

B.temple = () => { const c = corners(40, 0.92, 0.92, 0);
  // stylobate steps
  let steps = ""; for (let i = 0; i < 2; i++) steps += poly([[c.L[0] + i * 4, c.L[1] + 6 - i * 3], [CX, c.B[1] + 10 - i * 3], [c.R[0] - i * 4, c.R[1] + 6 - i * 3], [CX, c.T[1] + 2]], shade("#d8ccb0", 1 - i * 0.08));
  // columns on the front-right edge
  let cols = ""; for (let i = 0; i <= 4; i++) { const tt = i / 4; const x = c.B[0] + (c.R[0] - c.B[0]) * tt, y = c.B[1] + (c.R[1] - c.B[1]) * tt; cols += poly([[x - 2.5, y - 4], [x + 2.5, y - 5.2], [x + 2.5, y - 34], [x - 2.5, y - 32.8]], "#e0d6bc"); }
  const pediment = poly([c.B2, c.R2, [(c.B2[0] + c.R2[0]) / 2, (c.B2[1] + c.R2[1]) / 2 - 16]], shade("#d8ccb0", 1.05));
  const entab = poly([c.B2, c.R2, [c.R2[0], c.R2[1] - 6], [c.B2[0], c.B2[1] - 6]], "#e8ddc4");
  return svg(steps + cols + entab + pediment + gableRoof(corners(40 + 6, 0.92, 0.92), 12, GOLD)); };

B.citadel = () => { const c = corners(52, 1.0, 1.0);
  const keep = corners(84, 0.42, 0.42, -2);
  return svg(walls(c, FORT) + crenel(c, FORT) + door(c, 12, 20, "#2a2e33") + windows(c, 3, 52, "#20262c")
    + walls(keep, shade(FORT, 1.06)) + crenel(keep, shade(FORT, 1.06))); };

B.palace = () => { const c = corners(46, 1.0, 1.0);
  const portico = poly([[CX - 10, c.B[1] - 2], [CX + 10, c.B[1] - 2], [CX + 10, c.B[1] - 26], [CX - 10, c.B[1] - 26]], "#e0d6bc");
  const cols = line([CX - 8, c.B[1] - 3], [CX - 8, c.B[1] - 24], "#c8bca0", 2.5) + line([CX + 8, c.B[1] - 3], [CX + 8, c.B[1] - 24], "#c8bca0", 2.5);
  return svg(walls(c, "#cdc0a4") + hipRoof(c, 22, TERRA) + windows(c, 4, 46) + portico + cols + door(c, 10, 18, "#3a2a1c") + finial(CX, c.T2[1] - 22, 10)); };

B.council_hall = () => { const c = corners(46, 0.94, 0.94);
  // belfry / clock tower centred
  const bx = CX, bbY = c.T2[1];
  const belfry = poly([[bx - 9, bbY], [bx, bbY + 5], [bx, bbY + 5 - 34], [bx - 9, bbY - 34]], shade("#cdc0a4", 0.78)) + poly([[bx + 9, bbY], [bx, bbY + 5], [bx, bbY + 5 - 34], [bx + 9, bbY - 34]], shade("#cdc0a4", 0.96));
  const clock = circle(bx + 4, bbY - 20, 4, "#e8ddc4") + circle(bx + 4, bbY - 20, 4, "none", ` stroke="#3a2a1c" stroke-width="1"`);
  const bspire = poly([[bx - 9, bbY - 34], [bx, bbY - 29], [bx + 9, bbY - 34], [bx, bbY - 50]], shade(TERRA, 0.95));
  return svg(walls(c, "#cdc0a4") + hipRoof(c, 20, TERRA) + windows(c, 3, 46) + door(c, 14, 22, "#33241a") + belfry + clock + bspire + finial(bx, bbY - 50, 7)); };

B.mint = () => { const c = corners(40, 0.86, 0.86); const dx = (c.R[0] + c.B[0]) / 2, dy = (c.R[1] + c.B[1]) / 2;
  return svg(walls(c, "#c4b89c") + topFace(c, shade(SLATE, 1.05)) + door(c, 12, 20, "#2a2016") + circle(dx, dy - 26, 6, GOLD) + circle(dx, dy - 26, 6, "none", ` stroke="#8a6a30" stroke-width="1"`) + `<text x="${r(dx)}" y="${r(dy - 23)}" font-size="7" fill="#7a5a20" text-anchor="middle" font-family="serif">✦</text>`); };

B.bank = () => { const c = corners(42, 0.9, 0.9);
  let cols = ""; for (let i = 0; i <= 3; i++) { const tt = i / 3; const x = c.B[0] + (c.R[0] - c.B[0]) * tt, y = c.B[1] + (c.R[1] - c.B[1]) * tt; cols += poly([[x - 2.4, y - 3], [x + 2.4, y - 4], [x + 2.4, y - 30], [x - 2.4, y - 29]], "#e0d6bc"); }
  const pediment = poly([c.B2, c.R2, [(c.B2[0] + c.R2[0]) / 2, (c.B2[1] + c.R2[1]) / 2 - 14]], shade("#d8ccb0", 1.05));
  return svg(walls(c, "#cbbfa4") + topFace(c, shade("#d8ccb0", 0.9)) + cols + pediment + circle((c.B[0] + c.R[0]) / 2, (c.B2[1] + c.R2[1]) / 2 - 8, 4, GOLD)); };

B.harbor = () => { const c = corners(20, 1.0, 0.8, 2);
  const water = `<ellipse cx="70" cy="168" rx="66" ry="12" fill="#183440"/>`;
  const quay = poly([c.L, c.B, c.R, c.T], "#9a8f78") + walls(c, "#9a8f78");
  let bollards = ""; for (const p of [c.L, c.T, c.R]) bollards += circle(p[0], p[1] - 3, 2.6, "#3a2f22");
  // a timber crane
  const cbx = c.T[0] + 8, cby = c.T2[1];
  const crane = line([cbx, cby + 8], [cbx, cby - 28], WOOD, 3) + line([cbx, cby - 28], [cbx + 26, cby - 20], WOOD, 2.4) + line([cbx + 26, cby - 20], [cbx + 26, cby - 8], "#2a3742", 1);
  return svg(water + quay + bollards + crane); };

// ── emit ─────────────────────────────────────────────────────────────────────
const files = Object.keys(B);
for (const name of files) {
  writeFileSync(join(OUT, `${name}.svg`), B[name]());
}
console.log(`Wrote ${files.length} sprites → ${OUT}\n  ${files.join(", ")}`);
