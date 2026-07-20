// Isometric settlement / estate schematic renderer (ported from the mockup).
// Pure DOM/SVG — populates a given <svg> element. Fed by HubDetail, so it costs
// nothing on the sim hot path (rendered once when a settlement tab is opened).
import type { HubDetail } from "@types";

const NS = "http://www.w3.org/2000/svg";
type G = SVGElement;
function el(p: string, a: Record<string, string | number>): G {
  const e = document.createElementNS(NS, p) as G;
  for (const k in a) e.setAttribute(k, String(a[k]));
  return e;
}
function txt(g: G, x: number, y: number, s: number, t: string, fill: string | null, wt?: number, anchor?: string, font?: string) {
  const e = el("text", { x, y, "font-size": s, fill: fill ?? "#000", "font-weight": wt ?? 400, "font-family": font ?? '"IM Fell English", Georgia, serif' });
  if (anchor) e.setAttribute("text-anchor", anchor);
  e.textContent = t; g.appendChild(e);
}
function rng(seed: number) { let a = seed >>> 0; return () => { a |= 0; a = (a + 0x6D2B79F5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }

// ── isometric building kit ──────────────────────────────────────────────────
const INK = "#3f3120", GOLD = "#c9a24a";
const PLAS = ["#ecdcb4", "#cdba8c"], STON = ["#dccfa8", "#bcae84"], WOOD = ["#caa974", "#a98b58"];
const TERR = ["#cf7a47", "#a3542f"], SLAT = ["#7d8a98", "#5c6675"];
function isoP(cx: number, cy: number, s: number, x: number, y: number, z: number) { return [cx + (x - y) * s, cy + (x + y) * s * 0.5 - z * s]; }
function fc(g: G, pts: number[][], fill: string, sw?: number) { g.appendChild(el("polygon", { points: pts.map((p) => p[0].toFixed(1) + "," + p[1].toFixed(1)).join(" "), fill, stroke: INK, "stroke-width": sw == null ? 0.5 : sw, "stroke-linejoin": "round" })); }
function gshadow(g: G, cx: number, cy: number, s: number, w: number) { g.appendChild(el("ellipse", { cx: cx.toFixed(1), cy: (cy + s * 0.16).toFixed(1), rx: (w * s).toFixed(1), ry: (w * s * 0.42).toFixed(1), fill: "#000", "fill-opacity": .13 })); }
function isoBox(g: G, cx: number, cy: number, s: number, w: number, d: number, h: number, rh: number, wall: string[], roof: string[]) {
  const hw = w / 2, hd = d / 2, P = (x: number, y: number, z: number) => isoP(cx, cy, s, x, y, z);
  fc(g, [P(hw, -hd, 0), P(hw, hd, 0), P(hw, hd, h), P(hw, -hd, h)], wall[1]);
  fc(g, [P(-hw, hd, 0), P(hw, hd, 0), P(hw, hd, h), P(-hw, hd, h)], wall[0]);
  if (rh > 0) { fc(g, [P(-hw, hd, h), P(hw, hd, h), P(hw, 0, h + rh), P(-hw, 0, h + rh)], roof[0]); fc(g, [P(hw, hd, h), P(hw, -hd, h), P(hw, 0, h + rh)], roof[1]); }
  else fc(g, [P(-hw, -hd, h), P(hw, -hd, h), P(hw, hd, h), P(-hw, hd, h)], roof[0]);
}
function door(g: G, cx: number, cy: number, s: number) { const P = (x: number, y: number, z: number) => isoP(cx, cy, s, x, y, z); fc(g, [P(-0.12, 0.41, 0), P(0.12, 0.41, 0), P(0.12, 0.41, 0.32), P(-0.12, 0.41, 0.32)], "#5a3c22", .3); }
function flag(g: G, cx: number, cy: number, s: number, top: number, col: string) { const a = isoP(cx, cy, s, 0, 0, top), f = isoP(cx, cy, s, 0, 0, top + 0.5); g.appendChild(el("line", { x1: a[0].toFixed(1), y1: a[1].toFixed(1), x2: f[0].toFixed(1), y2: f[1].toFixed(1), stroke: INK, "stroke-width": .8 })); g.appendChild(el("path", { d: `M${f[0].toFixed(1)} ${f[1].toFixed(1)} l${(0.42 * s).toFixed(1)} ${(0.1 * s).toFixed(1)} l${(-0.42 * s).toFixed(1)} ${(0.1 * s).toFixed(1)} Z`, fill: col || GOLD })); }

function house(g: G, x: number, b: number, s: number) { gshadow(g, x, b, s, 0.8); isoBox(g, x, b, s, 1, 0.82, 0.66, 0.5, PLAS, TERR); door(g, x, b, s); }
function manor(g: G, x: number, b: number, s: number) { gshadow(g, x, b, s, 1.1); isoBox(g, x - 0.5 * s, b + 0.26 * s, s, 0.75, 0.75, 0.6, 0.45, PLAS, TERR); isoBox(g, x, b, s, 1.55, 1.05, 0.78, 0.55, PLAS, TERR); door(g, x, b, s); }
function warehouse(g: G, x: number, b: number, s: number) { gshadow(g, x, b, s, 1.05); isoBox(g, x, b, s, 1.8, 1.0, 0.62, 0.4, WOOD, TERR); const P = (X: number, Y: number, Z: number) => isoP(x, b, s, X, Y, Z); for (const dx of [-0.5, 0.08]) fc(g, [P(dx, 0.5, 0), P(dx + 0.34, 0.5, 0), P(dx + 0.34, 0.5, 0.38), P(dx, 0.5, 0.38)], "#4f3320", .3); }
function granary(g: G, x: number, b: number, s: number) { gshadow(g, x, b, s, 1.0); isoBox(g, x + 0.72 * s, b + 0.36 * s, s, 0.44, 0.44, 0.98, 0.16, STON, ["#9c5a34", "#7f4a2a"]); isoBox(g, x, b, s, 1.1, 0.86, 0.56, 0.48, ["#d8b27c", "#bb9560"], ["#9c5a34", "#7f4a2a"]); }
function workshop(g: G, x: number, b: number, s: number) { house(g, x, b, s); isoBox(g, x + 0.3 * s, b + 0.1 * s, s, 0.18, 0.18, 1.05, 0, ["#8a684a", "#6c5036"], ["#6c5036", "#56432e"]); for (let i = 0; i < 3; i++) g.appendChild(el("circle", { cx: (x + 0.3 * s).toFixed(1), cy: (b - 1.0 * s - i * 4).toFixed(1), r: (2 + i), fill: "#d3dae0", "fill-opacity": (.5 - i * .13).toFixed(2) })); }
function guildhall(g: G, x: number, b: number, s: number) { gshadow(g, x, b, s, 1.15); isoBox(g, x, b, s, 1.5, 1.1, 0.78, 0.5, STON, SLAT); const tx = x + 0.74 * s; isoBox(g, tx, b, s, 0.5, 0.5, 1.0, 0, STON, STON); const P = (X: number, Y: number, Z: number) => isoP(tx, b, s, X, Y, Z); fc(g, [P(-0.25, 0.25, 1.0), P(0.25, 0.25, 1.0), P(0, 0, 1.6)], TERR[0]); fc(g, [P(0.25, -0.25, 1.0), P(0.25, 0.25, 1.0), P(0, 0, 1.6)], TERR[1]); flag(g, tx, b, s, 1.6, GOLD); }
function shipyard(g: G, x: number, b: number, s: number) { const P = (X: number, Y: number, Z: number) => isoP(x, b, s, X, Y, Z); fc(g, [P(-0.45, 0.85, 0), P(0.2, 0.85, 0), P(0.2, 1.9, 0), P(-0.45, 1.9, 0)], "#7a5a36", .3); boat(g, P(-0.12, 2.15, 0)[0], P(-0.12, 2.15, 0)[1], s * 0.55); gshadow(g, x, b, s, 1.0); isoBox(g, x, b, s, 1.45, 0.92, 0.6, 0.42, WOOD, TERR); }
function windmill(g: G, x: number, b: number, s: number) { gshadow(g, x, b, s, 0.7); isoBox(g, x, b, s, 0.62, 0.62, 1.3, 0.32, STON, TERR); const hub = isoP(x, b, s, 0, -0.32, 1.45), hx = hub[0], hy = hub[1]; for (const a of [0.5, 2.07, 3.64, 5.21]) g.appendChild(el("line", { x1: hx.toFixed(1), y1: hy.toFixed(1), x2: (hx + Math.cos(a) * s * 1.0).toFixed(1), y2: (hy + Math.sin(a) * s * 1.0).toFixed(1), stroke: INK, "stroke-width": 1.5 })); g.appendChild(el("circle", { cx: hx.toFixed(1), cy: hy.toFixed(1), r: 1.8, fill: INK })); }
function mine(g: G, x: number, b: number, s: number) { gshadow(g, x, b, s, 1.0); g.appendChild(el("ellipse", { cx: (x + 0.45 * s).toFixed(1), cy: (b + 0.22 * s).toFixed(1), rx: (0.55 * s).toFixed(1), ry: (0.24 * s).toFixed(1), fill: "#b09870" })); const P = (X: number, Y: number, Z: number) => isoP(x, b, s, X, Y, Z), aL = P(-0.45, 0.2, 0), aR = P(0.05, -0.2, 0), top = P(-0.2, 0, 1.2); g.appendChild(el("path", { d: `M${aL[0].toFixed(1)} ${aL[1].toFixed(1)} L${top[0].toFixed(1)} ${top[1].toFixed(1)} L${aR[0].toFixed(1)} ${aR[1].toFixed(1)}`, fill: "none", stroke: "#54422c", "stroke-width": 2, "stroke-linejoin": "round" })); const ad = P(0.18, 0.32, 0); g.appendChild(el("path", { d: `M${(ad[0] - 0.22 * s).toFixed(1)} ${ad[1].toFixed(1)} A${(0.22 * s).toFixed(1)} ${(0.16 * s).toFixed(1)} 0 0 1 ${(ad[0] + 0.22 * s).toFixed(1)} ${ad[1].toFixed(1)} Z`, fill: "#241a12" })); }
function office(g: G, x: number, b: number, s: number) { house(g, x, b, s); flag(g, x + 0.28 * s, b - 0.28 * s, s, 1.0, "#9a5a7a"); }
function boat(g: G, x: number, y: number, s: number) { g.appendChild(el("path", { d: `M${(x - s).toFixed(1)} ${y.toFixed(1)} Q${x.toFixed(1)} ${(y + s * 0.6).toFixed(1)} ${(x + s).toFixed(1)} ${y.toFixed(1)} Z`, fill: "#6e4f30" })); g.appendChild(el("path", { d: `M${x.toFixed(1)} ${(y - s).toFixed(1)} L${x.toFixed(1)} ${y.toFixed(1)} L${(x + s * 0.6).toFixed(1)} ${(y - s * 0.2).toFixed(1)} Z`, fill: "#e8dcc0" })); }
const ICONS: Record<string, (g: G, x: number, b: number, s: number) => void> = { guildhall, granary, warehouse, workshop, docks: shipyard, shipyard, windmill, mine, manor, office, house };
function iconFor(t: string) { return ICONS[t] || house; }

// ── climate ground + vegetation ─────────────────────────────────────────────
const GROUND = ["#c4d59a", "#e6cf90", "#aedd92", "#c6d2bd", "#dfeaf0"];
const TILEG = ["#e9eecb", "#f4e7bc", "#dcf0c0", "#e3e8d6", "#eef4f7"];
const SEA = ["#6f9fb6", "#3d6b91"];
function tDecid(g: G, x: number, b: number, s: number) { g.appendChild(el("line", { x1: x.toFixed(1), y1: b.toFixed(1), x2: x.toFixed(1), y2: (b - s * 0.55).toFixed(1), stroke: "#6b4f2e", "stroke-width": Math.max(1, s * 0.14) })); g.appendChild(el("ellipse", { cx: x.toFixed(1), cy: (b - s * 0.95).toFixed(1), rx: (s * 0.6).toFixed(1), ry: (s * 0.55).toFixed(1), fill: "#5e8a3e", stroke: "#456a2c", "stroke-width": .5 })); g.appendChild(el("ellipse", { cx: (x - s * 0.18).toFixed(1), cy: (b - s * 1.12).toFixed(1), rx: (s * 0.24).toFixed(1), ry: (s * 0.2).toFixed(1), fill: "#80ad5b", "fill-opacity": .7 })); }
function tConifer(g: G, x: number, b: number, s: number) { g.appendChild(el("line", { x1: x.toFixed(1), y1: b.toFixed(1), x2: x.toFixed(1), y2: (b - s * 0.3).toFixed(1), stroke: "#5a3f24", "stroke-width": Math.max(1, s * 0.12) })); for (let i = 0; i < 3; i++) { const yy = b - s * 0.28 - i * s * 0.4; g.appendChild(el("path", { d: `M${(x - s * (0.5 - i * 0.1)).toFixed(1)} ${yy.toFixed(1)} L${x.toFixed(1)} ${(yy - s * 0.62).toFixed(1)} L${(x + s * (0.5 - i * 0.1)).toFixed(1)} ${yy.toFixed(1)} Z`, fill: i === 2 ? "#43723c" : "#37602f", stroke: "#28492a", "stroke-width": .4 })); } }
function tPalm(g: G, x: number, b: number, s: number) { const tx = x + s * 0.08, ty = b - s * 1.1; g.appendChild(el("path", { d: `M${x.toFixed(1)} ${b.toFixed(1)} Q${(x - s * 0.12).toFixed(1)} ${(b - s * 0.6).toFixed(1)} ${tx.toFixed(1)} ${ty.toFixed(1)}`, fill: "none", stroke: "#8a6638", "stroke-width": Math.max(1.3, s * 0.16) })); for (const a of [-2.5, -1.9, -1.25, -0.6, 0.0]) g.appendChild(el("path", { d: `M${tx.toFixed(1)} ${ty.toFixed(1)} q${(Math.cos(a) * s * 0.5).toFixed(1)} ${(Math.sin(a) * s * 0.4).toFixed(1)} ${(Math.cos(a) * s * 0.95).toFixed(1)} ${(Math.sin(a) * s * 0.35 + s * 0.25).toFixed(1)}`, fill: "none", stroke: "#4e9040", "stroke-width": 1.4, "stroke-linecap": "round" })); g.appendChild(el("circle", { cx: tx.toFixed(1), cy: ty.toFixed(1), r: 1.6, fill: "#8a6638" })); }
function tCactus(g: G, x: number, b: number, s: number) { const w = s * 0.32; g.appendChild(el("rect", { x: (x - w / 2).toFixed(1), y: (b - s * 0.95).toFixed(1), width: w.toFixed(1), height: (s * 0.95).toFixed(1), rx: (w / 2).toFixed(1), fill: "#5b8a4a", stroke: "#3f6a34", "stroke-width": .5 })); g.appendChild(el("path", { d: `M${(x + w * 0.3).toFixed(1)} ${(b - s * 0.55).toFixed(1)} q${(s * 0.3).toFixed(1)} 0 ${(s * 0.3).toFixed(1)} ${(-s * 0.3).toFixed(1)}`, fill: "none", stroke: "#5b8a4a", "stroke-width": Math.max(2, w * 0.7), "stroke-linecap": "round" })); g.appendChild(el("path", { d: `M${(x - w * 0.3).toFixed(1)} ${(b - s * 0.4).toFixed(1)} q${(-s * 0.28).toFixed(1)} 0 ${(-s * 0.28).toFixed(1)} ${(-s * 0.28).toFixed(1)}`, fill: "none", stroke: "#5b8a4a", "stroke-width": Math.max(2, w * 0.7), "stroke-linecap": "round" })); }
function tShrub(g: G, x: number, b: number, s: number) { for (const dx of [-0.32, 0, 0.32]) g.appendChild(el("circle", { cx: (x + dx * s).toFixed(1), cy: (b - s * 0.22).toFixed(1), r: (s * 0.3).toFixed(1), fill: "#86954e", "fill-opacity": .92 })); }
function tRock(g: G, x: number, b: number, s: number) { g.appendChild(el("path", { d: `M${(x - s * 0.45).toFixed(1)} ${b.toFixed(1)} L${(x - s * 0.2).toFixed(1)} ${(b - s * 0.42).toFixed(1)} L${(x + s * 0.18).toFixed(1)} ${(b - s * 0.48).toFixed(1)} L${(x + s * 0.45).toFixed(1)} ${b.toFixed(1)} Z`, fill: "#9a958c", stroke: "#6f6a60", "stroke-width": .5 })); g.appendChild(el("path", { d: `M${(x - s * 0.2).toFixed(1)} ${(b - s * 0.42).toFixed(1)} L${(x + s * 0.18).toFixed(1)} ${(b - s * 0.48).toFixed(1)} L${(x + s * 0.05).toFixed(1)} ${(b - s * 0.4).toFixed(1)} Z`, fill: "#eef1f3" })); }
function plant(band: number, g: G, x: number, b: number, s: number, alt: boolean) {
  if (band === 1) return (alt ? tCactus : tPalm)(g, x, b, s);
  if (band === 2) return tPalm(g, x, b, s);
  if (band === 3) return tConifer(g, x, b, s);
  if (band === 4) return tRock(g, x, b, s);
  return (alt ? tShrub : tDecid)(g, x, b, s);
}
function goodBadge(g: G, x: number, y: number, icon: string) { g.appendChild(el("circle", { cx: x.toFixed(1), cy: y.toFixed(1), r: 8.5, fill: "#fff7e6", stroke: "#b89a5a", "stroke-width": 1.2 })); txt(g, x, y + 3.7, 11, icon, null, 400, "middle", '"Segoe UI Emoji","Noto Color Emoji",system-ui'); }

// ── scene model ──────────────────────────────────────────────────────────────
export interface SceneBldg { t: string; name: string; owner: string; guild?: boolean; tier?: number; goodIcon?: string | null; }
export interface Scene { name: string; isEstate: boolean; population: number; band: number; coastal: boolean; meta: string; buildings: SceneBldg[]; seed: number; }

const BAND_NAME = ["Temperate", "Arid", "Tropical", "Boreal", "Polar"];
export function koppenBand(k: number): number {
  if ([4, 5, 6, 7].includes(k)) return 1;          // deserts/steppe
  if ([1, 2, 3, 23, 24].includes(k)) return 2;     // tropical
  if ([21, 22].includes(k)) return 4;              // tundra / ice
  if ([13, 16, 17, 27, 28, 29, 30, 31].includes(k)) return 3; // boreal / continental
  return 0;                                        // temperate
}
function structType(name: string): string {
  const n = name.toLowerCase();
  if (n.includes("granary")) return "granary";
  if (n.includes("warehouse")) return "warehouse";
  if (n.includes("shipyard") || n.includes("dock")) return "shipyard";
  if (n.includes("guild")) return "guildhall";
  if (n.includes("workshop")) return "workshop";
  return "house";
}
function estateIcon(kind: number): string { return ({ 1: "manor", 2: "mine", 3: "manor", 4: "shipyard", 5: "manor", 6: "windmill" } as Record<number, string>)[kind] || "manor"; }
const ESTATE_LABEL: Record<number, string> = { 1: "Farm", 2: "Mine", 3: "Plantation", 4: "Fishery", 5: "Vineyard", 6: "Manufactory" };
function cap(s: string) { return s ? s.charAt(0).toUpperCase() + s.slice(1) : s; }
export function fmtPop(p: number) { return p >= 1e6 ? (p / 1e6).toFixed(2) + "M" : p >= 1e3 ? Math.round(p / 1e3) + "k" : String(Math.round(p)); }

/** Build a scene from a live HubDetail. `goodIcon` resolves a good name → emoji. */
export function sceneFromDetail(d: HubDetail, goodIcon: (g: string) => string): Scene {
  const band = koppenBand(d.koppen);
  const blds: SceneBldg[] = [];
  if (!d.is_estate) {
    for (const [name] of d.structures ?? []) blds.push({ t: structType(name), name, owner: "Settlement" });
    for (const e of (d.estates_here ?? []).slice(0, 5))
      blds.push({ t: estateIcon(e.kind), name: e.name || ESTATE_LABEL[e.kind] || "Estate", owner: e.owner || "Settlement", guild: e.owner_is_guild, tier: e.tier, goodIcon: e.good ? goodIcon(e.good) : null });
    for (const o of (d.offices_here ?? []).slice(0, 3)) blds.push({ t: "office", name: "Counting-house", owner: o.holder });
  } else {
    const k = d.estate_kind ?? 0;
    blds.push({ t: estateIcon(k), name: d.estate_good ? cap(d.estate_good) : (ESTATE_LABEL[k] || "Estate"), owner: d.estate_owner || "Settlement", goodIcon: d.estate_good ? goodIcon(d.estate_good) : null });
  }
  if (blds.length === 0) blds.push({ t: "house", name: "Hamlet", owner: "Settlement" });
  const meta = d.is_estate
    ? `${ESTATE_LABEL[d.estate_kind ?? 0] || "estate"}${d.estate_good ? " · " + d.estate_good : ""}`
    : `pop ${fmtPop(d.population)} · ${BAND_NAME[band]}${d.coastal ? " · coastal" : ""}`;
  return { name: d.name, isEstate: d.is_estate, population: d.population, band, coastal: d.coastal, meta, buildings: blds.slice(0, 11), seed: (d.id || 1) >>> 0 };
}

// ── render into an existing <svg> ─────────────────────────────────────────────
export function renderSettlement(svg: SVGSVGElement, sc: Scene, W = 360): number {
  while (svg.firstChild) svg.removeChild(svg.firstChild);
  const band = sc.band || 0, coastal = !!sc.coastal, seaH = coastal ? 28 : 0;
  const blds = sc.buildings.length ? sc.buildings : [{ t: "house", name: "Hamlet", owner: "Settlement" }];
  const top = 48, pad = 12, dwellH = sc.isEstate ? 0 : 48, tileH = 96;
  const cols = blds.length <= 1 ? 1 : 2, rows = Math.ceil(blds.length / cols);
  const H = top + rows * tileH + dwellH + pad + seaH;
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  svg.appendChild(el("rect", { x: 6, y: 6, width: W - 12, height: H - 12, rx: 9, fill: GROUND[band] }));
  const tw = (W - 2 * pad) / cols;
  blds.forEach((bb, i) => tile(svg, bb, pad + (i % cols) * tw, top + Math.floor(i / cols) * tileH, tw, tileH, band));
  if (!sc.isEstate) {
    const r = rng(sc.seed), by = H - pad - seaH - 8, n2 = Math.min(16, Math.max(5, Math.round(sc.population / 13000))), gap = (W - 2 * pad) / n2;
    txt(svg, pad, by - 20, 9.5, "Dwellings & countryside", "#5a4a30", 600);
    txt(svg, W - pad, by - 20, 9, `Settlement · ${fmtPop(sc.population)} souls`, "#7a6a4a", 400, "end");
    for (let i = 0; i < n2; i++) { const px = pad + gap * (i + 0.5) + (r() - 0.5) * 3; if (i % 3 === 2) plant(band, svg, px, by, 8, i % 2 === 1); else house(svg, px, by, 7.5); }
  }
  if (coastal) drawSea(svg, 9, H - 7 - seaH, W - 18, seaH, sc.seed);
  svg.appendChild(el("rect", { x: 6, y: 6, width: W - 12, height: H - 12, rx: 9, fill: "none", stroke: "#cbbd97", "stroke-width": 1.5 }));
  // title cartouche
  svg.appendChild(el("rect", { x: 8, y: 8, width: Math.max(120, sc.name.length * 9 + 22), height: 32, rx: 4, fill: "rgba(20,16,10,.5)" }));
  txt(svg, 15, 25, 14, sc.name, "#f1dca0", 700, undefined, '"Cinzel", serif');
  txt(svg, 15, 36, 10, sc.meta, "#d8c6a0", 400);
  return H;
}
function tile(svg: G, b: SceneBldg, x: number, y: number, w: number, h: number, band: number) {
  svg.appendChild(el("rect", { x: (x + 4).toFixed(1), y: (y + 4).toFixed(1), width: (w - 8).toFixed(1), height: (h - 8).toFixed(1), rx: 7, fill: TILEG[band || 0], stroke: "#dccfa9", "stroke-width": 1 }));
  const cx = x + w / 2, baseY = y + h * 0.48, u = Math.min(w * 0.26, 24);
  plant(band || 0, svg, x + w * 0.13, baseY + u * 0.28, u * 0.85, false);
  plant(band || 0, svg, x + w * 0.87, baseY + u * 0.12, u * 0.7, true);
  iconFor(b.t)(svg, cx, baseY, u);
  if (b.goodIcon) goodBadge(svg, cx + u * 0.7, baseY - u * 1.5, b.goodIcon);
  txt(svg, cx, y + h * 0.70, 11.5, b.name, "#3f3120", 700, "middle");
  const owner = b.owner || "Settlement", isCity = /^Settlement$|^City of/.test(owner);
  txt(svg, cx, y + h * 0.83, 10, owner, isCity ? "#6a7c92" : "#b07b34", isCity ? 400 : 600, "middle");
  const chips: string[] = []; if (b.tier) chips.push("Tier " + b.tier); if (b.guild) chips.push("GUILD");
  if (chips.length) txt(svg, cx, y + h * 0.94, 8.5, chips.join("   ·   "), b.guild ? "#c89a5a" : "#8a7a58", 600, "middle");
}
function drawSea(g: G, x: number, y: number, w: number, h: number, seed: number) {
  const clip = "sea" + seed; const cp = el("clipPath", { id: clip }); cp.appendChild(el("rect", { x: x.toFixed(1), y: y.toFixed(1), width: w.toFixed(1), height: h.toFixed(1), rx: 6 })); g.appendChild(cp);
  const grp = el("g", { "clip-path": `url(#${clip})` }); g.appendChild(grp);
  grp.appendChild(el("rect", { x: x.toFixed(1), y: y.toFixed(1), width: w.toFixed(1), height: h.toFixed(1), fill: SEA[1] }));
  let d = `M${x.toFixed(1)} ${(y + 6).toFixed(1)} `; for (let i = 0; i <= w; i += 16) d += `q8 -5 16 0 `; d += `L${(x + w).toFixed(1)} ${y.toFixed(1)} L${x.toFixed(1)} ${y.toFixed(1)} Z`;
  grp.appendChild(el("path", { d, fill: SEA[0] }));
  let f = `M${x.toFixed(1)} ${(y + 6).toFixed(1)} `; for (let i = 0; i <= w; i += 16) f += `q8 -5 16 0 `;
  grp.appendChild(el("path", { d: f, fill: "none", stroke: "#dfeef4", "stroke-width": 1.4, "stroke-opacity": .8 }));
  for (let i = 0; i < w / 30; i++) grp.appendChild(el("line", { x1: (x + 10 + i * 30).toFixed(1), y1: (y + 16).toFixed(1), x2: (x + 18 + i * 30).toFixed(1), y2: (y + 16).toFixed(1), stroke: "#bfe0ec", "stroke-width": 1, "stroke-opacity": .5 }));
  boat(grp, x + w * 0.7, y + 18, 6);
  txt(g, x + w - 4, y + h - 4, 9, "⚓ harbour", "#dfeef4", 600, "end");
}
