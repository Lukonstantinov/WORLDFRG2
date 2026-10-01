// The council's rooms as PIXEL SCENES (design handoff "Council & People", 7a/7b).
// Pure canvas: a back wall and perspective floor per hall style, a dais with the
// head's throne, councillors on benches along an arc, a lectern with the motion.
// The caller supplies each seat's likeness (a `Person` from the face genome) and
// gets back hit-spots in LOGICAL pixels so the DOM can lay labels and click
// targets over the scaled canvas.
import { drawFigure, mix, type Person } from "@ui/campaign/cultureDress";

export type HallStyle = "med" | "hansa" | "steppe";
export interface SceneSeat { head: boolean; person: Person; ring: string; lean: number }
export interface SceneOpts { style: HallStyle; houses: string[]; debate: boolean; leanColor: (v: number) => string }
export interface Spot { idx: number; x: number; top: number; w: number; h: number; label: number }
export interface Scene { canvas: HTMLCanvasElement; spots: Spot[]; LW: number; LH: number }

interface Hall { wall: string; wallD: string; col: string; colD: string; recess: string; ceil: string; trim: string; f1: string; f2: string; carpet: string; carpetE: string; bench: string; benchL: string; sky: string }
const HALL: Record<HallStyle, Hall> = {
  med: { wall: "#c4ae86", wallD: "#9c8662", col: "#e6dcc4", colD: "#b4a888", recess: "#5a4a36", ceil: "#6a5238", trim: "#d8b24a", f1: "#c2a274", f2: "#8a6a46", carpet: "#8a2a2a", carpetE: "#d8b24a", bench: "#6a4228", benchL: "#8a5a36", sky: "#8ab8d8" },
  hansa: { wall: "#d8ccb0", wallD: "#b8a888", col: "#4a3020", colD: "#3a2418", recess: "#8a4a32", ceil: "#3a2418", trim: "#c9a227", f1: "#9a7448", f2: "#7a5836", carpet: "#2f4a6a", carpetE: "#c9a227", bench: "#5a3a22", benchL: "#7a5232", sky: "#a8c8d0" },
  steppe: { wall: "#e8dcc0", wallD: "#c8b894", col: "#a8784a", colD: "#7a5232", recess: "#c0503a", ceil: "#c0503a", trim: "#d8a040", f1: "#9a3a2a", f2: "#6a2a20", carpet: "#3a5a7a", carpetE: "#d8a040", bench: "#3a5a7a", benchL: "#5a7a9a", sky: "#e8e0c8" },
};
const hexRGB = (h: string) => { const t = h.replace("#", ""); return [0, 2, 4].map((i) => parseInt(t.slice(i, i + 2), 16)); };
const mixRGB = (a: number[], b: number[], t: number) => a.map((v, i) => v + (b[i] - v) * t);

function scene(style: HallStyle, houses: string[], LW: number, LH: number) {
  const H = HALL[style], canvas = document.createElement("canvas");
  canvas.width = LW; canvas.height = LH;
  const x = canvas.getContext("2d")!; x.imageSmoothingEnabled = false;
  const R = (a: number, b: number, w: number, h: number, c: string) => { x.fillStyle = c; x.fillRect(Math.round(a), Math.round(b), Math.round(w), Math.round(h)); };
  const VY = 64, cx = LW / 2, hs = houses.length ? houses : ["#9a2b2b", "#2f6fb0", "#c9a227"];
  R(0, 0, LW, VY, H.wall);
  if (style === "med") {
    R(0, 0, LW, 9, H.ceil); for (let i = 0; i < LW; i += 12) R(i, 2, 8, 5, mix(H.ceil, "#000000", 0.25));
    R(0, 9, LW, 3, H.trim); R(0, 12, LW, 2, H.wallD);
    const n = 8, step = LW / n;
    for (let k = 0; k < n; k++) {
      const a = k * step + 7, w = step - 14, top = 22;
      R(a, top, w, VY - top, H.recess); x.fillStyle = H.recess; x.beginPath(); x.arc(a + w / 2, top, w / 2, Math.PI, 0); x.fill();
      if (k % 2) {
        R(a + w / 2 - 3, top - 4, 6, 26, H.sky); x.fillStyle = H.sky; x.beginPath(); x.arc(a + w / 2, top - 4, 3, Math.PI, 0); x.fill();
        x.fillStyle = "rgba(255,240,200,.08)"; x.beginPath(); x.moveTo(a + w / 2 - 3, top + 22); x.lineTo(a + w / 2 + 3, top + 22); x.lineTo(a + w / 2 + 22, LH); x.lineTo(a + w / 2 - 10, LH); x.fill();
      } else {
        const hc = hs[(k / 2) % hs.length];
        R(a + w / 2 - 6, top - 6, 12, 30, hc); R(a + w / 2 - 6, top - 6, 12, 2, H.trim);
        x.fillStyle = hc; x.beginPath(); x.moveTo(a + w / 2 - 6, top + 24); x.lineTo(a + w / 2, top + 29); x.lineTo(a + w / 2 + 6, top + 24); x.fill();
        R(a + w / 2 - 2, top + 6, 4, 4, H.trim);
      }
    }
    for (let k = 0; k <= n; k++) { const a = k * step - 4; R(a, 14, 8, VY - 14, H.col); R(a + 5, 14, 3, VY - 14, H.colD); R(a - 1, 14, 10, 3, H.colD); R(a - 1, VY - 4, 10, 4, H.colD); }
  } else if (style === "hansa") {
    R(0, 0, LW, 6, H.ceil);
    for (let i = 0; i < LW; i += 46) { R(i, 0, 6, VY, H.col); R(i + 4, 0, 2, VY, H.colD); }
    R(0, 12, LW, 5, H.col); R(0, 40, LW, 4, H.col);
    for (let i = 0; i < LW; i += 92) { x.strokeStyle = H.col; x.lineWidth = 3; x.beginPath(); x.moveTo(i + 6, 44); x.lineTo(i + 46, 17); x.moveTo(i + 92, 44); x.lineTo(i + 52, 17); x.stroke(); }
    for (const wx of [LW * 0.18, LW * 0.82]) {
      R(wx - 12, 17, 24, 23, "#3a2a1a"); R(wx - 10, 19, 20, 19, H.sky);
      for (let i = 0; i < 4; i++) for (let j = 0; j < 4; j++) { R(wx - 10 + i * 5, 19 + j * 5, 1, 19, "#5a6a6a"); R(wx - 10, 19 + j * 5, 20, 1, "#5a6a6a"); }
    }
    [LW * 0.34, LW * 0.44, LW * 0.56, LW * 0.66].forEach((bx, i) => { const hc = hs[i % hs.length]; R(bx - 6, 17, 12, 26, hc); R(bx - 6, 17, 12, 2, H.trim); R(bx - 2, 26, 4, 4, H.trim); });
  } else {
    for (let i = -LH; i < LW; i += 8) { x.strokeStyle = H.col; x.lineWidth = 1; x.beginPath(); x.moveTo(i, 22); x.lineTo(i + VY - 22, VY); x.moveTo(i + VY - 22, 22); x.lineTo(i, VY); x.stroke(); }
    x.strokeStyle = H.recess; x.lineWidth = 2; for (let k = -10; k <= 10; k++) { x.beginPath(); x.moveTo(cx, -60); x.lineTo(cx + k * 30, 22); x.stroke(); }
    R(0, 0, LW, 4, H.ceil); R(0, 20, LW, 3, H.trim);
    [LW * 0.12, LW * 0.3, LW * 0.7, LW * 0.88].forEach((bx, i) => { const hc = hs[i % hs.length]; R(bx - 10, 26, 20, 30, hc); for (let j = 0; j < 4; j++) R(bx - 8, 29 + j * 7, 16, 2, H.trim); });
  }
  // floor, in perspective
  const img = x.getImageData(0, VY, LW, LH - VY), D = img.data, c1 = hexRGB(H.f1), c2 = hexRGB(H.f2), ca = hexRGB(H.carpet), ce = hexRGB(H.carpetE);
  for (let j = 0; j < LH - VY; j++) for (let i = 0; i < LW; i++) {
    const y = j + VY, dz = y - 36, u = (i - cx) / dz * 6, v = 380 / dz; let c: number[];
    if (style === "steppe") { const ring = Math.abs(u) + Math.abs(v - 9) * 0.9; c = Math.floor(ring * 1.6) % 3 === 0 ? ce : (Math.floor(u * 2 + v) % 2 ? c1 : c2); if (Math.abs(u) > 9) c = c2; }
    else if (style === "hansa") { const row = Math.floor(v * 1.4); const n = ((row * 37 + Math.floor((u + row * 0.7) / 2.6) * 11) % 7) / 14; c = mixRGB(row % 2 ? c1 : c2, c2, n); if (Math.abs((v * 1.4) % 1) < 0.08) c = mixRGB(c2, [0, 0, 0], 0.3); }
    else c = (Math.floor(u) + Math.floor(v)) % 2 ? c1 : c2;
    if (style !== "steppe" && Math.abs(u) < 1.6) c = Math.abs(u) > 1.35 ? ce : ca;
    const sf = 0.78 + 0.22 * (j / (LH - VY)), k = (j * LW + i) * 4;
    D[k] = c[0] * sf; D[k + 1] = c[1] * sf; D[k + 2] = c[2] * sf; D[k + 3] = 255;
  }
  x.putImageData(img, 0, VY);
  R(0, VY - 2, LW, 2, mix(H.wallD, "#000000", 0.3));
  return { canvas, x, H, R, VY, cx };
}

const upscale = (cv: HTMLCanvasElement, SC: number) => {
  const out = document.createElement("canvas"); out.width = cv.width * SC; out.height = cv.height * SC;
  const o = out.getContext("2d")!; o.imageSmoothingEnabled = false; o.drawImage(cv, 0, 0, out.width, out.height); return out;
};

/** 7a — the chamber as a pixel scene. Returns the logical canvas (382×150) and
 *  each seat's hit-spot; `SC` is the integer upscale the caller should show it at. */
export function drawChamber(seatsIn: SceneSeat[], o: SceneOpts, SC = 3): Scene & { out: HTMLCanvasElement } {
  const LW = 382, LH = 150, { canvas, x, H, R, VY, cx } = scene(o.style, o.houses, LW, LH);
  const headI = Math.max(0, seatsIn.findIndex((s) => s.head)), head = seatsIn[headI];
  const spots: Spot[] = [];
  const others = seatsIn.map((s, idx) => ({ s, idx })).filter((e) => e.idx !== headI);
  // dais, throne and the head of the council
  R(cx - 44, VY - 12, 88, 6, H.benchL); R(cx - 48, VY - 6, 96, 12, H.bench); R(cx - 48, VY - 6, 96, 1, H.trim); R(cx - 52, VY + 6, 104, 4, mix(H.bench, "#000000", 0.3));
  R(cx - 12, 4, 24, VY - 14, H.bench); R(cx - 9, 7, 18, VY - 20, head.ring); R(cx - 12, 4, 24, 2, H.trim); R(cx - 2, 0, 4, 5, H.trim);
  drawFigure(x, cx - 14, VY - 58, 28, head.person.kit, { person: head.person, cols: 28, occasion: "ceremonial" });
  R(cx - 40, VY - 22, 80, 18, H.bench); R(cx - 40, VY - 22, 80, 2, H.benchL); R(cx - 40, VY - 9, 80, 2, H.trim); R(cx - 6, VY - 19, 12, 8, H.trim); R(cx - 4, VY - 17, 8, 4, head.ring);
  spots.push({ idx: headI, x: cx, top: VY - 60, w: 30, h: 42, label: VY - 2 });
  // councillors on an arc, back to front
  const n = others.length, half = Math.ceil(n / 2), place: { e: (typeof others)[number]; sx: number; sy: number }[] = [];
  others.forEach((e, i) => {
    const left = i < half, k = left ? i : i - half, m = left ? half : n - half, a = Math.PI * (0.1 + (k / Math.max(1, m - 1)) * 0.33);
    place.push({ e, sx: left ? cx - Math.cos(a) * 174 : cx + Math.cos(a) * 174, sy: VY + 20 + Math.sin(a) * 46 });
  });
  place.sort((a, b) => a.sy - b.sy);
  for (const { e, sx, sy } of place) {
    const fw = 24, fh = Math.round(fw * 2.1), top = sy - fh + 4;
    R(sx - 15, sy - 32, 30, 8, mix(H.bench, "#000000", 0.25));
    drawFigure(x, sx - fw / 2, top, fw, e.s.person.kit, { person: e.s.person, cols: 24, occasion: "national" });
    R(sx - 17, sy - 25, 34, 22, H.bench); R(sx - 17, sy - 25, 34, 2, H.benchL); R(sx - 17, sy - 21, 34, 3, e.s.ring);
    for (let q = -12; q <= 12; q += 8) R(sx + q - 1, sy - 16, 2, 11, mix(H.bench, "#000000", 0.3));
    R(sx - 19, sy - 3, 38, 3, mix(H.bench, "#000000", 0.45));
    if (o.debate) { const lc = o.leanColor(e.s.lean); R(sx - 2, top - 5, 4, 4, lc); R(sx - 1, top - 6, 2, 1, lc); R(sx - 1, top - 1, 2, 1, lc); }
    spots.push({ idx: e.idx, x: sx, top, w: 26, h: fh - 22, label: sy });
  }
  // lectern with the motion
  { const lx = cx, ly = LH - 16; R(lx - 10, ly - 24, 20, 24, H.bench); R(lx - 13, ly - 28, 26, 6, H.benchL); R(lx - 9, ly - 31, 18, 5, "#efe6d0"); R(lx - 9, ly - 31, 18, 1, "#c8b890"); R(lx + 10, ly - 36, 2, 6, "#efe6d0"); R(lx + 10, ly - 38, 2, 2, "#ffcc55"); }
  const vg = x.createRadialGradient(cx, LH * 0.55, LH * 0.3, cx, LH * 0.55, LW * 0.62); vg.addColorStop(0, "rgba(0,0,0,0)"); vg.addColorStop(1, "rgba(6,8,14,.42)"); x.fillStyle = vg; x.fillRect(0, 0, LW, LH);
  return { canvas, out: upscale(canvas, SC), spots, LW, LH };
}

export interface TableGeom { tx: number; ty: number; tw: number; th: number }
export interface TableScene extends Scene { out: HTMLCanvasElement; geom: TableGeom; chairs: { idx: number; cx: number; cy: number; top: boolean }[]; headAt: { x: number; y: number } }

/** 7b — the council table from above. */
export function drawTable(seatsIn: SceneSeat[], SC = 3): TableScene {
  const LW = 392, LH = 146, canvas = document.createElement("canvas"); canvas.width = LW; canvas.height = LH;
  const x = canvas.getContext("2d")!;
  const R = (a: number, b: number, w: number, h: number, c: string) => { x.fillStyle = c; x.fillRect(Math.round(a), Math.round(b), Math.round(w), Math.round(h)); };
  for (let j = 0; j < LH; j += 6) { const off = (j / 6 % 3) * 23; for (let i = -off; i < LW; i += 70) R(i, j, 69, 5, ["#a88458", "#9a7a50", "#b08c5e"][(i + j) % 3 === 0 ? 0 : (((i + j) / 7) | 0) % 3]); R(0, j + 5, LW, 1, "#7a5a3a"); }
  const rx0 = 40, ry0 = 22, rw = LW - 80, rh = LH - 44; R(rx0, ry0, rw, rh, "#2f4a6a"); R(rx0 + 3, ry0 + 3, rw - 6, rh - 6, "#3a5a7e");
  for (let i = rx0 + 8; i < rx0 + rw - 8; i += 10) { R(i, ry0 + 4, 4, 2, "#c9a227"); R(i, ry0 + rh - 6, 4, 2, "#c9a227"); }
  const tx = 92, ty = 50, tw = 212, th = 46;
  R(tx + 3, ty + 4, tw, th, "rgba(0,0,0,.25)"); R(tx, ty, tw, th, "#6a4228"); R(tx + 2, ty + 2, tw - 4, th - 4, "#7e5232");
  for (let j = ty + 5; j < ty + th - 3; j += 4) R(tx + 3, j, tw - 6, 1, "#6e4629");
  R(tx, ty, tw, 2, "#9a6a42"); R(tx + tw / 2 - 52, ty + 12, 104, 22, "#2f5a3a"); R(tx + tw / 2 - 50, ty + 14, 100, 18, "#3a6a46");
  for (const [px, py, w, h] of [[tx + 14, ty + 8, 14, 10], [tx + 32, ty + 28, 12, 9], [tx + 166, ty + 6, 13, 10], [tx + 184, ty + 28, 14, 9], [tx + 62, ty + 36, 10, 8], [tx + 128, ty + 36, 11, 8]]) { R(px, py, w, h, "#efe6d0"); R(px + 2, py + 3, w - 4, 1, "#b8a888"); R(px + 2, py + 5, w - 5, 1, "#b8a888"); }
  for (const [px, py] of [[tx + 32, ty + 12], [tx + 172, ty + 22], [tx + 48, ty + 4]]) { R(px, py, 4, 4, "#1c1c22"); R(px + 1, py - 4, 1, 4, "#e8e0d0"); }
  for (const [px, py] of [[tx + 6, ty + 20], [tx + tw - 9, ty + 20], [tx + tw / 2 - 2, ty + 3]]) { x.fillStyle = "rgba(255,200,90,.28)"; x.beginPath(); x.arc(px + 1.5, py + 1.5, 7, 0, 7); x.fill(); R(px, py, 3, 3, "#efe6d0"); R(px + 1, py - 2, 1, 2, "#ffcc55"); }
  const headI = Math.max(0, seatsIn.findIndex((s) => s.head)), head = seatsIn[headI];
  const others = seatsIn.map((s, idx) => ({ s, idx })).filter((e) => e.idx !== headI), n = others.length, topN = Math.ceil(n / 2);
  const chairs: TableScene["chairs"] = [];
  others.forEach((e, i) => { const top = i < topN, k = top ? i : i - topN, m = top ? topN : n - topN; chairs.push({ idx: e.idx, cx: tx + 24 + (k + 0.5) * ((tw - 48) / m), cy: top ? ty - 13 : ty + th + 13, top }); });
  for (const p of chairs) { R(p.cx - 9, p.cy - 8, 18, 16, "#4a2e1a"); R(p.cx - 9, p.top ? p.cy - 11 : p.cy + 6, 18, 5, "#3a2414"); R(p.cx - 7, p.cy - 6, 14, 12, seatsIn[p.idx].ring); }
  R(tx - 26, ty + th / 2 - 14, 22, 28, "#3a2414"); R(tx - 30, ty + th / 2 - 16, 8, 32, "#2a1a0e"); R(tx - 23, ty + th / 2 - 11, 16, 22, head.ring); R(tx - 30, ty + th / 2 - 16, 8, 2, "#c9a227");
  return { canvas, out: upscale(canvas, SC), spots: [], LW, LH, geom: { tx, ty, tw, th }, chairs, headAt: { x: tx - 18, y: ty + th / 2 } };
}
