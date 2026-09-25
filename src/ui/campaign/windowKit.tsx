import { useEffect, useRef, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import type { CultureBrief, HubDetail } from "@types";
import { campaignGetCultures } from "@bridge";
import { SERIF } from "@ui/campaign/chronicleTheme";
import { genCity, renderIso, presentIso, type CityCfg, type IsoRender, type SpriteLookup } from "@canvas/cityArt";
import {
  SCENE_CARAVAN, pickFamily, waterKind, popBucket, isWalled, deriveSlots, deriveDistricts, sceneShips,
} from "@ui/campaign/settlementWindowData";

// ── Shared shell for the 1180px "design handoff" windows ──────────────────────
// The settlement window (CityView), the colonial city view, the coin & bank
// window and the house plate all use the same header / band / 3-column card
// grid, the same Chronicle-dark tokens and the same small drawing primitives.
// Chronicle dark only — see CityView.tsx's header for why no light variant.

/** Chronicle-dark tokens, from the handoff's token table. */
export const K = {
  bg: "#0d1521", head: "#111b2a", card: "#0f1826", bd: "#1e2e42", tx: "#cfe2f6", mu: "#9fb4cc", fa: "#6f88a6",
  ac: "#d8b24a", acBg: "rgba(216,178,74,.14)", acBd: "rgba(216,178,74,.38)", pos: "#7fd0a0", neg: "#e8a07c",
  bar: "#1a2536", sea: "#5aa8d8", river: "#6fc3b0", land: "#d8a656", chip: "rgba(9,14,20,.84)", lock: "#4a5c72",
  scene: "#0a1018", hf: SERIF, bf: "system-ui,-apple-system,'Segoe UI',sans-serif",
} as const;
export const SCENE_W = 1180, SCENE_H = 420;
/** The floating-window width that hosts a 1180px window plus its own padding. */
export const WIDE_WINDOW = "min(1206px, calc(100vw - 24px))";

export const fk = (n: number): string => n < 0 ? "−" + fk(-n) : n >= 1e5 ? Math.round(n / 1e3) + "k" : n >= 1e3 ? (n / 1e3).toFixed(1).replace(/\.0$/, "") + "k" : String(Math.round(n));
export const fc = (n: number) => Math.round(n).toLocaleString("en-US");
/** Health colour for a 0..1 score (higher = better). */
export const gcol = (v: number) => v > .6 ? K.pos : v > .38 ? K.ac : K.neg;

// ── culture kits (one fetch, shared by every window that draws a city) ────────
let kitCache: Map<string, number> | null = null;
let kitInflight: Promise<Map<string, number>> | null = null;
function loadKits(): Promise<Map<string, number>> {
  if (kitCache) return Promise.resolve(kitCache);
  if (!kitInflight) {
    kitInflight = campaignGetCultures().then((cs: CultureBrief[]) => {
      const m = new Map<string, number>();
      for (const c of cs) if (typeof c.kit === "number" && c.kit >= 0) m.set(c.name, c.kit);
      kitCache = m; return m;
    }).catch(() => new Map<string, number>());
  }
  return kitInflight;
}
export function useCultureKits(): Map<string, number> | null {
  const [kits, setKits] = useState<Map<string, number> | null>(kitCache);
  useEffect(() => { let alive = true; loadKits().then((m) => { if (alive) setKits(m); }); return () => { alive = false; }; }, []);
  return kits;
}

/** The iso render is the expensive part (hundreds of shaded polygons), so scenes
 *  are cached on exactly what they depend on. Small LRU — a settlement window
 *  holds one entry, a colony's growth strip four. */
const sceneCache = new Map<string, IsoRender>();
export function cachedScene(key: string, make: () => IsoRender): IsoRender {
  const hit = sceneCache.get(key);
  if (hit) { sceneCache.delete(key); sceneCache.set(key, hit); return hit; }
  const r = make();
  sceneCache.set(key, r);
  while (sceneCache.size > 16) { const k = sceneCache.keys().next().value; if (k === undefined) break; sceneCache.delete(k); }
  return r;
}

/** Everything the scene generator needs for one live city — the architectural
 *  family, water, walls and the slot/district rows the scene draws. */
export function cityScene(detail: HubDetail, o: { kit?: number; seaAccess?: boolean; elevation?: number; river: boolean }) {
  const style = pickFamily(detail.koppen, o.kit, detail.coastal, o.elevation);
  const water = waterKind({ coastal: detail.coastal, seaAccess: o.seaAccess, river: o.river, family: style });
  const tier = detail.dev_tier ?? 0;
  const walled = isWalled(detail.population, tier);
  const bucket = popBucket(detail.population);
  const slots = deriveSlots(detail, o.seaAccess);
  const districts = deriveDistricts(detail, slots);
  const caravanN = detail.vessels?.classes.find((c) => c.kind === "caravan")?.registered ?? 0;
  const cfg: CityCfg = {
    name: detail.name || "city", style, water, walled, popBucket: bucket,
    buildings: slots.map((s, i) => ({ kind: s.kind, status: s.status, color: s.color, ref: i })),
    districts: districts.map((d) => ({ name: d.name, color: d.color })),
    caravan: caravanN > 0 || (detail.in_by_land ?? 0) > 0 ? SCENE_CARAVAN[style] : null,
    ships: sceneShips(detail.vessels, water, detail.in_by_sea ?? 0),
  };
  return { style, water, walled, bucket, tier, slots, districts, cfg };
}
export const cfgKey = (cfg: CityCfg) => [cfg.name, cfg.style, cfg.water, cfg.walled ? 1 : 0, cfg.popBucket, cfg.ships, cfg.caravan ?? "",
  cfg.buildings.map((b) => `${b.kind}:${b.status}:${b.color}`).join(","), cfg.districts.map((d) => d.color).join(",")].join("|");

/** A small iso render of a city (growth strip, seat band), drawn 1:1 at W×H and
 *  scaled by CSS. `N`/`R` are the generator's grid and city half-size. */
export function IsoThumb({ cfg, W, H, N = 28, R = 6, TW = 12, style, sprites }:
  { cfg: CityCfg; W: number; H: number; N?: number; R?: number; TW?: number; style?: CSSProperties; sprites?: SpriteLookup }) {
  const ref = useRef<HTMLCanvasElement | null>(null);
  const key = `thumb|${W}x${H}|${N}|${R}|${TW}|${cfgKey(cfg)}`;
  useEffect(() => {
    const cv = ref.current; if (!cv) return;
    const iso = cachedScene(key, () => renderIso(genCity(cfg, N, R), cfg, { W, H, scale: 1, TW, wash: .1, sprites }));
    presentIso(cv, iso.canvas, W, H);
  }, [key]); // eslint-disable-line react-hooks/exhaustive-deps
  return <canvas ref={ref} style={{ display: "block", width: "100%", height: "100%", imageRendering: "pixelated", ...style }} />;
}

// ── small presentational pieces ──────────────────────────────────────────────

export function Card({ title, meta, span, children }: { title: string; meta?: ReactNode; span?: number; children: ReactNode }) {
  return (
    <div style={{ background: K.card, border: `1px solid ${K.bd}`, borderRadius: 6, padding: "12px 14px 14px", minWidth: 0,
      display: "flex", flexDirection: "column", gap: 10, ...(span ? { gridColumn: `span ${span}` } : {}) }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
        <span style={{ font: `600 10px/1 ${K.bf}`, letterSpacing: .7, textTransform: "uppercase", color: K.fa }}>{title}</span>
        <span style={{ flex: 1 }} />
        {meta && <span style={{ font: `400 11px/1.2 ${K.bf}`, color: K.fa, textAlign: "right" }}>{meta}</span>}
      </div>
      {children}
    </div>
  );
}
export function CardGrid({ children }: { children: ReactNode }) {
  return <div style={{ display: "grid", gridTemplateColumns: "repeat(3, minmax(0, 1fr))", gap: 12, padding: "14px 16px 18px" }}>{children}</div>;
}
export function Bar({ frac, color, h = 6 }: { frac: number; color: string; h?: number }) {
  return (
    <div style={{ flex: 1, height: h, background: K.bar, borderRadius: 4, overflow: "hidden", minWidth: 0 }}>
      <div style={{ width: `${Math.max(0, Math.min(100, frac * 100))}%`, height: "100%", background: color, borderRadius: 4 }} />
    </div>
  );
}
export function Stack({ parts, h = 8 }: { parts: [number, string][]; h?: number }) {
  return (
    <div style={{ display: "flex", height: h, borderRadius: 4, overflow: "hidden", gap: 2, background: K.bar }}>
      {parts.filter(([f]) => f > 0).map(([f, c], i) => <div key={i} style={{ flex: `${f} 0 0`, background: c }} />)}
    </div>
  );
}
export function Spark({ vals, color, w = 180, h = 30 }: { vals: number[]; color: string; w?: number; h?: number }) {
  if (vals.length < 2) return null;
  const mn = Math.min(...vals), mx = Math.max(...vals);
  const pts = vals.map((v, k) => `${(k / (vals.length - 1) * w).toFixed(1)},${(h - 2 - (v - mn) / (mx - mn || 1) * (h - 4)).toFixed(1)}`).join(" ");
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} style={{ display: "block", overflow: "visible", maxWidth: "100%" }}>
      <polyline points={`0,${h} ${pts} ${w},${h}`} style={{ fill: color, opacity: .14, stroke: "none" }} />
      <polyline points={pts} style={{ fill: "none", stroke: color, strokeWidth: 1.6, strokeLinejoin: "round" }} />
    </svg>
  );
}
/** A line chart with event markers — `marks` are [index, bad] pairs (red / green). */
export function MarkChart({ vals, color, w, h, marks = [] }: { vals: number[]; color: string; w: number; h: number; marks?: [number, boolean][] }) {
  if (vals.length < 2) return <div style={{ height: h, font: `400 10.5px ${K.bf}`, color: K.fa, display: "flex", alignItems: "center" }}>Charts need two recorded years.</div>;
  const mn = Math.min(...vals), mx = Math.max(...vals);
  const X = (k: number) => k / (vals.length - 1) * w, Y = (v: number) => h - 3 - (v - mn) / (mx - mn || 1) * (h - 10);
  const pts = vals.map((v, k) => `${X(k).toFixed(1)},${Y(v).toFixed(1)}`).join(" ");
  return (
    <svg width="100%" viewBox={`0 0 ${w} ${h}`} style={{ display: "block", overflow: "visible" }}>
      <polyline points={`0,${h} ${pts} ${w},${h}`} style={{ fill: color, opacity: .14, stroke: "none" }} />
      <polyline points={pts} style={{ fill: "none", stroke: color, strokeWidth: 1.8, strokeLinejoin: "round" }} />
      {marks.map(([k, bad], i) => (
        <g key={i}>
          <line x1={X(k)} x2={X(k)} y1={0} y2={h} style={{ stroke: bad ? K.neg : K.pos, strokeWidth: 1, strokeDasharray: "2 2", opacity: .8 }} />
          <circle cx={X(k)} cy={Y(vals[k])} r={3} style={{ fill: bad ? K.neg : K.pos }} />
        </g>
      ))}
    </svg>
  );
}
export function Pill({ children, strong }: { children: ReactNode; strong?: boolean }) {
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 4, background: strong ? K.acBg : "transparent",
      color: strong ? K.ac : K.mu, border: `1px solid ${strong ? K.acBd : K.bd}`, borderRadius: 4, padding: "2px 9px",
      font: `600 10px/1.3 ${K.bf}`, letterSpacing: .4, whiteSpace: "nowrap" }}>{children}</span>
  );
}
export function Tag({ children, color = K.mu, dashed }: { children: ReactNode; color?: string; dashed?: boolean }) {
  return <span style={{ font: `600 9px/1.5 ${K.bf}`, color, border: `1px ${dashed ? "dashed" : "solid"} ${K.bd}`, borderRadius: 4, padding: "0 6px", whiteSpace: "nowrap" }}>{children}</span>;
}
export function Swatch({ color, size = 8, round }: { color: string; size?: number; round?: boolean }) {
  return <span style={{ width: size, height: size, borderRadius: round ? "50%" : 2, background: color, flex: "none", display: "inline-block" }} />;
}
/** A sub-heading inside a card, with an optional right-aligned note. */
export function Sub({ children, meta }: { children: ReactNode; meta?: ReactNode }) {
  return (
    <div style={{ display: "flex", gap: 6, alignItems: "baseline", marginTop: 2, font: `600 9.5px/1 ${K.bf}`, letterSpacing: .6, textTransform: "uppercase", color: K.fa }}>
      {children}{meta && <span style={{ font: `400 10px ${K.bf}`, letterSpacing: 0, textTransform: "none", marginLeft: "auto" }}>{meta}</span>}
    </div>
  );
}
/** A dotted-leader key/value row. */
export function KV({ k, children }: { k: string; children: ReactNode }) {
  return (
    <div style={{ display: "flex", alignItems: "baseline", gap: 8, font: `400 11.5px ${K.bf}`, color: K.mu }}>
      <span>{k}</span><span style={{ flex: 1, borderBottom: `1px dotted ${K.bd}`, transform: "translateY(-3px)" }} />
      <span style={{ color: K.tx, fontWeight: 600, whiteSpace: "nowrap" }}>{children}</span>
    </div>
  );
}
export function Big({ value, sub, color = K.tx }: { value: ReactNode; sub?: ReactNode; color?: string }) {
  return (
    <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
      <span style={{ font: `700 28px/1 ${K.hf}`, color }}>{value}</span>
      {sub && <span style={{ font: `400 11px ${K.bf}`, color: K.fa }}>{sub}</span>}
    </div>
  );
}
export function Pips({ n, of, w = 10 }: { n: number; of: number; w?: number }) {
  return (
    <div style={{ display: "flex", gap: 2, flex: "none" }}>
      {Array.from({ length: of }, (_, k) => <span key={k} style={{ width: w, height: 5, borderRadius: 4, background: k < n ? K.ac : K.bar }} />)}
    </div>
  );
}
export function HdrRow({ cols, labels }: { cols: string; labels: [string, boolean?][] }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: cols, gap: 10, font: `600 9.5px ${K.bf}`, letterSpacing: .5, textTransform: "uppercase", color: K.fa, paddingBottom: 3, borderBottom: `1px solid ${K.bd}` }}>
      {labels.map(([t, r], i) => <span key={i} style={r ? { textAlign: "right" } : undefined}>{t}</span>)}
    </div>
  );
}
export function Note({ children }: { children: ReactNode }) {
  return <div style={{ font: `400 10.5px/1.35 ${K.bf}`, color: K.fa }}>{children}</div>;
}
export function Empty({ children }: { children: ReactNode }) {
  return <div style={{ color: K.fa }}>{children}</div>;
}
/** Blit a cached offscreen canvas at CSS size `size`, pixelated. */
export function Blit({ src, size, style }: { src: HTMLCanvasElement; size: number; style?: CSSProperties }) {
  const ref = useRef<HTMLCanvasElement | null>(null);
  useEffect(() => {
    const el = ref.current; if (!el) return;
    el.width = src.width; el.height = src.height;
    const ctx = el.getContext("2d"); if (!ctx) return;
    ctx.imageSmoothingEnabled = false; ctx.clearRect(0, 0, el.width, el.height); ctx.drawImage(src, 0, 0);
  }, [src]);
  return <canvas ref={ref} style={{ width: size, height: size, flex: "none", display: "block", imageRendering: "pixelated", ...style }} />;
}
export function HeadStat({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <span style={{ color: K.fa, fontSize: 11, whiteSpace: "nowrap" }}>
      {label} <b style={{ color: color ?? K.tx, font: `700 15px ${K.hf}` }}>{value}</b>
    </span>
  );
}
export const chipStyle = (color: string = K.tx): CSSProperties => ({
  background: K.chip, color, border: `1px solid ${K.bd}`, borderRadius: 4, padding: "3px 9px",
  font: `600 10.5px/1.3 ${K.bf}`, backdropFilter: "blur(2px)",
});

/** The window frame + header row: mark · name · tag pill · subtitle · stats · close. */
export function WindowFrame({ mark, name, tag, sub, stats, onClose, onDragStart, children }: {
  mark: ReactNode; name: string; tag?: string; sub?: ReactNode; stats: [string, string, string?][];
  onClose?: () => void; onDragStart?: (e: React.PointerEvent<HTMLElement>) => void; children: ReactNode;
}) {
  return (
    <div style={{ background: K.bg, color: K.tx, border: `1px solid ${K.bd}`, borderRadius: 8, overflow: "hidden", font: `400 12px/1.4 ${K.bf}` }}>
      <div onPointerDown={onDragStart} style={{ display: "flex", alignItems: "center", gap: 12, padding: "12px 18px", background: K.head,
        borderBottom: `1px solid ${K.bd}`, flexWrap: "wrap", cursor: onDragStart ? "move" : undefined }}>
        {mark}
        <span style={{ font: `700 20px/1.1 ${K.hf}`, color: K.ac, whiteSpace: "nowrap" }}>{name}</span>
        {tag && <Pill strong>{tag}</Pill>}
        <span style={{ color: K.mu, fontSize: 12, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: "1 1 200px" }}>{sub}</span>
        {stats.map(([l, v, c]) => <HeadStat key={l} label={l} value={v} color={c} />)}
        {onClose && <span data-no-drag onClick={onClose} style={{ color: K.fa, fontSize: 14, paddingLeft: 4, cursor: "pointer" }} title="Close">✕</span>}
      </div>
      {children}
    </div>
  );
}
