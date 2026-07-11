#!/usr/bin/env node
// Generate the standalone HTML fish gallery for WorldForge 2.
//
//   node scripts/gen_fish_gallery.mjs
//
// Reads scripts/fish_catalogue.json (dumped from the Rust catalogue by the
// `emit_fish_catalogue` test) and writes, into public/fish/:
//   - <slug>.html   one browsable page per species (inline parametric SVG plate)
//   - index.html    a grouped gallery linking them all
//
// The SVG is drawn parametrically from each fish's body class (salmonid /
// cyprinid / catfish / sturgeon / percid / pike / eel / cichlid) + a hue derived
// from its slug, so every species is visually distinct and deterministic.

import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const OUT = join(here, "..", "public", "fish");
mkdirSync(OUT, { recursive: true });

const fish = JSON.parse(readFileSync(join(here, "fish_catalogue.json"), "utf8"));

// ── helpers ───────────────────────────────────────────────────────────────────
const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const ZONE = ["headwater / upper river", "middle river", "lower river / delta"];

function hash(str) { let h = 2166136261 >>> 0; for (const c of str) { h ^= c.charCodeAt(0); h = Math.imul(h, 16777619); } return h >>> 0; }
function hueOf(slug) { return hash(slug) % 360; }

function bodyClass(real, slug) {
  const r = (real + " " + slug).toLowerCase();
  if (/shrimp/.test(r)) return "shrimp";
  if (/seal|nerpa/.test(r)) return "seal";
  if (/sturgeon|sterlet/.test(r)) return "sturgeon";
  if (/\beel\b/.test(r)) return "eel";
  if (/pike(?!-?perch)/.test(r) && !/perch/.test(r)) return "pike";
  if (/perch|zander|pike-perch|ruffe/.test(r)) return "percid";
  if (/catfish|wels|redtail|sculpin|bullhead|loach/.test(r)) return "catfish";
  if (/trout|char|grayling|salmon|taimen|whitefish|cisco|omul|mullet/.test(r)) return "salmonid";
  if (/cichlid|tilapia|eartheater|mbuna|arapaima|snook|bass|lungfish/.test(r)) return "cichlid";
  return "cyprinid"; // roach, bream, carp, barbel, tetra, pacu, dollar, bocachico…
}

// Per-class silhouette parameters (facing LEFT).
const CLASS = {
  salmonid: { h: 46, headX: 150, tailX: 420, taper: 0.38, dorsal: "single", adipose: true },
  cyprinid: { h: 66, headX: 158, tailX: 412, taper: 0.40, dorsal: "single" },
  catfish:  { h: 40, headX: 138, tailX: 430, taper: 0.30, dorsal: "single", barbels: true, flathead: true },
  sturgeon: { h: 34, headX: 128, tailX: 432, taper: 0.28, dorsal: "single", barbels: true, scutes: true, snout: 26 },
  percid:   { h: 58, headX: 158, tailX: 414, taper: 0.42, dorsal: "double", spiny: true },
  pike:     { h: 40, headX: 120, tailX: 424, taper: 0.34, dorsal: "back", ducksnout: true },
  eel:      { h: 26, headX: 120, tailX: 446, taper: 0.70, dorsal: "long", ribbon: true },
  cichlid:  { h: 72, headX: 168, tailX: 408, taper: 0.46, dorsal: "long", spiny: true },
  seal:     { h: 70, headX: 150, tailX: 430, taper: 0.5, seal: true },
  shrimp:   { h: 40, headX: 170, tailX: 410, shrimp: true },
};

function fishSVG(f, w = 600, hgt = 340) {
  const cls = bodyClass(f.real, f.slug);
  const P = CLASS[cls] ?? CLASS.cyprinid;
  const hue = hueOf(f.slug);
  const cy = hgt / 2;
  const id = "g" + hash(f.slug).toString(36);
  const bodyTop = `hsl(${hue} 45% 52%)`;
  const bodyMid = `hsl(${hue} 55% 42%)`;
  const belly = `hsl(${(hue + 30) % 360} 35% 78%)`;
  const fin = `hsl(${hue} 60% 34%)`;
  const finEdge = `hsl(${hue} 65% 26%)`;

  if (P.shrimp) {
    return shrimpSVG(f, w, hgt, hue, id, bodyMid, fin);
  }

  const { h, headX, tailX, taper } = P;
  const midX = (headX + tailX) / 2;
  const snout = P.snout ?? (P.ducksnout ? 8 : 12);
  const noseX = headX - snout;
  const th = h * taper; // half-height at the tail wrist

  // Body outline (closed): nose → over the back → tail wrist → under the belly.
  const body =
    `M ${noseX} ${cy} ` +
    `C ${headX - 6} ${cy - h * 0.75}, ${midX - 40} ${cy - h}, ${midX + 30} ${cy - h * 0.9} ` +
    `S ${tailX - 10} ${cy - th * 1.15}, ${tailX} ${cy - th} ` +
    `L ${tailX} ${cy + th} ` +
    `C ${tailX - 10} ${cy + th * 1.15}, ${midX + 30} ${cy + h * 0.9}, ${midX - 40} ${cy + h} ` +
    `S ${headX - 6} ${cy + h * 0.75}, ${noseX} ${cy} Z`;

  // Caudal fin.
  const teX = tailX + (P.ribbon ? 26 : 62);
  const spread = P.ribbon ? th * 1.4 : h * 1.0;
  const tail = P.ribbon
    ? `M ${tailX} ${cy - th} Q ${teX} ${cy - spread * 0.5} ${teX} ${cy} Q ${teX} ${cy + spread * 0.5} ${tailX} ${cy + th} Z`
    : `M ${tailX} ${cy - th} L ${teX} ${cy - spread} L ${tailX + 20} ${cy} L ${teX} ${cy + spread} L ${tailX} ${cy + th} Z`;

  // Dorsal fin(s).
  let dorsal = "";
  if (P.dorsal === "double") {
    dorsal =
      `<path d="M ${midX - 46} ${cy - h + 6} L ${midX - 20} ${cy - h - 30} L ${midX + 4} ${cy - h + 4} Z" fill="${fin}" stroke="${finEdge}"/>` +
      `<path d="M ${midX + 12} ${cy - h + 4} Q ${midX + 42} ${cy - h - 16} ${midX + 70} ${cy - h * 0.7} Z" fill="${fin}" stroke="${finEdge}"/>`;
  } else if (P.dorsal === "long") {
    dorsal = `<path d="M ${headX + 40} ${cy - h + 8} Q ${midX} ${cy - h - 34} ${tailX - 28} ${cy - th - 6} L ${tailX - 30} ${cy - th + 6} Q ${midX} ${cy - h + 12} ${headX + 46} ${cy - h + 18} Z" fill="${fin}" stroke="${finEdge}"/>`;
  } else if (P.dorsal === "back") {
    dorsal = `<path d="M ${tailX - 90} ${cy - th - 4} Q ${tailX - 55} ${cy - th - 34} ${tailX - 20} ${cy - th} Z" fill="${fin}" stroke="${finEdge}"/>`;
  } else {
    dorsal = `<path d="M ${midX - 34} ${cy - h + 4} Q ${midX} ${cy - h - 40} ${midX + 40} ${cy - h + 8} Z" fill="${fin}" stroke="${finEdge}"/>`;
  }
  if (P.adipose) dorsal += `<path d="M ${tailX - 40} ${cy - th - 2} q 12 -8 20 0 Z" fill="${fin}"/>`;

  // Anal + pectoral fins.
  const anal = `<path d="M ${midX + 4} ${cy + h - 4} Q ${midX + 24} ${cy + h + 26} ${midX + 48} ${cy + h - 8} Z" fill="${fin}" stroke="${finEdge}"/>`;
  const pect = `<path d="M ${headX + 20} ${cy + 8} Q ${headX + 40} ${cy + 44} ${headX + 66} ${cy + 20} Z" fill="${fin}" opacity="0.92" stroke="${finEdge}"/>`;

  // Barbels (catfish / sturgeon whiskers) + snout scutes.
  let extras = "";
  if (P.barbels) {
    extras += `<path d="M ${noseX + 2} ${cy - 4} q -30 6 -46 26" fill="none" stroke="${finEdge}" stroke-width="2.4" stroke-linecap="round"/>`;
    extras += `<path d="M ${noseX + 2} ${cy + 6} q -34 8 -40 30" fill="none" stroke="${finEdge}" stroke-width="2.4" stroke-linecap="round"/>`;
  }
  if (P.scutes) {
    for (let i = 0; i < 6; i++) {
      const x = headX + 20 + i * ((tailX - headX - 30) / 6);
      extras += `<path d="M ${x} ${cy - h * 0.55} l 7 -7 l 7 7 z" fill="${finEdge}" opacity="0.7"/>`;
    }
  }

  // Eye + gill + a couple of scale/lateral hints.
  const eyeX = headX + (P.flathead ? 22 : 16), eyeY = cy - h * 0.22;
  const eye = `<circle cx="${eyeX}" cy="${eyeY}" r="7" fill="#f6f3ea"/><circle cx="${eyeX - 1}" cy="${eyeY}" r="3.6" fill="#12202c"/>`;
  const gill = `<path d="M ${headX + 34} ${cy - h * 0.55} Q ${headX + 24} ${cy} ${headX + 34} ${cy + h * 0.55}" fill="none" stroke="${finEdge}" stroke-width="2" opacity="0.6"/>`;
  const lateral = `<path d="M ${headX + 40} ${cy - 2} Q ${midX} ${cy - 6} ${tailX - 10} ${cy}" fill="none" stroke="${finEdge}" stroke-width="1.6" opacity="0.45"/>`;

  return `<svg viewBox="0 0 ${w} ${hgt}" width="100%" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="${esc(f.name)}">
  <defs>
    <linearGradient id="${id}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="${bodyTop}"/>
      <stop offset="0.55" stop-color="${bodyMid}"/>
      <stop offset="1" stop-color="${belly}"/>
    </linearGradient>
  </defs>
  <rect width="${w}" height="${hgt}" fill="none"/>
  <path d="${tail}" fill="${fin}" stroke="${finEdge}" stroke-width="2"/>
  ${dorsal}${anal}
  <path d="${body}" fill="url(#${id})" stroke="${finEdge}" stroke-width="2.5"/>
  ${pect}${extras}${gill}${lateral}${eye}
</svg>`;
}

function shrimpSVG(f, w, hgt, hue, id, col, fin) {
  const cy = hgt / 2;
  return `<svg viewBox="0 0 ${w} ${hgt}" width="100%" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="${esc(f.name)}">
  <path d="M 430 ${cy - 40} C 300 ${cy - 90}, 170 ${cy - 40}, 150 ${cy + 10} C 200 ${cy + 70}, 340 ${cy + 60}, 430 ${cy + 20} C 470 ${cy}, 470 ${cy - 20}, 430 ${cy - 40} Z" fill="${col}" stroke="${fin}" stroke-width="2.5"/>
  ${Array.from({ length: 6 }, (_, i) => `<path d="M ${420 - i * 42} ${cy - 30} q -6 -30 -18 -46" fill="none" stroke="${fin}" stroke-width="2.5" stroke-linecap="round"/>`).join("")}
  <circle cx="420" cy="${cy - 26}" r="6" fill="#f6f3ea"/><circle cx="419" cy="${cy - 26}" r="3" fill="#12202c"/>
  <path d="M 430 ${cy - 34} q 40 -18 66 -6 M 430 ${cy - 26} q 44 -6 70 8" fill="none" stroke="${fin}" stroke-width="2" stroke-linecap="round"/>
</svg>`;
}

// ── page template ───────────────────────────────────────────────────────────
const CSS = `
:root{color-scheme:dark}
*{box-sizing:border-box}
body{margin:0;font:15px/1.6 -apple-system,Segoe UI,Roboto,sans-serif;background:#070f16;color:#d7e6f2}
a{color:#7fc8e0;text-decoration:none}
a:hover{text-decoration:underline}
.wrap{max-width:1080px;margin:0 auto;padding:26px 20px 60px}
.plate{background:radial-gradient(130% 100% at 30% 18%,#14304a,#0a1620 72%);border:1px solid #16344c;border-radius:14px;padding:18px}
.chip{display:inline-block;background:#0e2233;border:1px solid #1d3d55;border-radius:999px;padding:2px 10px;margin:3px 5px 0 0;font-size:12px;color:#a9c4d8}
.bin{font-style:italic;color:#7fae8f}
h1{font-size:26px;margin:0 0 2px}
.muted{color:#8aa0b8}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(210px,1fr));gap:14px;margin-top:14px}
.card{background:#0a1620;border:1px solid #16283a;border-radius:11px;padding:10px;transition:border-color .15s}
.card:hover{border-color:#2b567a}
.card .thumb{background:radial-gradient(130% 100% at 30% 20%,#14304a,#0a1620 70%);border-radius:8px;overflow:hidden}
.card b{display:block;margin-top:7px;color:#e6f2fb;font-size:13.5px}
.card .sub{color:#7fae8f;font-style:italic;font-size:11.5px}
h2{margin:34px 0 4px;font-size:16px;color:#9fc0da;border-bottom:1px solid #16283a;padding-bottom:6px}
`;

const zoneLabel = (g, z) => g === "lake" ? "lake species" : (ZONE[z] ?? "river");
const prettyBands = (b) => b.map((x) => x.replace(/([a-z])([A-Z])/g, "$1 $2")).join(" · ");

// Prefer a real picture at ./<slug>.png (drop your artwork in public/fish/); fall
// back to the parametric SVG. Same slug the in-app Hydrology plate loads, so one
// image file lights up BOTH the gallery and the app.
function art(f, w, h) {
  return `<img src="./${f.slug}.png" alt="${esc(f.name)}" style="width:100%;display:block" ` +
    `onerror="this.style.display='none';this.nextElementSibling.style.display='block'">` +
    `<div style="display:none">${fishSVG(f, w, h)}</div>`;
}

function page(f) {
  return `<!doctype html><html lang="en"><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>${esc(f.name)} — WorldForge fish</title><style>${CSS}</style>
<div class="wrap">
  <p><a href="./index.html">← All fish</a></p>
  <div class="plate">${art(f, 600, 340)}</div>
  <h1>${esc(f.name)}</h1>
  <div class="bin">${esc(f.binomial)}</div>
  <p><span class="chip">🌡 ${esc(prettyBands(f.bands))}</span><span class="chip">📍 ${esc(zoneLabel(f.group, f.zone))}</span><span class="chip">≈ real ${esc(f.real)}</span></p>
  <p class="muted">${esc(f.blurb)}</p>
</div></html>`;
}

// Assign a display section.
function section(f) {
  if (f.group === "arid") return "Desert & oasis";
  if (f.group === "lake") return "Lakes";
  const b = f.bands;
  if (b.includes("Tropical") && !b.includes("CoolTemperate")) return "Tropical rivers";
  if ((b.includes("Boreal") || b.includes("Polar")) && !b.includes("WarmTemperate")) return "Boreal & subarctic rivers";
  return "Temperate rivers";
}
const SECTION_ORDER = ["Temperate rivers", "Tropical rivers", "Boreal & subarctic rivers", "Desert & oasis", "Lakes"];

function indexPage(list) {
  const bySec = new Map();
  for (const f of list) { const s = section(f); (bySec.get(s) ?? bySec.set(s, []).get(s)).push(f); }
  const secHtml = SECTION_ORDER.filter((s) => bySec.has(s)).map((s) => {
    const cards = bySec.get(s).sort((a, b) => a.zone - b.zone || a.name.localeCompare(b.name)).map((f) =>
      `<a class="card" href="./${f.slug}.html"><div class="thumb">${art(f, 300, 150)}</div><b>${esc(f.name)}</b><span class="sub">${esc(f.real)} · ${esc(zoneLabel(f.group, f.zone))}</span></a>`
    ).join("");
    return `<h2>${esc(s)} <span class="muted" style="font-weight:400">(${bySec.get(s).length})</span></h2><div class="grid">${cards}</div>`;
  }).join("");
  return `<!doctype html><html lang="en"><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>WorldForge 2 — Fish of the Rivers & Lakes</title><style>${CSS}</style>
<div class="wrap">
  <h1>🐟 Fish of the Rivers &amp; Lakes</h1>
  <p class="muted">${list.length} freshwater species of WorldForge 2 — the fish that populate each river reach (upper / middle / delta) and each lake type. Click any fish for its plate.</p>
  ${secHtml}
</div></html>`;
}

// ── emit ────────────────────────────────────────────────────────────────────
let n = 0;
for (const f of fish) { writeFileSync(join(OUT, `${f.slug}.html`), page(f)); n++; }
writeFileSync(join(OUT, "index.html"), indexPage(fish));
console.log(`Wrote ${n} fish pages + index.html to public/fish/`);
