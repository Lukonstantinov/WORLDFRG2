import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useViewportStore } from "../state/viewportStore";
import { getRiverSystems, renderWorldCrop } from "../bridge/tauri";
import type { RiverNode, RiverData, Toponym } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** 🌊 Hydrology dashboard — every river system with full hydrological stats.
 *  Main list is an accordion of river SYSTEMS (trunks); expanding one opens the
 *  "snip hero" detail: a location snip (the river + tributaries + cities), the
 *  elevation profile, a stat block, and the full recursive named tributary tree.
 *  Selecting a river or tributary highlights + zooms to it on the map. */
export function HydrologyPanel() {
  const open = useUIStore((s) => s.showHydrology);
  const rivers = useWorldStore((s) => s.rivers);
  const settlements = useWorldStore((s) => s.settlements);
  const toponyms = useWorldStore((s) => s.toponyms);
  const focusOn = useViewportStore((s) => s.focusOn);
  const setRiverHighlight = useUIStore((s) => s.setRiverHighlight);
  const setRiverHighlightColors = useUIStore((s) => s.setRiverHighlightColors);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.hydrology);

  const [systems, setSystems] = useState<RiverNode[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<"length" | "discharge" | "order">("length");
  const [openId, setOpenId] = useState<number | null>(null);
  const [selId, setSelId] = useState<number | null>(null);
  const [colorMode, setColorMode] = useState<ColorMode>("branch");

  // Fetch the system tree whenever the panel opens or the rivers change.
  useEffect(() => {
    if (!open) return;
    if (rivers.length === 0) { setSystems([]); return; }
    let alive = true;
    setLoading(true); setErr(null);
    getRiverSystems(
      rivers,
      settlements.map((s) => ({ x: s.x, y: s.y, name: s.name, size: s.size })),
    )
      .then((sys) => { if (alive) { setSystems(sys); setLoading(false); } })
      .catch((e) => { if (alive) { setErr(String(e)); setLoading(false); } });
    return () => { alive = false; };
  }, [open, rivers, settlements]);

  // Assign a display name to every node: a matching river toponym if one sits on
  // the reach, else a stable generated name. Memoized over the loaded systems.
  const names = useMemo(() => nameMap(systems, toponyms), [systems, toponyms]);

  const openSys = useMemo(() => systems.find((s) => s.id === openId) ?? null, [systems, openId]);
  // Per-river glow colour for the open system, by the chosen scheme.
  const colorMap = useMemo(() => (openSys ? buildColorMap(openSys, colorMode) : {}), [openSys, colorMode]);

  // Glow the selected river system's subtree on the map, coloured by scheme
  // (cleared when nothing is open or the panel is closed).
  useEffect(() => {
    if (!open || !openSys) { setRiverHighlight(null); setRiverHighlightColors(null); return; }
    const node = findNode(openSys, selId) ?? openSys;
    setRiverHighlight(subtreeIds(node));
    setRiverHighlightColors(colorMap);
  }, [open, openSys, selId, colorMap, setRiverHighlight, setRiverHighlightColors]);
  // Clear the highlight when the panel unmounts.
  useEffect(() => () => { setRiverHighlight(null); setRiverHighlightColors(null); }, [setRiverHighlight, setRiverHighlightColors]);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    let list = systems;
    if (q) list = systems.filter((r) => (names.get(r.id) ?? "").toLowerCase().includes(q));
    const key = (r: RiverNode) =>
      sort === "length" ? r.length_km : sort === "discharge" ? r.discharge_m3s : r.order;
    return [...list].sort((a, b) => key(b) - key(a));
  }, [systems, query, sort, names]);

  if (!open) return null;
  const close = () => useUIStore.getState().setShowHydrology(false);
  const byId = new Map<number, RiverData>(rivers.map((r, i) => [i, r]));

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>🌊 Hydrology · Rivers</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>

      <div data-no-drag style={{ display: "flex", gap: 6, alignItems: "center", padding: "7px 10px", borderBottom: "1px solid #16283a" }}>
        <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search rivers…"
          style={{ flex: 1, minWidth: 0, background: "#0a141d", border: "1px solid #1d2f42", borderRadius: 5,
            color: "#cfe2f6", fontSize: 11, padding: "3px 7px" }} />
        <select value={sort} onChange={(e) => setSort(e.target.value as any)}
          style={{ background: "#0a141d", border: "1px solid #1d2f42", borderRadius: 5, color: "#9fb6cc", fontSize: 11, padding: "3px 4px" }}>
          <option value="length">Length</option>
          <option value="discharge">Discharge</option>
          <option value="order">Order</option>
        </select>
        <select value={colorMode} onChange={(e) => setColorMode(e.target.value as ColorMode)}
          title="How tributaries are coloured on the map & in the snip"
          style={{ background: "#0a141d", border: "1px solid #1d2f42", borderRadius: 5, color: "#9fb6cc", fontSize: 11, padding: "3px 4px" }}>
          <option value="branch">🎨 Branch</option>
          <option value="order">🎨 By order</option>
          <option value="single">🎨 Single</option>
        </select>
      </div>

      <div style={{ overflowY: "auto", maxHeight: "68vh", padding: "6px 8px 12px" }}>
        {loading && <div style={empty}>Tracing the drainage network…</div>}
        {err && <div style={{ ...empty, color: "#d08a6a" }}>Couldn’t read rivers: {err}</div>}
        {!loading && !err && rivers.length === 0 && (
          <div style={empty}>Run the Rivers &amp; Lakes step (5) to generate rivers first.</div>
        )}
        {!loading && !err && rivers.length > 0 && shown.length === 0 && (
          <div style={empty}>No rivers match “{query}”.</div>
        )}

        {!loading && shown.map((sys) => {
          const isOpen = openId === sys.id;
          const nm = names.get(sys.id) ?? "River";
          const sel = isOpen ? (findNode(sys, selId) ?? sys) : sys;
          return (
            <div key={sys.id} style={{ marginBottom: 6, border: `1px solid ${isOpen ? "#204058" : "#152535"}`,
              borderRadius: 7, background: isOpen ? "#0c1a24" : "transparent", overflow: "hidden" }}>
              {/* Collapsed accordion row */}
              <div onClick={() => { setOpenId(isOpen ? null : sys.id); setSelId(sys.id); }}
                style={{ display: "flex", alignItems: "center", gap: 8, padding: "7px 9px", cursor: "pointer" }}>
                <span style={{ color: "#5f7a95", fontSize: 10, width: 8 }}>{isOpen ? "▾" : "▸"}</span>
                <span style={ordBadge}>{sys.order}</span>
                <span style={{ fontWeight: 650, color: "#d6e8f6", fontSize: 12.5, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis", flex: "0 1 auto" }}>{nm}</span>
                {sys.navigable && <span style={navTag}>⚓</span>}
                <Sparkline profile={sys.profile} />
                <span style={{ color: "#7d94ab", fontSize: 10.5, fontVariantNumeric: "tabular-nums", whiteSpace: "nowrap" }}>
                  {fmt(sys.length_km)} km
                </span>
                <span style={{ color: "#5f7a95", fontSize: 10, whiteSpace: "nowrap" }}>
                  ⑂{sys.trib_total} · ⛆{sys.city_total}
                </span>
              </div>

              {/* Expanded snip-hero detail */}
              {isOpen && (
                <div style={{ padding: "2px 9px 10px" }}>
                  <Snip node={sel} byId={byId} names={names} colors={colorMap} onLocate={() => focusOn(sel.mouth_x, sel.mouth_y)} />

                  <div style={{ display: "flex", gap: 6, flexWrap: "wrap", margin: "9px 0 8px" }}>
                    {sel.navigable && <span style={chip}>⚓ navigable {fmt(sel.navigable_km)} km</span>}
                    <span style={{ ...chip, ...srcChip }}>⛰ {sourceLabel(sel.source_kind)} · {fmt(sel.source_elev_m)} m</span>
                    <span style={chip}>{mouthLabel(sel.mouth_kind)}</span>
                    <button data-no-drag onClick={() => focusOn(sel.mouth_x, sel.mouth_y)} style={locateBtn}>📍 Locate</button>
                  </div>

                  <div style={statsGrid}>
                    <Stat k="Length" v={fmt(sel.length_km)} u="km" />
                    <Stat k="Elev. drop" v={fmt(sel.drop_m)} u="m" />
                    <Stat k="Avg. gradient" v={sel.avg_slope_m_per_km.toFixed(1)} u="m/km" />
                    <Stat k="Discharge" v={fmt(sel.discharge_m3s)} u="m³/s" />
                    <Stat k="Max width" v={`≈${fmt(sel.max_width_m)}`} u="m" />
                    <Stat k="Max depth" v={`≈${fmt(sel.max_depth_m)}`} u="m" />
                  </div>

                  {/* Real-world counterpart — the comparable river */}
                  <div style={counter}>
                    Comparable real-world river · <b>{sel.counterpart}</b>
                    <span style={{ color: "#5f7a95", marginLeft: 6 }}>(by discharge)</span>
                  </div>

                  <div style={subLabel}>Long profile · source → mouth</div>
                  <Profile node={sel} names={names} />

                  {sel.cities.length > 0 && (
                    <div style={{ marginTop: 8 }}>
                      <div style={subLabel}>Cities on this reach</div>
                      <div style={{ display: "flex", flexWrap: "wrap", gap: 5 }}>
                        {sel.cities.map((c, i) => (
                          <span key={i} onClick={() => focusOn(c.x, c.y)} data-no-drag
                            style={cityChip} title={`${fmt(c.dist_from_mouth_km)} km from the mouth`}>
                            ⛆ {c.name} <span style={{ color: "#8a7a4a" }}>{fmt(c.dist_from_mouth_km)}km</span>
                          </span>
                        ))}
                      </div>
                    </div>
                  )}

                  <div style={{ ...subLabel, marginTop: 10 }}>Tributaries · full network</div>
                  <div style={{ fontSize: 12 }}>
                    <TreeRow node={sys} names={names} colors={colorMap} selId={sel.id}
                      onSelect={(id, mx, my) => { setSelId(id); focusOn(mx, my); }} depth={0} isRoot />
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ── Location snip: real terrain crop with only this river's subtree drawn over it ──
function Snip({ node, byId, names, colors, onLocate }: {
  node: RiverNode; byId: Map<number, RiverData>; names: Map<number, string>;
  colors: Record<number, string>; onLocate: () => void;
}) {
  const geo = useMemo(() => {
    const ids = subtreeIds(node);
    const paths = ids.map((id) => byId.get(id)?.points ?? []).filter((p) => p.length > 1);
    if (paths.length === 0) return null;
    let minx = Infinity, miny = Infinity, maxx = -Infinity, maxy = -Infinity;
    for (const p of paths) for (const [x, y] of p) {
      if (x < minx) minx = x; if (x > maxx) maxx = x;
      if (y < miny) miny = y; if (y > maxy) maxy = y;
    }
    const pad = Math.max(3, Math.max(maxx - minx, maxy - miny) * 0.06);
    minx -= pad; miny -= pad; maxx += pad; maxy += pad;
    const bw = Math.max(1, maxx - minx), bh = Math.max(1, maxy - miny);
    const VW = 200, VH = Math.max(70, Math.min(150, (VW * bh) / bw));
    return { ids, minx, miny, maxx, maxy, bw, bh, VW, VH };
  }, [node, byId]);

  // Fetch a real terrain crop for the bbox and turn it into a data-URL backdrop.
  const [bg, setBg] = useState<string | null>(null);
  useEffect(() => {
    if (!geo) { setBg(null); return; }
    let alive = true;
    setBg(null);
    renderWorldCrop(Math.floor(geo.minx), Math.floor(geo.miny), Math.ceil(geo.maxx), Math.ceil(geo.maxy), "land", 256)
      .then((img) => {
        if (!alive) return;
        const bin = atob(img.data);
        const arr = new Uint8ClampedArray(bin.length);
        for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
        const cv = document.createElement("canvas");
        cv.width = img.w; cv.height = img.h;
        const cx = cv.getContext("2d");
        if (!cx) return;
        cx.putImageData(new ImageData(arr, img.w, img.h), 0, 0);
        setBg(cv.toDataURL());
      })
      .catch(() => { if (alive) setBg(null); });
    return () => { alive = false; };
  }, [geo?.minx, geo?.miny, geo?.maxx, geo?.maxy]);

  if (!geo) return null;
  const { ids, minx, miny, bw, bh, VW, VH } = geo;
  const sx = (x: number) => ((x - minx) / bw) * VW;
  const sy = (y: number) => ((y - miny) / bh) * VH;
  const cities = subtreeCities(node).slice(0, 14);
  const trunkPts = byId.get(node.id)?.points ?? [];

  return (
    <div style={{ position: "relative" }}>
      <svg viewBox={`0 0 ${VW} ${VH}`} onClick={onLocate}
        style={{ width: "100%", height: "auto", display: "block", borderRadius: 7, border: "1px solid #16283a",
          background: "radial-gradient(120% 90% at 22% 12%, #1b2f3d 0%, #10202b 55%, #0b161e 100%)", cursor: "pointer" }}>
        {/* real terrain crop backdrop (falls back to the gradient until loaded) */}
        {bg && <image href={bg} x={0} y={0} width={VW} height={VH} preserveAspectRatio="none" opacity={0.92} />}
        {/* tributaries (thin) — coloured by the chosen scheme */}
        {ids.map((id, i) => {
          if (id === node.id) return null;
          const pts = byId.get(id)?.points ?? [];
          if (pts.length < 2) return null;
          return <polyline key={i} points={pts.map(([x, y]) => `${sx(x)},${sy(y)}`).join(" ")}
            fill="none" stroke={colors[id] ?? "#57a6cf"} strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round" opacity={0.95} />;
        })}
        {/* trunk (bold + halo) */}
        {trunkPts.length > 1 && <>
          <polyline points={trunkPts.map(([x, y]) => `${sx(x)},${sy(y)}`).join(" ")} fill="none" stroke="#4aa3c4" strokeWidth={5} strokeLinecap="round" strokeLinejoin="round" opacity={0.14} />
          <polyline points={trunkPts.map(([x, y]) => `${sx(x)},${sy(y)}`).join(" ")} fill="none" stroke="#2f74b8" strokeWidth={2.4} strokeLinecap="round" strokeLinejoin="round" />
        </>}
        {/* mouth marker */}
        <path d={`M${sx(node.mouth_x) - 3.5} ${sy(node.mouth_y) - 3.5} L${sx(node.mouth_x) + 3.5} ${sy(node.mouth_y) - 1.5} L${sx(node.mouth_x)} ${sy(node.mouth_y) + 3.5} Z`} fill="#cf6a5a" />
        {/* cities */}
        {cities.map((c, i) => (
          <g key={i}>
            <circle cx={sx(c.x)} cy={sy(c.y)} r={2.4} fill="#e0b24a" stroke="#1a1408" strokeWidth={0.7} />
            <text x={sx(c.x) + 3.4} y={sy(c.y) - 2.6} fill="#f0dca4" fontSize={6} fontFamily="ui-sans-serif, system-ui">{c.name}</text>
          </g>
        ))}
        <text x={6} y={12} fill="#dbeaf7" fontSize={8.5} fontWeight={700} fontFamily="ui-sans-serif, system-ui">{names.get(node.id) ?? "River"}</text>
      </svg>
    </div>
  );
}

// ── Long profile: area chart with city pins + confluence ticks ──
function Profile({ node, names }: { node: RiverNode; names: Map<number, string> }) {
  const p = node.profile;
  if (!p || p.length < 2) return <div style={{ ...empty, padding: "4px 0" }}>No profile.</div>;
  const VW = 200, VH = 74, L = 4, R = 6, T = 6, B = 14;
  const maxE = Math.max(1, ...p);
  const x = (t: number) => L + t * (VW - L - R);
  const y = (e: number) => VH - B - (e / maxE) * (VH - B - T);
  const pts = p.map((e, i) => [x(i / (p.length - 1)), y(e)] as const);
  const area = pts.map(([a, b]) => `${a},${b}`).join(" ") + ` ${x(1)},${VH - B} ${x(0)},${VH - B}`;
  const len = Math.max(1, node.length_km);

  return (
    <svg viewBox={`0 0 ${VW} ${VH}`} style={{ width: "100%", height: "auto", display: "block" }}>
      {[0, 1, 2].map((i) => {
        const e = (maxE * i) / 2, yy = y(e);
        return <g key={i}>
          <line x1={L} y1={yy} x2={VW - R} y2={yy} stroke="#16283a" strokeWidth={0.5} />
          <text x={L} y={yy - 1.4} fill="#4d647a" fontSize={5} fontFamily="ui-monospace, monospace">{Math.round(e)}m</text>
        </g>;
      })}
      <polygon points={area} fill="#4aa3c433" />
      <polyline points={pts.map(([a, b]) => `${a},${b}`).join(" ")} fill="none" stroke="#4aa3c4" strokeWidth={1.3} />
      {/* confluence ticks (children join at join_km along this reach) */}
      {node.children.map((c, i) => {
        const t = Math.max(0, Math.min(1, c.join_km / len));
        return <g key={"c" + i}>
          <path d={`M${x(t) - 2} ${VH - B + 2} L${x(t) + 2} ${VH - B + 2} L${x(t)} ${VH - B - 1.5} Z`} fill="#86b6d6" />
        </g>;
      })}
      {/* city pins (source-side fraction from dist-to-mouth) */}
      {node.cities.slice(0, 6).map((c, i) => {
        const t = Math.max(0, Math.min(1, 1 - c.dist_from_mouth_km / len));
        const e = p[Math.round(t * (p.length - 1))] ?? 0;
        return <g key={"p" + i}>
          <line x1={x(t)} y1={y(e)} x2={x(t)} y2={T} stroke="#5a4a22" strokeWidth={0.5} strokeDasharray="1.5 1.5" />
          <circle cx={x(t)} cy={y(e)} r={1.9} fill="#e0b24a" stroke="#1a1408" strokeWidth={0.6} />
          <text x={x(t) + 3} y={T + 4} fill="#e6cf95" fontSize={5.2} fontFamily="ui-sans-serif, system-ui">{c.name}</text>
        </g>;
      })}
      <text x={L} y={VH - 3} fill="#7d94ab" fontSize={5}>source</text>
      <text x={VW - R} y={VH - 3} fill="#7d94ab" fontSize={5} textAnchor="end">▽ mouth</text>
    </svg>
  );
}

// ── Recursive named tributary tree ──
function TreeRow({ node, names, colors, selId, onSelect, depth, isRoot }: {
  node: RiverNode; names: Map<number, string>; colors: Record<number, string>; selId: number;
  onSelect: (id: number, mx: number, my: number) => void; depth: number; isRoot?: boolean;
}) {
  const nm = names.get(node.id) ?? "River";
  const on = node.id === selId;
  const swatch = colors[node.id];
  return (
    <div>
      <div data-no-drag onClick={() => onSelect(node.id, node.mid_x, node.mid_y)}
        style={{ display: "flex", alignItems: "center", gap: 6, padding: "2px 4px", marginLeft: depth * 12,
          cursor: "pointer", borderRadius: 4, background: on ? "#14324a" : "transparent" }}>
        {/* colour swatch keyed to the map glow (empty in Single mode) */}
        <span style={{ width: 8, height: 8, borderRadius: 2, flex: "0 0 auto",
          background: swatch ?? "transparent", border: swatch ? "none" : "1px solid #2a4256" }} />
        <span style={{ color: "#5f7a95" }}>{isRoot ? "" : "⑂"}</span>
        <span style={ordBadgeSm}>{node.order}</span>
        <span style={{ fontWeight: on ? 700 : 500, color: on ? "#cfe8fa" : isRoot ? "#bfe0f2" : "#a8c2d8" }}>{nm}</span>
        {node.navigable && <span style={{ color: "#89cbe0", fontSize: 10 }}>⚓</span>}
        <span style={{ color: "#5f7a95", fontSize: 10.5, fontVariantNumeric: "tabular-nums" }}>
          {fmt(node.length_km)} km{!isRoot && node.join_km > 0 ? ` · joins @ ${fmt(node.join_km)} km` : ""}
        </span>
      </div>
      {node.children.map((c) => (
        <TreeRow key={c.id} node={c} names={names} colors={colors} selId={selId} onSelect={onSelect} depth={depth + 1} />
      ))}
    </div>
  );
}

function Sparkline({ profile }: { profile: number[] }) {
  if (!profile || profile.length < 2) return <span style={{ flex: 1 }} />;
  const maxE = Math.max(1, ...profile);
  const W = 60, H = 14;
  const pts = profile.map((e, i) => `${(i / (profile.length - 1)) * W},${H - (e / maxE) * (H - 2) - 1}`).join(" ");
  return (
    <svg viewBox={`0 0 ${W} ${H}`} style={{ flex: 1, minWidth: 36, maxWidth: 74, height: 14 }} preserveAspectRatio="none">
      <polyline points={pts} fill="none" stroke="#4aa3c4" strokeWidth={1} opacity={0.85} />
    </svg>
  );
}

function Stat({ k, v, u }: { k: string; v: string; u?: string }) {
  return (
    <div style={{ background: "#10202e", border: "1px solid #16283a", borderRadius: 6, padding: "5px 7px" }}>
      <div style={{ fontSize: 9, letterSpacing: "0.04em", textTransform: "uppercase", color: "#56708a" }}>{k}</div>
      <div style={{ fontSize: 13.5, fontWeight: 650, fontVariantNumeric: "tabular-nums" }}>
        {v}{u && <span style={{ fontSize: 9.5, color: "#7d94ab", fontWeight: 500 }}> {u}</span>}
      </div>
    </div>
  );
}

// ── helpers ──
function fmt(n: number): string {
  return Math.round(n).toLocaleString("en-US");
}
function sourceLabel(k: string): string {
  return ({ alpine: "alpine spring", highland: "highland", hills: "hill country", lowland: "lowland", bog: "lowland bog" } as Record<string, string>)[k] ?? k;
}
function mouthLabel(k: number): string {
  return k === 1 ? "▽ delta mouth" : k === 2 ? "▽ estuary" : "▽ river mouth";
}
function subtreeIds(node: RiverNode): number[] {
  const out = [node.id];
  for (const c of node.children) out.push(...subtreeIds(c));
  return out;
}

// ── Tributary colouring ──
type ColorMode = "branch" | "order" | "single";
// Distinct hues that read on both the terrain snip and the dark map glow.
const BRANCH_PALETTE = ["#ffd24a", "#ff8f5a", "#e56ad0", "#9b8cff", "#5ad1e0", "#77d86a", "#ff6f91", "#b6e152", "#6aa8ff", "#f0a35e", "#4fe0b0", "#d98cff"];
const TRUNK_COLOR = "#cdeeff";
// Pale headwater → deep trunk, indexed by Strahler order 1..7.
const ORDER_PALETTE = ["#e6f3fb", "#a7dcf0", "#72c6e6", "#57a8d8", "#4f86c8", "#5566c8", "#6a4fb8"];

/** Build a river-index → glow colour map for one system by the chosen scheme.
 *  branch: the trunk is bright, each direct tributary (and its whole sub-network)
 *  gets its own hue. order: colour by Strahler order. single: {} (default cyan). */
function buildColorMap(root: RiverNode, mode: ColorMode): Record<number, string> {
  const m: Record<number, string> = {};
  if (mode === "single") return m;
  if (mode === "order") {
    const walk = (n: RiverNode) => {
      m[n.id] = ORDER_PALETTE[Math.min(Math.max(n.order, 1), ORDER_PALETTE.length) - 1];
      n.children.forEach(walk);
    };
    walk(root);
    return m;
  }
  // branch
  m[root.id] = TRUNK_COLOR;
  root.children.forEach((child, k) => {
    const col = BRANCH_PALETTE[k % BRANCH_PALETTE.length];
    const paint = (n: RiverNode) => { m[n.id] = col; n.children.forEach(paint); };
    paint(child);
  });
  return m;
}
function subtreeCities(node: RiverNode): RiverNode["cities"] {
  const out = [...node.cities];
  for (const c of node.children) out.push(...subtreeCities(c));
  return out;
}
function findNode(node: RiverNode, id: number | null): RiverNode | null {
  if (id == null) return null;
  if (node.id === id) return node;
  for (const c of node.children) { const f = findNode(c, id); if (f) return f; }
  return null;
}

/** Map every node id → display name: a nearby river toponym (so a named river
 *  matches its map label), else the backend's unique culture-styled `node.name`. */
function nameMap(systems: RiverNode[], toponyms: Toponym[]): Map<number, string> {
  const m = new Map<number, string>();
  const riverTops = toponyms.filter((t) => t.kind === "river");
  const used = new Set<string>();
  const walk = (n: RiverNode) => {
    let best: string | null = null, bestD = 36; // within 6 cells of the midpoint
    for (const t of riverTops) {
      const dx = t.x - n.mid_x, dy = t.y - n.mid_y, d = dx * dx + dy * dy;
      if (d < bestD && !used.has(t.name)) { bestD = d; best = t.name; }
    }
    const name = best ?? n.name;
    used.add(name);
    m.set(n.id, name);
    n.children.forEach(walk);
  };
  systems.forEach(walk);
  return m;
}

// ── styles ──
const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 356, maxHeight: "84vh",
  border: "1px solid #1d2f42", borderRadius: 9, zIndex: 42,
  boxShadow: "0 18px 44px -20px rgba(0,0,0,.85)", color: "#d8e8f6",
  fontFamily: "ui-sans-serif, system-ui, sans-serif", display: "flex", flexDirection: "column",
};
const header: React.CSSProperties = {
  display: "flex", alignItems: "center", justifyContent: "space-between",
  padding: "8px 11px", borderBottom: "1px solid #16283a", fontSize: 12.5, fontWeight: 600,
};
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "12px 4px" };
const ordBadge: React.CSSProperties = {
  display: "grid", placeItems: "center", width: 18, height: 18, borderRadius: 5,
  background: "#12314a", color: "#bfe0f2", fontSize: 11, fontWeight: 700,
  fontVariantNumeric: "tabular-nums", border: "1px solid #244a68", flex: "0 0 auto",
};
const ordBadgeSm: React.CSSProperties = { ...ordBadge, width: 15, height: 15, fontSize: 9 };
const navTag: React.CSSProperties = { color: "#89cbe0", fontSize: 11 };
const statsGrid: React.CSSProperties = { display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 6 };
const chip: React.CSSProperties = {
  fontSize: 10.5, padding: "2px 7px", borderRadius: 4, border: "1px solid #1d2f42", color: "#9fb6cc",
  background: "#0b161e", whiteSpace: "nowrap",
};
const srcChip: React.CSSProperties = { color: "#cbb682", borderColor: "#45402a", background: "#1c1a12" };
const cityChip: React.CSSProperties = { ...chip, cursor: "pointer", color: "#e6cf95" };
const locateBtn: React.CSSProperties = {
  fontSize: 10.5, padding: "2px 8px", borderRadius: 4, border: "1px solid #2a5a72",
  background: "#102735", color: "#a9d8ea", cursor: "pointer", marginLeft: "auto",
};
const counter: React.CSSProperties = {
  fontSize: 11.5, color: "#8aa0b8", marginTop: 9, paddingTop: 8, borderTop: "1px dashed #1d2f42",
};
const subLabel: React.CSSProperties = {
  fontSize: 10, letterSpacing: "0.06em", textTransform: "uppercase", color: "#56708a", margin: "10px 0 5px",
};
