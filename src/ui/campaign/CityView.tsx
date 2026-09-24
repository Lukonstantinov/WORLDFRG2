import { useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import type { CultureBrief, HubDetail, MerchantRoute, TradeFlows } from "@types";
import { campaignGetCultures, campaignMerchantRoutes, campaignTradeFlows } from "@bridge";
import { useCampaignStore } from "@state/campaignStore";
import { useWorldStore } from "@state/worldStore";
import { useGoodsStore } from "@state/goodsStore";
import { GOOD_DEFS } from "@goods";
import { GoodIcon } from "@ui/goods/GoodIcon";
import { koppenCode, koppenName } from "@ui/world/climate";
import { SERIF } from "@ui/campaign/chronicleTheme";
import {
  STYLES, WALL_LABEL, genCity, renderIso, presentIso, renderPlan, isoLandmarkAt, planLandmarkAt,
  landmarkIcon, vesselIcon, toHex,
  type CityCfg, type IsoRender, type PlanGeom, type StyleKey,
} from "@canvas/cityArt";
import {
  TIER_NAMES, CIVIC_COLOR, HULL_NAMES, VESSEL_ICONS, SCENE_CARAVAN,
  pickFamily, waterKind, popBucket, isWalled, deriveSlots, deriveDistricts, tierChecklist,
  seaShare, sceneShips, type SlotRow,
} from "@ui/campaign/settlementWindowData";

// ── The settlement window (design handoff: "WorldForge Settlement Window") ──
// A culture- and climate-styled pixel-isometric scene of the city (with a
// top-down Plan toggle), the development-tier ladder, and six cards: building
// slots, population, goods by mode, the vessel registry, manufacture chains and
// trade partners. The scene is `canvas/cityArt.ts`; every number here comes from
// HubDetail / TradeFlows / MerchantRoute — see `settlementWindowData.ts` for
// what is derived and what is deliberately hidden (faiths, a construction queue).
//
// Theme: this ships the CHRONICLE DARK variant only. The app has no light/dark
// switch for campaign windows (every other one is Chronicle dark), so the
// handoff's "Organic light" token set would have no way to be selected; it is
// queued rather than carried as dead tokens.

/** Chronicle-dark tokens for this window, from the handoff's token table. */
const K = {
  bg: "#0d1521", head: "#111b2a", card: "#0f1826", bd: "#1e2e42", tx: "#cfe2f6", mu: "#9fb4cc", fa: "#6f88a6",
  ac: "#d8b24a", acBg: "rgba(216,178,74,.14)", acBd: "rgba(216,178,74,.38)", pos: "#7fd0a0", neg: "#e8a07c",
  bar: "#1a2536", sea: "#5aa8d8", river: "#6fc3b0", land: "#d8a656", chip: "rgba(9,14,20,.84)", lock: "#4a5c72",
  scene: "#0a1018", hf: SERIF, bf: "system-ui,-apple-system,'Segoe UI',sans-serif",
} as const;
const SOCIETY = [
  { key: "patrician", label: "Patricians", color: "#c8813a" },
  { key: "burgher", label: "Burghers", color: "#5a8ac8" },
  { key: "commoner", label: "Commoners", color: "#6aa05a" },
  { key: "underclass", label: "Underclass", color: "#9a8a78" },
] as const;
const SCENE_W = 1180, SCENE_H = 420;
const GOOD_BY_NAME = new Map(GOOD_DEFS.map((g) => [g.name, g]));

const fk = (n: number) => n >= 1e5 ? Math.round(n / 1e3) + "k" : n >= 1e3 ? (n / 1e3).toFixed(1).replace(/\.0$/, "") + "k" : String(Math.round(n));
const fc = (n: number) => Math.round(n).toLocaleString("en-US");

// ── PNG SPRITE PACK support (public/city-sprites/<stem>.png) ────────────────
// Preserved from the previous city plan: drop one transparent PNG per building
// type into `public/city-sprites/` and it is blitted in place of the procedural
// landmark; a missing sprite falls back to the drawn building. PNG only — the
// `.svg` templates in that folder are superseded placeholders, not the art.
const SPRITE_BASE = "/city-sprites/";
const SPRITE_MAP: Record<string, string> = {
  Guildhall: "guildhall", Workshop: "workshop", Granary: "granary", Warehouse: "warehouse",
  Shipyard: "shipyard", Fondaco: "fondaco", Cathedral: "cathedral", Temple: "temple",
  Citadel: "citadel", Palace: "palace", "Council Hall": "council_hall", Mint: "mint",
  Bank: "bank", Harbor: "harbor",
};
type SpriteState = HTMLImageElement | "loading" | "error";
const spriteCache = new Map<string, SpriteState>();
function loadSprite(stem: string, onReady: () => void): HTMLImageElement | null {
  const cur = spriteCache.get(stem);
  if (cur instanceof HTMLImageElement) return cur;
  if (cur === "loading" || cur === "error") return null;
  spriteCache.set(stem, "loading");
  const img = new Image();
  img.onload = () => { spriteCache.set(stem, img); onReady(); };
  img.onerror = () => { spriteCache.set(stem, "error"); };
  img.src = `${SPRITE_BASE}${stem}.png`;
  return null;
}

// ── caches: culture kits (one fetch), rendered scenes (per hub/bucket/content) ──
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
/** The iso render is the expensive part (hundreds of shaded polygons); a campaign
 *  tick refetches HubDetail every day, so the scene is cached on exactly what it
 *  depends on and only regenerates when buildings, tier, population bucket (or
 *  the other drawn inputs) change. Small LRU — one entry per recently opened city. */
const sceneCache = new Map<string, IsoRender>();
function cachedScene(key: string, make: () => IsoRender): IsoRender {
  const hit = sceneCache.get(key);
  if (hit) { sceneCache.delete(key); sceneCache.set(key, hit); return hit; }
  const r = make();
  sceneCache.set(key, r);
  while (sceneCache.size > 8) { const k = sceneCache.keys().next().value; if (k === undefined) break; sceneCache.delete(k); }
  return r;
}

/** One-line lore/role for each building type — shown on hover + in the ward grid. */
export const BUILDING_INFO: Record<string, string> = {
  Guildhall: "Seat of the merchant guild; lowers freight on goods leaving the city.",
  Workshop: "Artisans' works — raises the city's output of manufactured goods.",
  Granary: "Public grain store; buffers famine and lifts food output.",
  Warehouse: "Bonded storage that smooths supply and adds a little output.",
  Shipyard: "Builds and berths the resident house's ships.",
  Fondaco: "A foreign merchants' quarter and trade house — a diaspora enclave.",
  Cathedral: "The city's great sanctuary; draws pilgrims and steadies civic mood.",
  Temple: "A holy precinct; its festivals lift the people and draw the faithful.",
  Citadel: "The fortified seat of power; defends the city and resists takeover.",
  Palace: "The ruling house's grand residence and court.",
  "Council Hall": "Where the polis council sits and sets tariff, mint and law.",
  Mint: "Strikes the polis's own coin.",
  Bank: "A counting-house extending credit across the trade network.",
  Harbor: "Docks and quays working the city's sea trade.",
};

// ── small presentational pieces ──────────────────────────────────────────────

function Card({ title, meta, span, children }: { title: string; meta?: ReactNode; span?: number; children: ReactNode }) {
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
function Bar({ frac, color, h = 6 }: { frac: number; color: string; h?: number }) {
  return (
    <div style={{ flex: 1, height: h, background: K.bar, borderRadius: 4, overflow: "hidden", minWidth: 0 }}>
      <div style={{ width: `${Math.max(0, Math.min(100, frac * 100))}%`, height: "100%", background: color, borderRadius: 4 }} />
    </div>
  );
}
function Stack({ parts, h = 8 }: { parts: [number, string][]; h?: number }) {
  return (
    <div style={{ display: "flex", height: h, borderRadius: 4, overflow: "hidden", gap: 2, background: K.bar }}>
      {parts.filter(([f]) => f > 0).map(([f, c], i) => <div key={i} style={{ flex: `${f} 0 0`, background: c }} />)}
    </div>
  );
}
function Spark({ vals, color, w = 180, h = 30 }: { vals: number[]; color: string; w?: number; h?: number }) {
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
function Pill({ children, strong }: { children: ReactNode; strong?: boolean }) {
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 4, background: strong ? K.acBg : "transparent",
      color: strong ? K.ac : K.mu, border: `1px solid ${strong ? K.acBd : K.bd}`, borderRadius: 4, padding: "2px 9px",
      font: `600 10px/1.3 ${K.bf}`, letterSpacing: .4, whiteSpace: "nowrap" }}>{children}</span>
  );
}
function Swatch({ color, size = 8, round }: { color: string; size?: number; round?: boolean }) {
  return <span style={{ width: size, height: size, borderRadius: round ? "50%" : 2, background: color, flex: "none", display: "inline-block" }} />;
}
/** Blit a cached offscreen canvas at CSS size `size`, pixelated. */
function Blit({ src, size, style }: { src: HTMLCanvasElement; size: number; style?: CSSProperties }) {
  const ref = useRef<HTMLCanvasElement | null>(null);
  useEffect(() => {
    const el = ref.current; if (!el) return;
    el.width = src.width; el.height = src.height;
    const ctx = el.getContext("2d"); if (!ctx) return;
    ctx.imageSmoothingEnabled = false; ctx.clearRect(0, 0, el.width, el.height); ctx.drawImage(src, 0, 0);
  }, [src]);
  return <canvas ref={ref} style={{ width: size, height: size, flex: "none", display: "block", imageRendering: "pixelated", ...style }} />;
}
const LOCK = (
  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.75" strokeLinecap="round" strokeLinejoin="round">
    <rect x="4" y="11" width="16" height="10" rx="2" /><path d="M8 11V7a4 4 0 0 1 8 0v4" />
  </svg>
);
const chipStyle = (color: string = K.tx): CSSProperties => ({
  background: K.chip, color, border: `1px solid ${K.bd}`, borderRadius: 4, padding: "3px 9px",
  font: `600 10.5px/1.3 ${K.bf}`, backdropFilter: "blur(2px)",
});

// ── the window ────────────────────────────────────────────────────────────────

export function CityView({ detail }: { detail: HubDetail }) {
  const economy = useWorldStore((s) => s.economy);
  const settlements = useWorldStore((s) => s.settlements);
  const clock = useCampaignStore((s) => s.snapshot?.clock);
  const hubBrief = useCampaignStore((s) => s.snapshot?.hubs.find((h) => h.id === detail.id));
  const specs = useGoodsStore((s) => s.specs);
  const [kits, setKits] = useState<Map<string, number> | null>(kitCache);
  const [flows, setFlows] = useState<TradeFlows | null>(null);
  const [routes, setRoutes] = useState<MerchantRoute[]>([]);
  const [mode, setMode] = useState<"iso" | "plan">("iso");
  const [spriteTick, setSpriteTick] = useState(0);
  const [hover, setHover] = useState<SlotRow | null>(null);
  const bumpRef = useRef(() => setSpriteTick((t) => t + 1));
  const cvRef = useRef<HTMLCanvasElement | null>(null);
  const isoRef = useRef<IsoRender | null>(null);
  const planRef = useRef<PlanGeom | null>(null);
  const year = clock?.year ?? 0;

  useEffect(() => { let alive = true; loadKits().then((m) => { if (alive) setKits(m); }); return () => { alive = false; }; }, []);
  // Flows refresh with the campaign day (they are a small per-hub payload); the
  // world's merchant-route list is large, so it is re-read once per year only.
  useEffect(() => {
    let alive = true;
    campaignTradeFlows(detail.id).then((f) => { if (alive) setFlows(f); }).catch(() => { if (alive) setFlows(null); });
    return () => { alive = false; };
  }, [detail.id, clock?.tick]);
  useEffect(() => {
    let alive = true;
    campaignMerchantRoutes().then((r) => { if (alive) setRoutes(r); }).catch(() => { if (alive) setRoutes([]); });
    return () => { alive = false; };
  }, [detail.id, year]);

  const econHub = economy?.hubs.find((h) => h.id === detail.id);
  const settlement = settlements.find((s) => s.name === detail.name);
  const kit = detail.culture ? kits?.get(detail.culture) : undefined;
  const style: StyleKey = pickFamily(detail.koppen, kit, detail.coastal, econHub?.elevation);
  const S = STYLES[style];
  const riverTrade = (flows?.goods ?? []).some((g) => (g.river_volume ?? 0) > 0)
    || (detail.vessels?.classes.find((c) => c.kind === "river")?.registered ?? 0) > 0;
  const water = waterKind({ coastal: detail.coastal, seaAccess: econHub?.sea_access, river: settlement?.site === "river" || riverTrade, family: style });
  const tier = detail.dev_tier ?? 0;
  const walled = isWalled(detail.population, tier);
  const bucket = popBucket(detail.population);

  const slots = useMemo(() => deriveSlots(detail, econHub?.sea_access), [detail, econHub?.sea_access]);
  const districts = useMemo(() => deriveDistricts(detail, slots), [detail, slots]);
  const caravanN = detail.vessels?.classes.find((c) => c.kind === "caravan")?.registered ?? 0;
  const cfg: CityCfg = useMemo(() => ({
    name: detail.name || "city", style, water, walled, popBucket: bucket,
    buildings: slots.map((s, i) => ({ kind: s.kind, status: s.status, color: s.color, ref: i })),
    districts: districts.map((d) => ({ name: d.name, color: d.color })),
    caravan: caravanN > 0 || (detail.in_by_land ?? 0) > 0 ? SCENE_CARAVAN[style] : null,
    ships: sceneShips(detail.vessels, water, detail.in_by_sea ?? 0),
  }), [detail.name, detail.vessels, detail.in_by_land, detail.in_by_sea, style, water, walled, bucket, slots, districts, caravanN]);
  const sceneKey = useMemo(() => [detail.id, cfg.name, style, water, walled ? 1 : 0, bucket, tier, cfg.ships, cfg.caravan ?? "",
    cfg.buildings.map((b) => `${b.kind}:${b.status}:${b.color}`).join(","), cfg.districts.map((d) => d.color).join(","), spriteTick].join("|"),
  [detail.id, cfg, style, water, walled, bucket, tier, spriteTick]);

  // Draw the scene (iso: cached low-res render blitted pixelated; plan: redrawn).
  useEffect(() => {
    const cv = cvRef.current; if (!cv) return;
    const sprites = (kind: string) => { const stem = SPRITE_MAP[kind]; return stem ? loadSprite(stem, bumpRef.current) : null; };
    const iso = cachedScene(sceneKey, () => renderIso(genCity(cfg), cfg, { W: SCENE_W, H: SCENE_H, sprites }));
    isoRef.current = iso;
    if (mode === "iso") { presentIso(cv, iso.canvas, SCENE_W, SCENE_H); planRef.current = null; }
    else planRef.current = renderPlan(cv, iso.city, cfg, SCENE_W, SCENE_H);
  }, [sceneKey, cfg, mode]);

  const onMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const cv = cvRef.current, iso = isoRef.current; if (!cv || !iso) return;
    const rect = cv.getBoundingClientRect();
    const fx = (e.clientX - rect.left) / rect.width, fy = (e.clientY - rect.top) / rect.height;
    const hit = mode === "iso"
      ? isoLandmarkAt(iso, fx * iso.geom.lw, fy * iso.geom.lh)
      : planRef.current ? planLandmarkAt(iso.city, planRef.current, fx * SCENE_W, fy * SCENE_H) : null;
    setHover(hit ? slots[hit.ref] ?? null : null);
  };

  const sub = [detail.government?.govt_type, detail.culture ? `of the ${detail.culture}` : ""].filter(Boolean).join(" ");
  const kc = koppenCode(detail.koppen);
  const cityColor = detail.government?.council && detail.government.council !== "—"
    ? toHex(detail.government.council_color, CIVIC_COLOR)
    : toHex(detail.culture_moods?.[0]?.color, CIVIC_COLOR);
  const check = tierChecklist(detail, hubBrief?.hub_class ?? 0, tier);

  return (
    <div style={{ background: K.bg, color: K.tx, border: `1px solid ${K.bd}`, borderRadius: 8, overflow: "hidden", font: `400 12px/1.4 ${K.bf}` }}>
      {/* ── header ── */}
      <div style={{ display: "flex", alignItems: "center", gap: 12, padding: "12px 18px", background: K.head, borderBottom: `1px solid ${K.bd}`, flexWrap: "wrap" }}>
        <span style={{ width: 14, height: 14, borderRadius: "50%", background: cityColor, boxShadow: `0 0 0 2px ${K.bd}`, flex: "none" }} />
        <span style={{ font: `700 20px/1.1 ${K.hf}`, color: K.ac }}>{detail.name}</span>
        {tier > 0 && <Pill strong>{`${(TIER_NAMES[tier] ?? "").toUpperCase()} · TIER ${tier} OF 5`}</Pill>}
        <span style={{ color: K.mu, fontSize: 12, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: "1 1 200px" }}>
          {sub && <>{sub} · </>}<span style={{ color: K.fa }}>{S.label}{kc ? ` · ${kc} · ${koppenName(detail.koppen)}` : ""}</span>
        </span>
        <HeadStat label="Pop" value={fc(detail.population)} />
        {typeof detail.treasury === "number" && <HeadStat label="Treasury" value={fc(detail.treasury)} color={K.ac} />}
        {detail.coin_name && <HeadStat label="Coin" value={detail.coin_name} />}
      </div>

      {/* ── scene ── */}
      <div style={{ position: "relative", background: K.scene, borderBottom: `1px solid ${K.bd}`, aspectRatio: `${SCENE_W} / ${SCENE_H}` }}>
        <canvas ref={cvRef} onMouseMove={onMove} onMouseLeave={() => setHover(null)}
          style={{ display: "block", width: "100%", height: "100%", imageRendering: mode === "iso" ? "pixelated" : "auto" }} />
        <div style={{ position: "absolute", left: 12, top: 12, display: "flex", gap: 6, pointerEvents: "none" }}>
          {clock && <span style={chipStyle()}>{clock.season ? `${clock.season} · ` : ""}{clock.year}</span>}
          <span style={chipStyle(walled ? K.pos : K.mu)}>{walled ? WALL_LABEL[S.wall] : "Unwalled"}</span>
          {kc && <span style={chipStyle()}>{kc} climate</span>}
        </div>
        <div data-no-drag style={{ position: "absolute", right: 12, top: 12, display: "flex", background: K.chip, border: `1px solid ${K.bd}`, borderRadius: 4, padding: 2, gap: 2 }}>
          {(["iso", "plan"] as const).map((m) => (
            <button key={m} onClick={() => setMode(m)} style={{ all: "unset", cursor: "pointer", padding: "4px 12px", borderRadius: 4,
              font: `600 11px/1.2 ${K.bf}`, background: mode === m ? K.ac : "transparent", color: mode === m ? "#0b1420" : K.mu }}>
              {m === "iso" ? "Isometric" : "Plan"}
            </button>
          ))}
        </div>
        {districts.length > 0 && (
          <div style={{ position: "absolute", left: 12, bottom: 12, display: "flex", gap: 6, flexWrap: "wrap", maxWidth: "62%", pointerEvents: "none" }}>
            {districts.map((d) => (
              <span key={d.name} style={{ display: "flex", alignItems: "center", gap: 6, background: K.chip, border: `1px solid ${K.bd}`,
                borderRadius: 4, padding: "3px 10px 3px 7px", font: `600 11px/1.3 ${K.bf}`, color: K.tx }}>
                <Swatch color={d.color} size={9} />{d.name}
                <span style={{ color: K.fa, fontWeight: 400 }}>{d.works} {d.works === 1 ? "work" : "works"}</span>
              </span>
            ))}
          </div>
        )}
        {hover && (
          <div style={{ position: "absolute", right: 12, bottom: 12, width: 300, background: K.chip, border: `1px solid ${K.bd}`,
            borderRadius: 6, padding: "6px 9px", fontSize: 11, pointerEvents: "none" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <span style={{ color: K.ac, fontWeight: 700, fontFamily: K.hf, fontSize: 13 }}>{hover.kind}</span>
              <span style={{ flex: 1 }} />
              {hover.owner && <><Swatch color={hover.color} /><span style={{ color: K.mu }}>{hover.owner}</span></>}
            </div>
            {BUILDING_INFO[hover.kind] && <div style={{ color: K.mu, marginTop: 2, lineHeight: 1.4 }}>{BUILDING_INFO[hover.kind]}</div>}
            <div style={{ color: K.pos, marginTop: 2 }}>{hover.note}</div>
            {hover.derived && <div style={{ color: K.fa, marginTop: 2 }}>Drawn because of {hover.derived}.</div>}
          </div>
        )}
      </div>

      {/* ── tier ladder ── */}
      <div style={{ display: "flex", alignItems: "center", gap: 14, padding: "10px 18px", borderBottom: `1px solid ${K.bd}`, background: K.head, flexWrap: "wrap" }}>
        <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
          {[1, 2, 3, 4, 5].map((t) => <span key={t} title={TIER_NAMES[t]} style={{ width: 26, height: 6, borderRadius: 4, background: t <= tier ? K.ac : K.bar }} />)}
        </div>
        <span style={{ font: `600 11px ${K.bf}`, color: K.mu, whiteSpace: "nowrap" }}>
          {check.next ? <>Toward <b style={{ color: K.tx }}>{check.next}</b>{check.need && <span style={{ color: K.fa, fontWeight: 400 }}> · {check.need}</span>}</>
            : <b style={{ color: K.tx }}>{tier >= 5 ? "Apex tier held" : "Not yet ranked"}</b>}
        </span>
        <div style={{ display: "flex", gap: 12, flexWrap: "wrap", minWidth: 0 }}>
          {check.checks.map((c) => (
            <span key={c.label} title={c.title} style={{ font: `400 11px ${K.bf}`, whiteSpace: "nowrap",
              color: c.state === "met" ? K.pos : K.fa, opacity: c.state === "unknown" ? .7 : 1 }}>
              {c.state === "met" ? "✓" : c.state === "unknown" ? "?" : "○"} {c.label}
            </span>
          ))}
        </div>
      </div>

      {/* ── cards ── */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(3, minmax(0, 1fr))", gap: 12, padding: "14px 16px 18px" }}>
        <SlotsCard slots={slots} style={style} />
        <PeopleCard detail={detail} spark={hubBrief?.pop_spark ?? []} growth={hubBrief?.growth} />
        <TradeCard detail={detail} flows={flows} />
        <FleetCard detail={detail} style={style} />
        <WorksCard detail={detail} flows={flows} specs={specs} />
        <PartnersCard detail={detail} flows={flows} routes={routes} />
      </div>
    </div>
  );
}

function HeadStat({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <span style={{ color: K.fa, fontSize: 11, whiteSpace: "nowrap" }}>
      {label} <b style={{ color: color ?? K.tx, font: `700 15px ${K.hf}` }}>{value}</b>
    </span>
  );
}

// ── cards ─────────────────────────────────────────────────────────────────────

function SlotsCard({ slots, style }: { slots: SlotRow[]; style: StyleKey }) {
  const standing = slots.filter((s) => s.status === "op").length;
  let n = 0;
  return (
    <Card span={2} title={`Building slots · ${standing} standing`}
      meta={<><span style={{ color: K.pos }}>■</span> operating · □ site · locked</>}>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 6 }}>
        {slots.map((s, i) => {
          const act = s.status === "op" || s.status === "build";
          if (act) n++;
          return (
            <div key={i} title={[BUILDING_INFO[s.kind], s.derived ? `Drawn because of ${s.derived}.` : ""].filter(Boolean).join(" ")}
              style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 8px 5px 5px", borderRadius: 6, minWidth: 0,
                border: `1px solid ${s.status === "build" ? K.acBd : K.bd}`, background: s.status === "build" ? K.acBg : "transparent",
                opacity: s.status === "lock" ? .62 : 1 }}>
              <Blit src={landmarkIcon(s.kind, style, s.status, s.color, 46)} size={46} style={{ opacity: s.status === "site" ? .4 : 1 }} />
              <div style={{ minWidth: 0, display: "flex", flexDirection: "column", gap: 1 }}>
                <div style={{ font: `700 13px/1.15 ${K.hf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                  {act && <span style={{ font: `600 9px ui-monospace,Menlo,monospace`, color: K.fa, marginRight: 4 }}>{n}</span>}{s.kind}
                </div>
                <div style={{ font: `400 10.5px/1.2 ${K.bf}`, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                  {s.status === "op" ? <span style={{ color: K.pos }}>{s.note || "Operating"}</span>
                    : s.status === "site" ? <span style={{ color: K.fa }}>{s.note}</span>
                    : <span style={{ color: K.lock, display: "inline-flex", gap: 3, alignItems: "center" }}>{LOCK} {s.note}</span>}
                </div>
                {act && s.owner && (
                  <div style={{ font: `400 10px/1.2 ${K.bf}`, color: K.fa, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis", display: "flex", alignItems: "center", gap: 4 }}>
                    <Swatch color={s.color} size={7} round />{s.owner}
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </Card>
  );
}

function PeopleCard({ detail, spark, growth }: { detail: HubDetail; spark: number[]; growth?: number }) {
  const soc = detail.society;
  const moods = [...(detail.culture_moods ?? [])].sort((a, b) => b.share - a.share).slice(0, 6);
  const sub = (t: string, m?: string) => (
    <div style={{ display: "flex", gap: 6, alignItems: "baseline", marginTop: 2, font: `600 9.5px/1 ${K.bf}`, letterSpacing: .6, textTransform: "uppercase", color: K.fa }}>
      {t}{m && <span style={{ font: `400 10px ${K.bf}`, letterSpacing: 0, textTransform: "none", marginLeft: "auto" }}>{m}</span>}
    </div>
  );
  const up = (growth ?? 0) >= 0;
  return (
    <Card title="Population" meta={typeof growth === "number" ? `${up ? "+" : ""}${(growth * 100).toFixed(1)}% / mo` : undefined}>
      <div style={{ display: "flex", alignItems: "flex-end", gap: 12 }}>
        <div style={{ font: `700 28px/1 ${K.hf}`, color: K.tx }}>{fc(detail.population)}</div>
        <div style={{ flex: 1, minWidth: 0 }}><Spark vals={spark} color={up ? K.pos : K.neg} /></div>
      </div>
      {soc && (
        <div style={{ display: "flex", flexDirection: "column", gap: 5 }}>
          <Stack h={9} parts={SOCIETY.map((s) => [soc[s.key], s.color] as [number, string])} />
          <div style={{ display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 4 }}>
            {SOCIETY.map((s) => (
              <div key={s.key} style={{ font: `400 10px/1.25 ${K.bf}`, color: K.fa }}>
                <Swatch color={s.color} size={7} /> {s.label}<br />
                <b style={{ color: K.tx, fontSize: 12 }}>{fk(detail.population * soc[s.key])}</b>
              </div>
            ))}
          </div>
        </div>
      )}
      {moods.length > 0 && (
        <>
          {sub("Peoples", "share · prized goods met")}
          {moods.map((m) => {
            const col = toHex(m.color);
            return (
              <div key={m.name} style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ width: 92, flex: "none", font: `600 11.5px ${K.bf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis", display: "flex", gap: 5, alignItems: "center" }}>
                  <Swatch color={col} round />{m.name}
                </span>
                <Bar frac={m.share} color={col} />
                <span style={{ width: 34, textAlign: "right", font: `600 11px ${K.bf}`, color: K.tx }}>{Math.round(m.share * 100)}%</span>
                <span style={{ width: 36, textAlign: "right", font: `400 10.5px ${K.bf}`, color: m.satisfaction > .6 ? K.pos : m.satisfaction > .4 ? K.mu : K.neg }}>{Math.round(m.satisfaction * 100)}%</span>
              </div>
            );
          })}
        </>
      )}
      {/* Faiths: the handoff proposes `HubDetail.faiths`; the sim models no
          religion per city yet, so the row is hidden rather than invented. */}
    </Card>
  );
}

const TRADE_COLS = "190px minmax(0,1fr) 54px minmax(0,1fr) 110px 60px";
function TradeCard({ detail, flows }: { detail: HubDetail; flows: TradeFlows | null }) {
  const goods = [...(flows?.goods ?? [])].filter((g) => g.in_volume + g.out_volume > 0)
    .sort((a, b) => (b.in_volume + b.out_volume) - (a.in_volume + a.out_volume)).slice(0, 10);
  const mx = Math.max(1e-6, ...goods.map((g) => Math.max(g.in_volume, g.out_volume)));
  const meta = (
    <>bought <b style={{ color: K.pos }}>{fk(detail.bought ?? 0)}</b> · sold <b style={{ color: K.neg }}>{fk(detail.sold ?? 0)}</b>
      {flows && <> · made here {fk(flows.produced_here)} · used {fk(flows.consumed_here)}</>}</>
  );
  return (
    <Card span={2} title="Imports & exports by good" meta={meta}>
      {!flows ? <div style={{ color: K.fa }}>Reading this city's trade…</div> : goods.length === 0 ? <div style={{ color: K.fa }}>No trade recorded in the last year.</div> : (
        <>
          <div style={{ display: "grid", gridTemplateColumns: TRADE_COLS, gap: 10, font: `600 9.5px ${K.bf}`, letterSpacing: .5, textTransform: "uppercase", color: K.fa, paddingBottom: 2, borderBottom: `1px solid ${K.bd}` }}>
            <span>Good</span><span style={{ textAlign: "right" }}>Imports</span><span /><span>Exports</span><span>By sea · river · land</span><span style={{ textAlign: "right" }}>Net</span>
          </div>
          {goods.map((g) => {
            const def = GOOD_BY_NAME.get(g.name);
            const tot = Math.max(1e-6, g.last_volume);
            const sea = Math.min(1, (g.sea_volume ?? 0) / tot), riv = Math.min(1 - sea, (g.river_volume ?? 0) / tot);
            const transit = !g.produced && g.in_volume > 0 && g.out_volume > 0 && Math.min(g.in_volume, g.out_volume) / Math.max(g.in_volume, g.out_volume) > .5;
            const net = g.out_volume - g.in_volume;
            const tag = (t: string, c: string) => <span style={{ font: `600 9px ${K.bf}`, color: c, border: `1px solid ${K.bd}`, borderRadius: 4, padding: "0 5px", whiteSpace: "nowrap" }}>{t}</span>;
            return (
              <div key={g.good} style={{ display: "grid", gridTemplateColumns: TRADE_COLS, gap: 10, alignItems: "center" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
                  <GoodIcon name={g.name} size={24} />
                  <span style={{ font: `600 12px ${K.bf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{def?.label ?? g.name}</span>
                  {g.produced && tag("made", K.pos)}{transit && tag("transit", K.mu)}
                </div>
                <div style={{ display: "flex", justifyContent: "flex-end", height: 8 }}><div style={{ width: `${g.in_volume / mx * 100}%`, background: K.pos, borderRadius: "4px 0 0 4px", opacity: .9 }} /></div>
                <div style={{ font: `400 10.5px/1.1 ${K.bf}`, color: K.fa, textAlign: "center", whiteSpace: "nowrap" }}>{g.in_volume ? fk(g.in_volume) : "–"} · {g.out_volume ? fk(g.out_volume) : "–"}</div>
                <div style={{ display: "flex", height: 8 }}><div style={{ width: `${g.out_volume / mx * 100}%`, background: K.neg, borderRadius: "0 4px 4px 0", opacity: .9 }} /></div>
                <Stack h={7} parts={[[sea, K.sea], [riv, K.river], [Math.max(0, 1 - sea - riv), K.land]]} />
                <span style={{ textAlign: "right", font: `600 12px ${K.bf}`, color: net >= 0 ? K.neg : K.pos }}>{net >= 0 ? "+" : "−"}{fk(Math.abs(net))}</span>
              </div>
            );
          })}
          <div style={{ display: "flex", gap: 14, font: `400 10.5px ${K.bf}`, color: K.fa, flexWrap: "wrap" }}>
            {([["bought in", K.pos], ["sent out", K.neg], ["sea", K.sea], ["river", K.river], ["caravan", K.land]] as [string, string][]).map(([l, c]) => (
              <span key={l} style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><Swatch color={c} />{l}</span>
            ))}
            <span style={{ marginLeft: "auto" }}>last full year · grain-eq units</span>
          </div>
        </>
      )}
    </Card>
  );
}

function FleetCard({ detail, style }: { detail: HubDetail; style: StyleKey }) {
  const v = detail.vessels;
  const names = HULL_NAMES[style], icons = VESSEL_ICONS[style];
  const bySea = seaShare(detail);
  return (
    <Card title="Fleet & caravan registry" meta={v ? `${v.houses} seated ${v.houses === 1 ? "house" : "houses"}` : undefined}>
      {!v ? <div style={{ color: K.fa }}>No hulls are registered to houses seated here.</div> : (
        <>
          {v.classes.map((c) => {
            const col = c.kind === "sea" ? K.sea : c.kind === "river" ? K.river : K.land;
            return (
              <div key={c.kind} style={{ display: "flex", alignItems: "center", gap: 10, opacity: c.registered ? 1 : .45 }}>
                <Blit src={vesselIcon(icons[c.kind], style, 40)} size={40} style={{ borderRadius: 6 }} />
                <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 4 }}>
                  <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                    <span style={{ font: `700 13px ${K.hf}`, color: K.tx }}>{names[c.kind]}</span><span style={{ flex: 1 }} />
                    <b style={{ font: `700 16px ${K.hf}`, color: K.tx }}>{c.registered}</b><span style={{ font: `400 10px ${K.bf}`, color: K.fa }}>hulls</span>
                  </div>
                  <Stack h={7} parts={[[c.away, col], [c.idle, K.bar]]} />
                  <div style={{ font: `400 10.5px ${K.bf}`, color: K.fa }}>
                    {c.registered ? <><b style={{ color: K.tx }}>{c.away}</b> away carrying · <b style={{ color: K.tx }}>{c.idle}</b> idle in port</> : "none registered"}
                  </div>
                </div>
              </div>
            );
          })}
          <div style={{ display: "grid", gridTemplateColumns: "repeat(3, minmax(0, 1fr))", gap: 6, borderTop: `1px solid ${K.bd}`, paddingTop: 10 }}>
            <IoBox label="Inbound" value={String(v.inbound_cargoes)} sub={v.inbound_cargoes > 0 ? `cargoes · first in ${Math.round(v.inbound_eta)} d` : "cargoes"} />
            <IoBox label="Outbound" value={String(v.outbound_cargoes)} sub="cargoes in flight" />
            {bySea !== null && <IoBox label="Supply by sea" value={`${Math.round(bySea * 100)}%`} sub={`${Math.round((1 - bySea) * 100)}% overland`} />}
          </div>
          {v.land_pooled && <div style={{ font: `400 10px/1.35 ${K.bf}`, color: K.fa }}>River and caravan slots are pooled by dispatch — the land total is exact, the split indicative.</div>}
          <div style={{ font: `400 10px/1.35 ${K.bf}`, color: K.fa }}>Hull names are this city's building tradition, not sim types — a vessel is a counted slot, not an entity.</div>
        </>
      )}
    </Card>
  );
}
function IoBox({ label, value, sub }: { label: string; value: string; sub: string }) {
  return (
    <div>
      <div style={{ font: `600 9.5px ${K.bf}`, letterSpacing: .5, textTransform: "uppercase", color: K.fa }}>{label}</div>
      <div style={{ font: `700 18px/1.2 ${K.hf}`, color: K.tx }}>{value}</div>
      <div style={{ font: `400 10px ${K.bf}`, color: K.fa }}>{sub}</div>
    </div>
  );
}

function WorksCard({ detail, flows, specs }: { detail: HubDetail; flows: TradeFlows | null; specs: { id: string; name: string; inputs?: { good: string; qty: number }[] }[] }) {
  const works = (detail.estates_here ?? []).filter((e) => e.kind === 6);
  if (works.length === 0) {
    return <Card span={2} title="Manufactures · input chains"><div style={{ color: K.fa }}>No manufactory stands in this city.</div></Card>;
  }
  const specOf = (g: string) => specs.find((s) => s.id === g || s.name === g);
  const madeHere = new Set((flows?.goods ?? []).filter((g) => g.produced).map((g) => g.name));
  const ownEstate = new Set((detail.estates_here ?? []).filter((e) => e.kind !== 6).map((e) => e.good));
  const goodIdx = new Map((flows?.goods ?? []).map((g) => [g.name, g.good]));
  const topSupplier = (name: string): string | null => {
    const gi = goodIdx.get(name); if (gi === undefined) return null;
    const rs = (flows?.routes ?? []).filter((r) => r.good === gi && r.dir === 0).sort((a, b) => b.amount - a.amount);
    return rs[0]?.partner_name ?? null;
  };
  return (
    <Card span={2} title="Manufactures · input chains" meta={`${works.length} ${works.length === 1 ? "manufactory" : "manufactories"}`}>
      {works.map((w, i) => {
        const inputs = specOf(w.good)?.inputs ?? [];
        const owner = w.owner_is_civic ? "Civic" : w.owner;
        const ocol = w.owner_is_civic ? CIVIC_COLOR : (detail.houses ?? []).find((h) => h.name === w.owner)?.color ?? CIVIC_COLOR;
        return (
          <div key={w.hub} style={{ display: "grid", gridTemplateColumns: "minmax(0,1.25fr) 18px minmax(0,1fr) 150px", gap: 10, alignItems: "center", padding: "6px 0", borderBottom: i < works.length - 1 ? `1px solid ${K.bd}` : "none" }}>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", minWidth: 0 }}>
              {inputs.length === 0 && <span style={{ color: K.fa, fontSize: 11 }}>No recipe inputs</span>}
              {inputs.map((inp) => {
                const src = ownEstate.has(inp.good) ? "own estate" : madeHere.has(inp.good) ? "local market" : topSupplier(inp.good);
                const local = src === "own estate" || src === "local market";
                return (
                  <div key={inp.good} style={{ display: "flex", alignItems: "center", gap: 6, padding: "3px 8px 3px 4px", border: `1px solid ${K.bd}`, borderRadius: 4 }}>
                    <GoodIcon name={inp.good} size={22} />
                    <span style={{ font: `400 11px/1.15 ${K.bf}`, color: K.tx, whiteSpace: "nowrap" }}>
                      {GOOD_BY_NAME.get(inp.good)?.label ?? inp.good} <b>×{inp.qty}</b><br />
                      <span style={{ fontSize: 9.5, color: local ? K.pos : K.ac }}>{src ? (local ? src : `imported · ${src}`) : "no supply recorded"}</span>
                    </span>
                  </div>
                );
              })}
            </div>
            <span style={{ color: K.fa, fontSize: 15, textAlign: "center" }}>→</span>
            <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
              <GoodIcon name={w.good} size={32} />
              <div style={{ minWidth: 0 }}>
                <div style={{ font: `700 13.5px/1.15 ${K.hf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{GOOD_BY_NAME.get(w.good)?.label ?? w.good}</div>
                <div style={{ font: `400 11px ${K.bf}`, color: K.mu }}><b style={{ color: K.tx }}>{fc(w.output * 365)}</b> / yr{(w.damage ?? 0) > 0.05 && <span style={{ color: K.neg }}> · {Math.round((w.damage ?? 0) * 100)}% damaged</span>}</div>
              </div>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 3, alignItems: "flex-end" }}>
              <div style={{ display: "flex", gap: 2 }}>{[1, 2, 3, 4, 5].map((k) => <span key={k} style={{ width: 10, height: 5, borderRadius: 4, background: k <= w.tier ? K.ac : K.bar }} />)}</div>
              <span style={{ font: `400 10.5px ${K.bf}`, color: K.mu, display: "flex", gap: 4, alignItems: "center", whiteSpace: "nowrap", maxWidth: 150, overflow: "hidden", textOverflow: "ellipsis" }}>
                <Swatch color={toHex(ocol)} size={7} round />{owner}
              </span>
            </div>
          </div>
        );
      })}
    </Card>
  );
}

function PartnersCard({ detail, flows, routes }: { detail: HubDetail; flows: TradeFlows | null; routes: MerchantRoute[] }) {
  const partners = [...(flows?.partners ?? [])].sort((a, b) => b.pct - a.pct).slice(0, 6);
  /** Mode + days from the matching merchant route; else the dominant mode of this
   *  city's own recorded flows with that partner (no days then). */
  const modeOf = (hub: number, name: string): { mode: "sea" | "river" | "road"; days?: number } => {
    const rs = routes.filter((r) => (r.a_name === detail.name && r.b_name === name) || (r.b_name === detail.name && r.a_name === name));
    if (rs.length) {
      const r = rs.reduce((a, b) => (b.volume > a.volume ? b : a));
      return { mode: r.sea ? "sea" : r.river ? "river" : "road", days: r.days };
    }
    const fl = (flows?.routes ?? []).filter((r) => r.partner === hub);
    const tot = fl.reduce((s, r) => s + r.amount, 0), sea = fl.reduce((s, r) => s + (r.sea_amount ?? 0), 0), riv = fl.reduce((s, r) => s + (r.river_amount ?? 0), 0);
    if (tot <= 0) return { mode: "road" };
    return { mode: sea >= riv && sea > tot - sea - riv ? "sea" : riv > tot - sea - riv ? "river" : "road" };
  };
  return (
    <Card title="Trade partners & routes" meta="share of our book">
      {partners.length === 0 ? <div style={{ color: K.fa }}>{flows ? "No trade partners recorded yet." : "Reading this city's trade…"}</div> : (
        <>
          {partners.map((p) => {
            const { mode, days } = modeOf(p.hub, p.name);
            const ip = Math.round(p.in_pct ?? 0), op = Math.round(p.out_pct ?? 0);
            const mcol = mode === "sea" ? K.sea : mode === "river" ? K.river : K.land;
            return (
              <div key={p.hub} style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
                  <span style={{ font: `700 13px ${K.hf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{p.name}</span>
                  <span style={{ font: `600 9px ${K.bf}`, color: mcol, border: `1px solid ${K.bd}`, borderRadius: 4, padding: "0 6px", whiteSpace: "nowrap" }}>
                    {mode}{typeof days === "number" ? ` · ${Math.round(days)} d` : ""}
                  </span>
                  <span style={{ flex: 1 }} />
                  {p.goods.slice(0, 3).map((g) => <GoodIcon key={g} name={g} size={18} />)}
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <span style={{ width: 40, font: `400 10.5px ${K.bf}`, color: K.pos, textAlign: "right" }}>{ip}%</span>
                  <div style={{ flex: 1, display: "flex", justifyContent: "flex-end", height: 6 }}><div style={{ width: `${Math.min(100, ip * 2.2)}%`, background: K.pos, borderRadius: "4px 0 0 4px" }} /></div>
                  <span style={{ width: 1, height: 10, background: K.bd }} />
                  <div style={{ flex: 1, display: "flex", height: 6 }}><div style={{ width: `${Math.min(100, op * 2.2)}%`, background: K.neg, borderRadius: "0 4px 4px 0" }} /></div>
                  <span style={{ width: 40, font: `400 10.5px ${K.bf}`, color: K.neg }}>{op}%</span>
                </div>
              </div>
            );
          })}
          <div style={{ font: `400 10px ${K.bf}`, color: K.fa }}>
            <span style={{ color: K.pos }}>left</span> share of our imports · <span style={{ color: K.neg }}>right</span> share of our exports
          </div>
        </>
      )}
    </Card>
  );
}
