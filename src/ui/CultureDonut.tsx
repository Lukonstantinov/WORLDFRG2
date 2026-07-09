import { useEffect, useMemo, useState } from "react";
import { campaignGetCultures } from "../bridge/tauri";

/** A per-city culture pie/donut: the composition of a settlement's population as its
 *  majority people plus every minority quarter (grown by in-migration, faded by
 *  assimilation). Colours come from the world's culture palette (`campaignGetCultures`);
 *  a creole/unknown culture gets a deterministic fallback hue so it still reads. */

// Module-level cache of culture-name → "rgb(...)" so the donut + legend never refetch.
let _cache: Map<string, string> | null = null;
let _inflight: Promise<Map<string, string>> | null = null;
function loadCultureColors(): Promise<Map<string, string>> {
  if (_cache) return Promise.resolve(_cache);
  if (!_inflight) {
    _inflight = campaignGetCultures()
      .then((cs) => {
        const m = new Map<string, string>();
        for (const c of cs) m.set(c.name, `rgb(${c.color[0]},${c.color[1]},${c.color[2]})`);
        _cache = m;
        return m;
      })
      .catch(() => new Map<string, string>());
  }
  return _inflight;
}

/** Deterministic fallback colour for a culture not in the palette (e.g. a creole). */
function hashColor(s: string): string {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); }
  return `hsl(${Math.abs(h) % 360} 42% 58%)`;
}

export function CultureDonut({ majority, minorities, size = 92 }: {
  majority: string;
  minorities: [string, number][];
  size?: number;
}) {
  const [colors, setColors] = useState<Map<string, string>>(_cache ?? new Map());
  useEffect(() => {
    let ok = true;
    loadCultureColors().then((m) => { if (ok) setColors(new Map(m)); });
    return () => { ok = false; };
  }, []);

  const slices = useMemo(() => {
    const minSum = minorities.reduce((a, [, s]) => a + Math.max(0, s), 0);
    const majShare = Math.max(0, 1 - Math.min(minSum, 0.98));
    const raw = [{ name: majority, share: majShare, major: true },
      ...minorities.map(([n, s]) => ({ name: n, share: Math.max(0, s), major: false }))]
      .filter((s) => s.share > 0.003);
    const total = raw.reduce((a, s) => a + s.share, 0) || 1;
    return raw.map((s) => ({ ...s, frac: s.share / total }));
  }, [majority, minorities]);

  const colorOf = (n: string) => colors.get(n) ?? hashColor(n);
  const r = size / 2, ir = r * 0.58, cx = r, cy = r;
  let ang = -Math.PI / 2;
  const arcs = slices.map((s) => {
    const a0 = ang, a1 = ang + s.frac * Math.PI * 2; ang = a1;
    const large = a1 - a0 > Math.PI ? 1 : 0;
    const pt = (rr: number, a: number) => [cx + rr * Math.cos(a), cy + rr * Math.sin(a)];
    const [x0, y0] = pt(r, a0), [x1, y1] = pt(r, a1), [x2, y2] = pt(ir, a1), [x3, y3] = pt(ir, a0);
    const d = `M${x0} ${y0} A${r} ${r} 0 ${large} 1 ${x1} ${y1} L${x2} ${y2} A${ir} ${ir} 0 ${large} 0 ${x3} ${y3} Z`;
    return { d, color: colorOf(s.name), name: s.name, pct: Math.round(s.frac * 100), major: s.major };
  });

  return (
    <div style={{ display: "flex", gap: 10, alignItems: "center", margin: "2px 0 6px" }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} style={{ flex: "0 0 auto" }}>
        {arcs.length <= 1 ? (
          <>
            <circle cx={cx} cy={cy} r={r} fill={arcs[0]?.color ?? "#39506a"} />
            <circle cx={cx} cy={cy} r={ir} fill="#0a1018" />
          </>
        ) : arcs.map((a, i) => <path key={i} d={a.d} fill={a.color} stroke="#0a1018" strokeWidth={1} />)}
      </svg>
      <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 2 }}>
        {arcs.map((a, i) => (
          <div key={i} style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 10 }}>
            <span style={{ width: 9, height: 9, borderRadius: 2, background: a.color, flex: "0 0 auto" }} />
            <span style={{ flex: 1, color: "#cfe2f6", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {a.name}{a.major ? " · majority" : ""}
            </span>
            <span style={{ color: "#9ab0c8" }}>{a.pct}%</span>
          </div>
        ))}
      </div>
    </div>
  );
}
