import { useMemo } from "react";
import { useGoodsStore } from "@state/goodsStore";
import { useUIStore } from "@state/uiStore";
import { goodCategory, CATEGORY_ORDER } from "@goods";
import type { GoodSpec } from "@types";

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
  const allSpecs = useGoodsStore((s) => s.specs);
  const specs = allSpecs.filter((g) => g.enabled);

  // Every enabled good is classified by HOW it reaches the market:
  //  • Planted/Grown   — climate-belt crops (global/local distribution)
  //  • Extracted       — deposit goods dug from the ground (ores, gems, salt)
  //  • Manufactured    — finished goods made in cities from a recipe (no belt)
  const planted = specs.filter((g) => g.distribution === "global" || g.distribution === "local");
  const extracted = specs.filter((g) => g.distribution === "deposits");
  const manufactured = specs.filter((g) => g.distribution === "manufactured");

  if (!open) return null;

  return (
    <div style={panel} onClick={closeReview}>
      <div style={sheet} onClick={(e) => e.stopPropagation()}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
          <strong style={{ fontSize: 15 }}>Goods &amp; Chains — review before generating</strong>
          <span style={{ color: "#7a8aa0" }}>
            {planted.length} planted · {extracted.length} extracted · {manufactured.length} manufactured
            <span style={{ color: "#5a6a80" }}> · {specs.length}/{allSpecs.length} enabled</span>
          </span>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12, marginBottom: 14 }}>
          <Column title="🌾 Planted / Grown" note="climate-belt crops, seeded independently" goods={planted} />
          <Column title="🪨 Extracted" note="ore / gem / salt deposits" goods={extracted} />
          <Column title="🏭 Manufactured" note="made in cities — no map belt" goods={manufactured} />
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

/** A production-type column, with its goods sub-grouped by trade category
 *  (category-within-type), each good shown as a distinctly-coloured chip. */
function Column({ title, note, goods }: { title: string; note: string; goods: GoodSpec[] }) {
  const byCat = CATEGORY_ORDER
    .map((cat) => ({ cat, items: goods.filter((g) => goodCategory(g.id) === cat) }))
    .filter((g) => g.items.length > 0);
  return (
    <div style={{ background: "#0e1218", border: "1px solid #232b36", borderRadius: 6, padding: 10 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
        <span style={{ color: "#aeb9c8" }}>{title}</span>
        <span style={{ color: "#5a6a80", fontSize: 11 }}>{goods.length}</span>
      </div>
      <div style={{ color: "#5a6a80", fontSize: 10, marginBottom: 6 }}>{note}</div>
      {goods.length === 0 && <span style={{ color: "#5a6a80" }}>none</span>}
      {byCat.map(({ cat, items }) => (
        <div key={cat} style={{ marginBottom: 5 }}>
          <div style={{ color: "#5f7390", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4, marginBottom: 1 }}>
            {cat} · {items.length}
          </div>
          <div>
            {items.map((g) => (
              <span key={g.id} onClick={() => useUIStore.getState().setGoodDetail(g.id)}
                style={{ ...chip(g.color), cursor: "pointer" }}
                title={`${g.id} · value ${g.base_value} · bulk ${g.bulk ?? 1} — click for climates & heatmap`}>
                <span>{g.icon}</span><span>{g.name}</span>
              </span>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

/** Production chains as plain recipe EQUATIONS — one row per manufactured good,
 *  `input (+ input) → product · tier`, grouped by the product's category. This
 *  replaced an SVG bipartite DAG that always tangled into one unreadable column
 *  (every product pulled from one shared pool of raws). */
function ChainGraph({ specs }: { specs: GoodSpec[] }) {
  const byId = useMemo(() => new Map(specs.map((s) => [s.id, s])), [specs]);
  const recipes = useMemo(
    () => specs.filter((s) => s.distribution === "manufactured" && (s.inputs?.length ?? 0) > 0),
    [specs],
  );
  if (recipes.length === 0) {
    return <div style={{ color: "#5a6a80", padding: "8px 0" }}>No production chains defined — every good is extracted or grown from the land.</div>;
  }
  const cats = CATEGORY_ORDER.filter((c) => recipes.some((r) => goodCategory(r.id) === c));
  return (
    <div style={{ background: "#0b0e13", border: "1px solid #232b36", borderRadius: 6, padding: "8px 10px" }}>
      {cats.map((cat) => (
        <div key={cat} style={{ marginBottom: 7 }}>
          <div style={{ color: "#5f7390", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 2 }}>{cat}</div>
          {recipes.filter((r) => goodCategory(r.id) === cat).map((r) => (
            <RecipeRow key={r.id} product={r} byId={byId} />
          ))}
        </div>
      ))}
    </div>
  );
}

/** One recipe equation: `🐑 Wool + 🐚 Dyes → 🧶 Woolen Cloth · luxury`. */
function RecipeRow({ product, byId }: { product: GoodSpec; byId: Map<string, GoodSpec> }) {
  const inputs = product.inputs ?? [];
  const tier = product.network_luxury ? "luxury" : ((product.need_tier ?? 0) >= 1 ? "comfort" : "common");
  const tierColor = product.network_luxury ? "#d9b06a" : "#6a7a90";
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 3, flexWrap: "wrap", padding: "3px 0", borderBottom: "1px solid #161d27" }}>
      {inputs.map((inp, k) => {
        const s = byId.get(inp.good);
        const bad = !s?.enabled;
        return (
          <span key={inp.good} style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
            {k > 0 && <span style={{ color: "#5a6a80", margin: "0 1px" }}>+</span>}
            <span style={chip(s?.color ?? "#c06060")} title={bad ? `${inp.good} — disabled or missing!` : inp.good}>
              <span>{s?.icon ?? "❔"}</span>
              <span style={{ color: bad ? "#e08080" : undefined }}>{trim(s?.name ?? inp.good)}</span>
              {inp.qty !== 1 && <span style={{ color: "#6a7a90", fontSize: 10 }}>×{round(inp.qty)}</span>}
            </span>
          </span>
        );
      })}
      <span style={{ color: "#7a8aa0", margin: "0 4px", fontWeight: 700 }}>→</span>
      <span style={chip(product.color)} title={`${product.id} · value ${round(product.base_value)} · bulk ${round(product.bulk ?? 1)}`}>
        <span>{product.icon}</span><span style={{ fontWeight: 600 }}>{trim(product.name)}</span>
      </span>
      <span style={{ color: tierColor, fontSize: 10, marginLeft: 2 }}>· {tier}</span>
    </div>
  );
}

function round(v: number) { return Math.round(v * 10) / 10; }
function trim(s: string) { return s.length > 15 ? s.slice(0, 14) + "…" : s; }
