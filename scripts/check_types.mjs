// Rust <-> TypeScript drift check (IMPROVEMENT_PLAN I-9). The TS types in src/types/
// are written by hand to mirror Rust serde structs, so a renamed or removed Rust field
// used to fail silently: the UI read `undefined`. This fails when a TS interface
// declares a field its same-named Rust struct does not send. Field NAMES only, no deps.
// A Rust field the UI does not declare is fine (the UI just ignores it).
//   node scripts/check_types.mjs
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const walk = (d) => readdirSync(d).flatMap((f) => {
  const p = join(d, f);
  return statSync(p).isDirectory() ? walk(p) : [p];
});
const noComments = (s) => s.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/[^\n]*/g, "");

// The body of the `{...}` block opening at index i (the index of the `{`).
function block(s, i) {
  let depth = 0;
  for (let j = i; j < s.length; j++) {
    if (s[j] === "{") depth++;
    else if (s[j] === "}" && --depth === 0) return s.slice(i + 1, j);
  }
  return "";
}
// Split a body on top-level commas/semicolons (and newlines for TS, whose fields may omit `;`) (ignores <>, (), [], {} nesting).
function topLevel(body, newlines) {
  const out = []; let depth = 0, cur = "";
  for (const ch of body) {
    if ("<([{".includes(ch)) depth++;
    if (">)]}".includes(ch)) depth--;
    if ((ch === "," || ch === ";" || (newlines && ch === "\n")) && depth === 0) { out.push(cur); cur = ""; } else cur += ch;
  }
  out.push(cur);
  return out.map((x) => x.trim()).filter(Boolean);
}
const camel = (n) => n.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase());

// ── Rust: every serde-Serialize struct with named fields ──
const rust = new Map(); // name -> [Set of field names] (one per same-named struct)
for (const f of walk("src-tauri/src").filter((p) => p.endsWith(".rs"))) {
  const src = noComments(readFileSync(f, "utf8"));
  const re = /((?:#\[[^\]]*\]\s*)*)pub(?:\([^)]*\))?\s+struct\s+(\w+)(?:<[^>]*>)?\s*\{/g;
  for (let m; (m = re.exec(src));) {
    const [, attrs, name] = m;
    if (!/derive\([^)]*Serialize/.test(attrs)) continue;
    const renameAll = /rename_all\s*=\s*"camelCase"/.test(attrs);
    const fields = new Set();
    // Attributes stay glued to the field they precede (they contain no top-level separators).
    for (const item of topLevel(block(src, m.index + m[0].length - 1), false)) {
      const fm = item.match(/^((?:#\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?(?:r#)?(\w+)\s*:/);
      if (!fm) continue;
      const [, fattrs, fname] = fm;
      if (/serde\([^)]*\bskip\b(?!_)/.test(fattrs) || /skip_serializing\b(?!_)/.test(fattrs)) continue;
      const rn = fattrs.match(/serde\([^)]*\brename\s*=\s*"([^"]+)"/);
      fields.add(rn ? rn[1] : renameAll ? camel(fname) : fname);
    }
    if (!rust.has(name)) rust.set(name, []);
    rust.get(name).push(fields);
  }
}

// ── TypeScript: every exported interface / object type alias ──
const ts = new Map(); // name -> { fields: Map(name -> optional), ext: [names], file }
for (const f of walk("src/types").filter((p) => p.endsWith(".ts"))) {
  const src = noComments(readFileSync(f, "utf8"));
  const re = /export\s+(?:interface\s+(\w+)(?:<[^>{]*>)?(?:\s+extends\s+([^{]+))?|type\s+(\w+)(?:<[^>=]*>)?\s*=)\s*\{/g;
  for (let m; (m = re.exec(src));) {
    const name = m[1] || m[3];
    const fields = new Map();
    for (const item of topLevel(block(src, m.index + m[0].length - 1), true)) {
      const fm = item.match(/^(?:readonly\s+)?["']?(\w+)["']?(\?)?\s*:/);
      if (fm) fields.set(fm[1], !!fm[2]);
    }
    const ext = (m[2] || "").split(",").map((x) => x.trim().replace(/<.*$/, "")).filter(Boolean);
    ts.set(name, { fields, ext, file: f });
  }
}
const allFields = (name, seen = new Set()) => {
  const t = ts.get(name);
  if (!t || seen.has(name)) return new Map();
  seen.add(name);
  const out = new Map();
  for (const e of t.ext) for (const [k, v] of allFields(e, seen)) out.set(k, v);
  for (const [k, v] of t.fields) out.set(k, v);
  return out;
};

// Frontend-only fields, added client-side after the IPC call. Keep this list short
// and say why; anything else here is real drift.
const CLIENT_ONLY = new Set([
  "MerchantRoute.path", "FuturesLane.path", // snapped to the road network by OverlayManager
  // MapCanvas decorates campaign hubs into map Settlements:
  "Settlement.dead", "Settlement.isNew", "Settlement.hubClass", "Settlement.isTransitKnot", "Settlement.relayCount",
]);

let checked = 0, drift = 0;
for (const [name, t] of ts) {
  const variants = rust.get(name);
  if (!variants) continue;
  checked++;
  const fields = allFields(name);
  // Same-named Rust structs in different modules: pass if ANY one matches.
  const misses = variants.map((rf) => [...fields.keys()].filter((k) => !rf.has(k) && !CLIENT_ONLY.has(`${name}.${k}`)));
  const best = misses.reduce((a, b) => (b.length < a.length ? b : a));
  if (best.length) {
    drift++;
    console.log(`${t.file}  ${name}: Rust never sends ${best.map((k) => k + (fields.get(k) ? "?" : "")).join(", ")}`);
  }
}
console.log(`checked ${checked} TS types against same-named Rust structs; ${drift} with drift`);
process.exit(drift ? 1 : 0);
