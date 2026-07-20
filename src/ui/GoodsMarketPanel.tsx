import { useEffect, useMemo, useState } from "react";
import { useCampaignStore } from "@state/campaignStore";
import { useUIStore } from "@state/uiStore";
import { campaignGetGoods } from "@bridge";
import { GOOD_DEFS } from "@goods";
import type { GoodMarketRow } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";
import { T, FZ } from "./chronicleTheme";
import { Panel, PanelHeader, PanelBody, Chip, EmptyNote } from "./kit";

const GICON = new Map(GOOD_DEFS.map((g) => [g.name, g.emoji]));
const GLABEL = new Map(GOOD_DEFS.map((g) => [g.name, g.label]));
const icon = (n: string) => GICON.get(n) ?? "\u{1F4E6}";
const label = (n: string) => GLABEL.get(n) ?? n;
const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(v < 10 ? 1 : 0));

/** Grade colour ramp (Coarse→Exquisite), matching the worldgen ladder. */
function gradeColor(q: number): string {
  const stops: [number, [number, number, number]][] = [
    [0, [150, 120, 110]], [0.32, [170, 150, 110]], [0.5, [150, 170, 120]],
    [0.68, [110, 180, 150]], [0.85, [120, 190, 230]], [1, [150, 205, 245]],
  ];
  for (let i = 0; i < stops.length - 1; i++) {
    const [a, ac] = stops[i], [b, bc] = stops[i + 1];
    if (q <= b) { const t = (q - a) / Math.max(1e-6, b - a); return `rgb(${ac.map((c, j) => Math.round(c + (bc[j] - c) * t)).join(",")})`; }
  }
  return "rgb(150,205,245)";
}

/** Grade colour by name (matches the worldgen ladder). */
function gradeColorByName(grade: string): string {
  switch (grade) {
    case "Exquisite": return "rgb(120,190,230)";
    case "Fine": return "rgb(110,180,150)";
    case "Standard": return "rgb(150,170,120)";
    case "Common": return "rgb(170,150,110)";
    default: return "rgb(150,120,110)";
  }
}

type Sort = "produced" | "traded" | "quality" | "name";
const SORTS: [Sort, string][] = [["produced", "Produced"], ["traded", "Traded"], ["quality", "Quality"], ["name", "Name"]];

/** DLC 4 · the floating Goods window — every good ranked by quality, with its best
 *  maker + where, average grade, and how much is produced / in trade. Sort + filter
 *  to answer "what's the world's finest silk, and who moves the most grain?".
 *  Built on the shared UI kit (src/ui/kit.tsx). */
export function GoodsMarketPanel() {
  const open = useUIStore((s) => s.showGoodsWindow);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;
  const [rows, setRows] = useState<GoodMarketRow[]>([]);
  const [sort, setSort] = useState<Sort>("produced");
  const [manuOnly, setManuOnly] = useState(false);
  const [tradedOnly, setTradedOnly] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const toggleRow = (g: string) => setExpanded((p) => { const n = new Set(p); n.has(g) ? n.delete(g) : n.add(g); return n; });

  useEffect(() => {
    if (!open || !active) return;
    campaignGetGoods().then(setRows).catch(() => setRows([]));
  }, [open, active, tick]);

  const view = useMemo(() => {
    let r = rows.filter((x) => (!manuOnly || x.manufactured) && (!tradedOnly || x.traded > 0.01));
    const key: Record<Sort, (x: GoodMarketRow) => number | string> = {
      produced: (x) => -x.produced, traded: (x) => -x.traded,
      quality: (x) => -x.best_quality, name: (x) => label(x.good),
    };
    return [...r].sort((a, b) => { const ka = key[sort](a), kb = key[sort](b); return ka < kb ? -1 : ka > kb ? 1 : 0; });
  }, [rows, sort, manuOnly, tradedOnly]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.goods);
  if (!open) return null;
  const close = () => useUIStore.getState().setShowGoodsWindow(false);
  const maxProd = Math.max(1, ...view.map((x) => x.produced));

  return (
    <Panel width={470} maxHeight="80vh" style={{ top: 60, right: 360, zIndex: 40, ...rootStyle }}>
      <PanelHeader icon="📦" title="Goods of the World — quality & trade" onDragStart={onPointerDown} onClose={close} />
      {!active && <EmptyNote>Begin the campaign (Step 11) — goods grade up as the world trades.</EmptyNote>}
      {active && (
        <>
          <div style={{ display: "flex", alignItems: "center", gap: 6, padding: "6px 10px", flexWrap: "wrap", borderBottom: `1px solid ${T.line}`, flex: "0 0 auto" }}>
            <span style={{ color: T.inkMid, fontSize: FZ.body }}>Sort</span>
            {SORTS.map(([id, lbl]) => <Chip key={id} on={sort === id} onClick={() => setSort(id)}>{lbl}</Chip>)}
            <span style={{ flex: 1 }} />
            <label style={flt}><input type="checkbox" checked={manuOnly} onChange={(e) => setManuOnly(e.target.checked)} /> manufactured</label>
            <label style={flt}><input type="checkbox" checked={tradedOnly} onChange={(e) => setTradedOnly(e.target.checked)} /> traded only</label>
          </div>
          <div style={{ display: "flex", fontSize: FZ.tiny, color: T.inkDim, padding: "3px 10px 2px", flex: "0 0 auto" }}>
            <span style={{ flex: "0 0 150px" }}>Good</span>
            <span style={{ flex: 1 }}>Best grade · where</span>
            <span style={{ width: 40, textAlign: "right" }}>Avg</span>
            <span style={{ width: 64, textAlign: "right" }}>Produced</span>
            <span style={{ width: 56, textAlign: "right" }}>Traded</span>
          </div>
          <PanelBody style={{ padding: 0 }}>
            {view.map((g) => {
              const isOpen = expanded.has(g.good);
              return (
              <div key={g.good}>
                <div style={{ ...row, cursor: "pointer" }} onClick={() => toggleRow(g.good)} title="Click to see the grade-by-grade breakdown">
                  <span style={{ flex: "0 0 150px", color: T.parchment, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    <span style={{ color: T.inkDim, fontSize: FZ.tiny }}>{g.grades.length > 1 ? (isOpen ? "▾ " : "▸ ") : "  "}</span>
                    {icon(g.good)} {label(g.good)}{g.manufactured ? <span title="manufactured" style={{ color: "#7fa0c0" }}> ⚒</span> : null}
                  </span>
                  <span style={{ flex: 1, display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
                    <span style={{ width: 90, height: 8, background: T.raised, borderRadius: 3, overflow: "hidden", flex: "0 0 auto" }}>
                      <span style={{ display: "block", height: "100%", width: `${Math.max(3, g.best_quality * 100)}%`, background: gradeColor(g.best_quality) }} />
                    </span>
                    <span style={{ color: gradeColor(g.best_quality), fontSize: FZ.small, fontWeight: 700 }}>{g.best_grade}</span>
                    {g.best_city ? <span style={{ color: T.inkMid, fontSize: FZ.tiny, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>· {g.best_city}</span> : null}
                  </span>
                  <span style={{ width: 40, textAlign: "right", color: gradeColor(g.avg_quality), fontSize: FZ.small }}>{Math.round(g.avg_quality * 100)}%</span>
                  <span style={{ width: 64, textAlign: "right", color: T.inkMid, fontSize: FZ.small }}>
                    <Bar v={g.produced} max={maxProd} c={T.gold} /> {fmt(g.produced)}
                  </span>
                  <span style={{ width: 56, textAlign: "right", color: T.goodInk, fontSize: FZ.small }}>{fmt(g.traded)}</span>
                </div>
                {isOpen && g.grades.map((b) => (
                  <div key={b.grade} style={{ ...row, background: T.card, paddingTop: 2, paddingBottom: 2, borderBottom: `1px solid ${T.lineSoft}` }}>
                    <span style={{ flex: "0 0 150px", paddingLeft: 22, color: gradeColorByName(b.grade), fontSize: FZ.small }}>
                      {label(g.good)} · <b>{b.grade}</b>
                    </span>
                    <span style={{ flex: 1, color: T.inkDim, fontSize: FZ.tiny }}>{b.n_producers} producer{b.n_producers === 1 ? "" : "s"}</span>
                    <span style={{ width: 40 }} />
                    <span style={{ width: 64, textAlign: "right", color: T.gold, fontSize: FZ.small }}>{fmt(b.produced)}</span>
                    <span style={{ width: 56, textAlign: "right", color: T.goodInk, fontSize: FZ.small }}>{fmt(b.traded)}</span>
                  </div>
                ))}
              </div>
              );
            })}
            {view.length === 0 && <EmptyNote>No goods match the filter.</EmptyNote>}
          </PanelBody>
        </>
      )}
    </Panel>
  );
}

function Bar({ v, max, c }: { v: number; max: number; c: string }) {
  return (
    <span style={{ display: "inline-block", width: 26, height: 6, background: T.raised, borderRadius: 2, overflow: "hidden", marginRight: 4, verticalAlign: "middle" }}>
      <span style={{ display: "block", height: "100%", width: `${Math.max(2, (v / max) * 100)}%`, background: c }} />
    </span>
  );
}

const row: React.CSSProperties = { display: "flex", alignItems: "center", gap: 4, padding: "4px 10px", borderBottom: `1px solid ${T.lineSoft}`, fontSize: FZ.body };
const flt: React.CSSProperties = { display: "flex", alignItems: "center", gap: 3, color: T.inkMid, fontSize: FZ.small };
