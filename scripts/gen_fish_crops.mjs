#!/usr/bin/env node
// Compute, per gallery section, the deterministic ordered slug list (the same
// order the Gemini sheets were rendered in) and flag which slugs still lack a
// public/fish/<slug>.png. Prints the missing ones with their (section, col, row)
// at 4 columns — the crop map for gen crops.
import { readFileSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const OUT = join(here, "..", "public", "fish");
const fish = JSON.parse(readFileSync(join(here, "fish_catalogue.json"), "utf8"));

// Must match scripts/gen_fish_gallery.mjs exactly.
function section(f) {
  if (f.group === "arid") return "Desert & oasis";
  if (f.group === "lake") return "Lakes";
  const b = f.bands;
  if (b.includes("Tropical") && !b.includes("CoolTemperate")) return "Tropical rivers";
  if ((b.includes("Boreal") || b.includes("Polar")) && !b.includes("WarmTemperate")) return "Boreal & subarctic rivers";
  return "Temperate rivers";
}
const SECTION_ORDER = ["Temperate rivers", "Tropical rivers", "Boreal & subarctic rivers", "Desert & oasis", "Lakes"];
const COLS = 4;

const bySec = new Map();
for (const f of fish) { const s = section(f); (bySec.get(s) ?? bySec.set(s, []).get(s)).push(f); }

const rows = [];
for (const s of SECTION_ORDER) {
  const list = (bySec.get(s) ?? []).sort((a, b) => a.zone - b.zone || a.name.localeCompare(b.name));
  list.forEach((f, i) => {
    const has = existsSync(join(OUT, `${f.slug}.png`));
    rows.push({ section: s, idx: i, col: i % COLS, row: Math.floor(i / COLS), slug: f.slug, name: f.name, has });
  });
}

const missing = rows.filter((r) => !r.has);
console.log("=== FULL ORDER (per section) ===");
let cur = "";
for (const r of rows) {
  if (r.section !== cur) { cur = r.section; console.log(`\n[${cur}] (${(bySec.get(cur) ?? []).length})`); }
  console.log(`  r${r.row} c${r.col}  ${r.has ? "   " : "NEW"} ${r.slug}  — ${r.name}`);
}
console.log(`\n=== MISSING (${missing.length}) ===`);
console.log(JSON.stringify(missing.map((r) => ({ section: r.section, col: r.col, row: r.row, slug: r.slug, name: r.name })), null, 0));
