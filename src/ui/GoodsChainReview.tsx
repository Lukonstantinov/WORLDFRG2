import { useMemo } from "react";
import { useGoodsStore } from "../state/goodsStore";
import { useUIStore } from "../state/uiStore";
import type { GoodSpec } from "../types";

const panel: React.CSSProperties = {
  position: "fixed", inset: 0, background: "rgba(0,0,0,0.6)", zIndex: 1100,
  display: "flex", alignItems: "center", justifyContent: "center",
};
const sheet: React.CSSProperties = {
  width: "min(1000px, 96vw)", maxHeight: "92vh", overflow: "auto",
  background: "#13181f", border: "1px solid #2c3442", borderRadius: 8,
  padding: 16, color: "#cfd8e3", fontSize: 12,
};
const btn: React.CSSProperties = {
  background: "#222b38", color: "#cfd8e3", border: "1px solid #36425a",
  borderRadius: 4, padding: "5px 11px", cursor: "pointer", fontSize: 12,
};
const primaryBtn: React.CSSProperties = { ...btn, background: "#2d7a3f", borderColor: "#3f9a53", color: "#fff", fontWeight: 600 };
const chip = (color: string): React.CSSProperties => ({
  display: "inline-flex", alignItems: "center", gap: 5, padding: "3px 8px", margin: 3,
  borderRadius: 12, background: "#1b2230", border: `1px solid ${color}55`,
});

/** Always-shown review before goods generation: planted/extracted vs manufactured
 *  goods, plus a schematic recipe DAG (depth = topo level, left→right). */
export function GoodsChainReview() {
  const open = useUIStore((s) => s.chainReviewOpen);
  const confirm = useUIStore((s) => s.chainReviewConfirm);
  const closeReview = useUIStore((s) => s.closeChainReview);
  const openEditor = useGoodsStore((s) => s.setOpen);
  const specs = useGoodsStore((s) => s.specs).filter((g) => g.enabled);

  const planted = specs.filter((g) => g.distribution !== "manufactured");
  const manufactured = specs.filter((g) => g.distribution === "manufactured");

  if (!open) return null;

  return (
    <div style={panel} onClick={closeReview}>
      <div style={sheet} onClick={(e) => e.stopPropagation()}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
          <strong style={{ fontSize: 15 }}>Goods &amp; Chains — review before generating</strong>
          <span style={{ color: "#7a8aa0" }}>{planted.length} planted · {manufactured.length} manufactured</span>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14, marginBottom: 14 }}>
          <Column title="🌾 Planted / Extracted (from the land)" note="climate-belt & deposit goods" goods={planted} />
          <Column title="🏭 Manufactured (in cities)" note="recipe goods — no map belt" goods={manufactured} />
        </div>

        <div style={{ fontWeight: 600, color: "#8aa0b8", marginBottom: 6 }}>Schematic — production chains (DAG)</div>
        <ChainGraph specs={specs} />

        <div style={{ display: "flex", gap: 8, marginTop: 14, justifyContent: "flex-end" }}>
          <button style={btn} onClick={() => { closeReview(); openEditor(true); }}>Edit goods…</button>
          <div style={{ flex: 1 }} />
          <button style={btn} onClick={closeReview}>Cancel</button>
          {confirm && (
            <button style={primaryBtn} onClick={() => { const fn = confirm; closeReview(); fn(); }}>
              Generate goods →
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function Column({ title, note, goods }: { title: string; note: string; goods: GoodSpec[] }) {
  return (
    <div style={{ background: "#0e1218", border: "1px solid #232b36", borderRadius: 6, padding: 10 }}>
      <div style={{ color: "#aeb9c8", marginBottom: 2 }}>{title}</div>
      <div style={{ color: "#5a6a80", fontSize: 10, marginBottom: 6 }}>{note}</div>
      <div>
        {goods.length === 0 && <span style={{ color: "#5a6a80" }}>none</span>}
        {goods.map((g) => (
          <span key={g.id} style={chip(g.color)} title={`${g.id} · value ${g.base_value} · bulk ${g.bulk ?? 1}`}>
            <span>{g.icon}</span><span>{g.name}</span>
          </span>
        ))}
      </div>
    </div>
  );
}

interface Node { id: string; depth: number; spec: GoodSpec; }

/** Lightweight layered DAG: depth = recipe level (raws at 0), columns left→right. */
function ChainGraph({ specs }: { specs: GoodSpec[] }) {
  const { nodes, edges, width, height } = useMemo(() => layout(specs), [specs]);
  if (nodes.length === 0) {
    return <div style={{ color: "#5a6a80", padding: "8px 0" }}>No production chains defined — every good is extracted from the land.</div>;
  }
  return (
    <div style={{ overflowX: "auto", background: "#0b0e13", border: "1px solid #232b36", borderRadius: 6, padding: 8 }}>
      <svg width={width} height={height} style={{ display: "block" }}>
        {edges.map((e, k) => {
          const x1 = e.from.x + NODE_W / 2, y1 = e.from.y;
          const x2 = e.to.x - NODE_W / 2, y2 = e.to.y;
          const mx = (x1 + x2) / 2;
          return (
            <g key={k}>
              <path d={`M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`}
                fill="none" stroke={e.bad ? "#c06060" : "#4a5a72"} strokeWidth={1.5} />
              <text x={mx} y={(y1 + y2) / 2 - 3} fill={e.bad ? "#e08080" : "#8aa0b8"} fontSize={9} textAnchor="middle">
                {e.bad ? "?" : `×${round(e.qty)}`}
              </text>
            </g>
          );
        })}
        {nodes.map((n) => (
          <g key={n.id}>
            <rect x={n.x - NODE_W / 2} y={n.y - NODE_H / 2} width={NODE_W} height={NODE_H} rx={6}
              fill="#161d28" stroke={n.spec.color} strokeWidth={1.5} />
            <text x={n.x} y={n.y - 2} fill="#e3e9f0" fontSize={11} textAnchor="middle">
              {n.spec.icon} {trim(n.spec.name)}
            </text>
            <text x={n.x} y={n.y + 11} fill="#7a8aa0" fontSize={8} textAnchor="middle">
              v{round(n.spec.base_value)} · b{round(n.spec.bulk ?? 1)}{(n.spec.perishable ?? 0) > 0 ? ` · p${round(n.spec.perishable ?? 0)}` : ""}
            </text>
          </g>
        ))}
      </svg>
    </div>
  );
}

const NODE_W = 116, NODE_H = 34, COL_GAP = 70, ROW_GAP = 14, PAD = 16;

function layout(specs: GoodSpec[]) {
  const byId = new Map(specs.map((s) => [s.id, s]));
  const isRecipe = (s?: GoodSpec) => !!s && s.distribution === "manufactured" && (s.inputs?.length ?? 0) > 0;
  // Depth via relaxation; nodes in the graph = manufactured goods ∪ their inputs.
  const inGraph = new Set<string>();
  for (const s of specs) {
    if (isRecipe(s)) {
      inGraph.add(s.id);
      for (const inp of s.inputs ?? []) inGraph.add(inp.good);
    }
  }
  const depth = new Map<string, number>();
  for (const id of inGraph) depth.set(id, 0);
  for (let pass = 0; pass < inGraph.size + 1; pass++) {
    let changed = false;
    for (const id of inGraph) {
      const s = byId.get(id);
      if (!isRecipe(s)) continue;
      // A manufactured good sits one column to the RIGHT of every input it
      // consumes — raw inputs (depth 0) push the product to depth 1, a product
      // built from another product lands at depth 2, etc. (Previously depth only
      // advanced for inputs that were themselves recipes, so a plain
      // raw→product chain left both nodes at depth 0 and the whole graph
      // collapsed into a single tangled column.)
      let d = 1;
      for (const inp of s!.inputs ?? []) {
        if (inGraph.has(inp.good)) d = Math.max(d, (depth.get(inp.good) ?? 0) + 1);
      }
      if (depth.get(id) !== d) { depth.set(id, d); changed = true; }
    }
    if (!changed) break;
  }
  // Columns by depth.
  const maxDepth = Math.max(0, ...[...depth.values()]);
  const cols: string[][] = Array.from({ length: maxDepth + 1 }, () => []);
  for (const id of inGraph) cols[depth.get(id) ?? 0].push(id);
  cols.forEach((c) => c.sort());

  const nodes: (Node & { x: number; y: number })[] = [];
  const pos = new Map<string, { x: number; y: number }>();
  cols.forEach((col, d) => {
    col.forEach((id, r) => {
      const x = PAD + NODE_W / 2 + d * (NODE_W + COL_GAP);
      const y = PAD + NODE_H / 2 + r * (NODE_H + ROW_GAP);
      const spec = byId.get(id) ?? fallbackSpec(id);
      pos.set(id, { x, y });
      nodes.push({ id, depth: d, spec, x, y });
    });
  });

  const edges: { from: { x: number; y: number }; to: { x: number; y: number }; qty: number; bad: boolean }[] = [];
  for (const id of inGraph) {
    const s = byId.get(id);
    if (!isRecipe(s)) continue;
    const to = pos.get(id)!;
    for (const inp of s!.inputs ?? []) {
      const from = pos.get(inp.good);
      const bad = !byId.get(inp.good)?.enabled;
      if (from) edges.push({ from, to, qty: inp.qty, bad });
    }
  }

  const width = PAD * 2 + (maxDepth + 1) * NODE_W + maxDepth * COL_GAP;
  const tallest = Math.max(1, ...cols.map((c) => c.length));
  const height = PAD * 2 + tallest * NODE_H + (tallest - 1) * ROW_GAP;
  return { nodes, edges, width, height };
}

function fallbackSpec(id: string): GoodSpec {
  return { id, name: id, icon: "❔", color: "#c06060", enabled: false, domain: "continental",
    distribution: "local", rarity: 0.5, desire: 0.4, network_luxury: false, builtin: false,
    category: "misc", need_tier: 1, base_value: 1 };
}

function round(v: number) { return Math.round(v * 10) / 10; }
function trim(s: string) { return s.length > 13 ? s.slice(0, 12) + "…" : s; }
