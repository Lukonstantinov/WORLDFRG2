import { useMemo } from "react";
import type { CoinUseCity } from "@types";

/** A1 · a self-contained SVG "spread map" for ONE coin — its geographic footprint
 *  at a glance, without toggling the big map overlay. Reuses the three-tier visual
 *  language of the on-map coin-usage overlay (OverlayManager.renderCoinUsage):
 *    • PRIMARY turf  — filled disc sized by volume (its own MINT is a bright star);
 *      the primary cities are wrapped in a translucent "territory" hull.
 *    • RESERVE reach — held abroad as a reserve coin: a green ring.
 *    • HELD-only     — a minor basket holding: a faint dot.
 *  `backdrop` (all coin-holding cities) is drawn as faint specks so the coin's reach
 *  reads against the whole coined world. Pure presentation over existing data. */
export function CoinMiniMap({
  usage, worldW, worldH, width = 172, backdrop = [],
}: {
  usage: CoinUseCity[];
  worldW: number;
  worldH: number;
  width?: number;
  backdrop?: { x: number; y: number }[];
}) {
  const H = worldW > 0 ? Math.max(72, Math.min(150, width * (worldH / worldW))) : 96;
  const sx = (x: number) => (worldW > 0 ? (x / worldW) * width : 0);
  const sy = (y: number) => (worldH > 0 ? (y / worldH) * H : 0);

  const primary = useMemo(() => usage.filter((u) => u.primary), [usage]);
  const maxVol = useMemo(() => Math.max(1e-6, ...usage.map((u) => u.volume)), [usage]);
  const coinColor = usage[0]?.color || "#c9a227";

  // Territory hull around the coin's core (primary cities), as normalized points.
  const hull = useMemo(() => {
    if (primary.length < 3) return "";
    const pts = primary.map((u) => ({ x: sx(u.x + 0.5), y: sy(u.y + 0.5) }));
    const h = convexHull(pts);
    if (h.length < 3) return "";
    return "M" + h.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join("L") + "Z";
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [primary, width, H, worldW, worldH]);

  if (worldW <= 0 || worldH <= 0) return null;

  return (
    <svg viewBox={`0 0 ${width} ${H}`} width="100%" style={{ display: "block", borderRadius: 5, background: "#0a121c", border: "1px solid #16273a" }}>
      {/* faint graticule so the frame reads as a map */}
      <line x1={width / 2} y1={0} x2={width / 2} y2={H} stroke="#12202e" strokeWidth={0.6} />
      <line x1={0} y1={H / 2} x2={width} y2={H / 2} stroke="#12202e" strokeWidth={0.6} />

      {/* backdrop: every coin-holding city, faint — the coined world for context */}
      {backdrop.map((c, i) => (
        <circle key={`b${i}`} cx={sx(c.x + 0.5)} cy={sy(c.y + 0.5)} r={0.8} fill="#2a3c52" />
      ))}

      {/* the coin's core territory */}
      {hull && <path d={hull} fill={coinColor} fillOpacity={0.12} stroke={coinColor} strokeOpacity={0.4} strokeWidth={0.7} strokeDasharray="3 2" />}

      {/* tiers */}
      {usage.map((u, i) => {
        const t = u.volume / maxVol;
        const cx = sx(u.x + 0.5), cy = sy(u.y + 0.5);
        if (u.primary) {
          const r = u.mint ? 3.4 : 1.6 + 2.0 * t;
          return (
            <g key={i}>
              <circle cx={cx} cy={cy} r={r} fill={u.mint ? "#f6df85" : coinColor} fillOpacity={u.mint ? 1 : 0.55 + 0.4 * t}
                stroke={u.mint ? "#fff3c0" : "#0c1826"} strokeWidth={u.mint ? 0.9 : 0.5} />
              {u.mint && <circle cx={cx} cy={cy} r={r + 1.6} fill="none" stroke="#f6df85" strokeOpacity={0.5} strokeWidth={0.6} />}
            </g>
          );
        }
        if (u.reserve_reach) {
          return <circle key={i} cx={cx} cy={cy} r={1.5 + 1.4 * t} fill="none" stroke="#37a05a" strokeWidth={1} strokeOpacity={0.9} strokeDasharray="2 1.5" />;
        }
        return <circle key={i} cx={cx} cy={cy} r={1.3} fill={coinColor} fillOpacity={0.32 + 0.25 * t} />;
      })}
    </svg>
  );
}

/** Monotone-chain convex hull of normalized points (mirrors OverlayManager). */
function convexHull(pts: { x: number; y: number }[]): { x: number; y: number }[] {
  if (pts.length < 3) return pts.slice();
  const p = pts.slice().sort((a, b) => a.x - b.x || a.y - b.y);
  const cross = (o: { x: number; y: number }, a: { x: number; y: number }, b: { x: number; y: number }) =>
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
  const lower: { x: number; y: number }[] = [];
  for (const q of p) { while (lower.length >= 2 && cross(lower[lower.length - 2], lower[lower.length - 1], q) <= 0) lower.pop(); lower.push(q); }
  const upper: { x: number; y: number }[] = [];
  for (let i = p.length - 1; i >= 0; i--) { const q = p[i]; while (upper.length >= 2 && cross(upper[upper.length - 2], upper[upper.length - 1], q) <= 0) upper.pop(); upper.push(q); }
  lower.pop(); upper.pop();
  return lower.concat(upper);
}

/** A tiny legend strip explaining the minimap tiers (shared by callers). */
export function CoinMiniMapLegend() {
  return (
    <div style={{ display: "flex", gap: 10, flexWrap: "wrap", fontSize: 8, color: "#7e93ab", marginTop: 3 }}>
      <span><span style={{ color: "#f6df85" }}>★</span> mint</span>
      <span><span style={{ color: "#c9a227" }}>●</span> primary turf</span>
      <span><span style={{ color: "#37a05a" }}>◌</span> held abroad (reserve)</span>
      <span><span style={{ color: "#4a5e76" }}>·</span> minor holding</span>
    </div>
  );
}
