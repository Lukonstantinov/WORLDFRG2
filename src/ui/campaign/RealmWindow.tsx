import { useEffect, useMemo, useRef, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { useWorldStore, type ProvinceRaster } from "@state/worldStore";
import {
  computeStates, campaignRealmOverview, campaignGetRealmFamily, campaignRealmWatch, campaignProvinceGoods, campaignGetGovernment,
  getStartRealms, setStartRealms, regenerateStartRealms, getRealmStartMode, setRealmStartMode,
} from "@bridge";
import type { PersonBrief, RealmWatch, StateRegion, RealmOverview, RealmProvinceRow, ProvinceGoodExploit, GovernmentBrief, StartRealm } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE, SERIF } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, Tabs, Badge, Chip, Ledger } from "@ui/kit";
import { GoodIcon } from "@ui/goods/GoodIcon";
import { CouncilChamber } from "@ui/campaign/CouncilChamber";
import { drawPixelIcon } from "@canvas/goodArt";
import { GOOD_DEFS } from "@goods";

/** THE ONE REALM WINDOW (PROVINCES_REALMS_LOGISTICS_REDESIGN.md §5/§8, mock-up v2):
 *  realm › province › city in one window, a realm map with toggles on the left and
 *  tabs on the right. In a campaign it reads the live realms; before one, it edits the
 *  world's authored starting realms (recolour, rename, assign provinces, new realm,
 *  regenerate) and what a new campaign does with them. */

const fmt = (n: number) => n >= 1e6 ? `${(n / 1e6).toFixed(2)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(1)}k` : `${Math.round(n)}`;
const goodColor = (g: string) => GOOD_DEFS.find((d) => d.name === g)?.color ?? "#9a8a70";
const label = (g: string) => GOOD_DEFS.find((d) => d.name === g)?.label ?? g.replace(/_/g, " ");
const PALETTE: [number, number, number][] = [
  [176, 71, 58], [63, 111, 168], [201, 154, 58], [79, 138, 90], [138, 79, 160], [47, 143, 138],
  [168, 96, 58], [106, 125, 47], [176, 74, 122], [74, 90, 160], [154, 138, 58], [58, 122, 90],
  [192, 106, 80], [90, 74, 138], [122, 154, 74], [168, 58, 90],
];
const RANKS = ["City-state", "Kingdom", "Great power", "Hegemon"];
const LEVEL = ["", "light", "moderate", "heavy"];
const ORDER = (u: number) => (u > 0.6 ? "seething" : u > 0.4 ? "restless" : u > 0.2 ? "settled" : "calm");
const hashColor = (s: string): [number, number, number] => {
  let h = 2166136261; for (const c of s) { h ^= c.charCodeAt(0); h = Math.imul(h, 16777619); }
  return PALETTE[(h >>> 0) % PALETTE.length];
};

type Fill = "realm" | "culture" | "order";
interface Owner { realm: number; color: [number, number, number] }

/** Province centroids from the raster, once per raster. */
const centroidCache = new WeakMap<ProvinceRaster, Map<number, [number, number]>>();
function centroids(r: ProvinceRaster): Map<number, [number, number]> {
  const hit = centroidCache.get(r); if (hit) return hit;
  const acc = new Map<number, [number, number, number]>();
  for (let i = 0; i < r.w * r.h; i++) {
    const id = r.data[i]; if (id === 0xffffffff) continue;
    const a = acc.get(id) ?? [0, 0, 0]; a[0] += i % r.w; a[1] += Math.floor(i / r.w); a[2]++; acc.set(id, a);
  }
  const sx = r.gridW / r.w, sy = r.gridH / r.h;
  const out = new Map<number, [number, number]>();
  for (const [id, a] of acc) out.set(id, [(a[0] / a[2]) * sx, (a[1] / a[2]) * sy]);
  centroidCache.set(r, out);
  return out;
}

/** The realm map: provinces filled by realm / culture / order, province and realm
 *  borders, each province's main exports as icons, towns, and country → market arrows. */
function RealmMap({ raster, owner, focus, fill, marks, rows, towns, hover, onHover, onPick }: {
  raster: ProvinceRaster; owner: Map<number, Owner>; focus: Set<number>; fill: Fill;
  marks: { borders: boolean; goods: boolean; cities: boolean; trade: boolean };
  rows: Map<number, RealmProvinceRow>; towns: { x: number; y: number; id: number; capital: boolean }[];
  hover: number | null; onHover: (p: number | null) => void; onPick: (p: number, town: number | null) => void;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  const W = 520, H = 380;
  const view = useMemo(() => {
    const cents = centroids(raster);
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (const p of focus) { const c = cents.get(p); if (!c) continue; x0 = Math.min(x0, c[0]); x1 = Math.max(x1, c[0]); y0 = Math.min(y0, c[1]); y1 = Math.max(y1, c[1]); }
    if (!isFinite(x0)) { x0 = 0; y0 = 0; x1 = raster.gridW; y1 = raster.gridH; }
    const span = Math.max(x1 - x0, (y1 - y0) * (W / H), raster.gridW * 0.04);
    const pad = span * 0.35;
    const s = W / (span + pad * 2);
    const cx = (x0 + x1) / 2, cy = (y0 + y1) / 2;
    return { x0: cx - W / 2 / s, y0: cy - H / 2 / s, s };
  }, [raster, focus]);
  const pidAt = (px: number, py: number) => {
    const wx = view.x0 + px / view.s, wy = view.y0 + py / view.s;
    const gx = ((wx % raster.gridW) + raster.gridW) % raster.gridW;
    const rx = Math.floor((gx / raster.gridW) * raster.w), ry = Math.floor((wy / raster.gridH) * raster.h);
    if (ry < 0 || ry >= raster.h) return null;
    const id = raster.data[ry * raster.w + rx];
    return id === 0xffffffff ? null : id;
  };
  useEffect(() => {
    const ctx = ref.current?.getContext("2d"); if (!ctx) return;
    const img = ctx.createImageData(W, H);
    const ids = new Int32Array(W * H);
    for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) { const p = pidAt(x, y); ids[y * W + x] = p ?? -1; }
    for (let i = 0; i < W * H; i++) {
      const p = ids[i]; const o = i * 4;
      if (p < 0) { img.data.set([30, 44, 52, 255], o); continue; }
      const own = owner.get(p); const mine = focus.has(p);
      let c: [number, number, number] = own?.color ?? [120, 112, 96];
      if (fill === "culture") c = hashColor(rows.get(p)?.culture ?? String(own?.realm ?? p));
      if (fill === "order") { const u = rows.get(p)?.unrest ?? 0.2; c = u > 0.6 ? [163, 38, 42] : u > 0.4 ? [192, 106, 42] : u > 0.2 ? [201, 162, 58] : [79, 138, 90]; }
      const shade = mine ? 0.86 + ((p * 37) % 7) * 0.045 : 0.55;
      const g = (c[0] + c[1] + c[2]) / 3;
      const mix = (v: number) => Math.round(mine ? v * shade : (v * 0.35 + g * 0.65) * shade);
      let rgb = [mix(c[0]), mix(c[1]), mix(c[2])];
      if (p === hover) rgb = rgb.map((v) => Math.min(255, v + 40));
      if (marks.borders) {
        const x = i % W, y = Math.floor(i / W);
        const right = x + 1 < W ? ids[i + 1] : p, down = y + 1 < H ? ids[i + W] : p;
        const edge = (q: number) => q >= 0 && q !== p;
        if (edge(right) || edge(down)) {
          const q = edge(right) ? right : down;
          const frontier = (owner.get(q)?.realm ?? -1) !== (own?.realm ?? -2);
          rgb = frontier ? [18, 13, 8] : rgb.map((v) => Math.round(v * 0.55));
        }
      }
      img.data.set([rgb[0], rgb[1], rgb[2], 255], o);
    }
    ctx.putImageData(img, 0, 0);
    const toPx = (x: number, y: number): [number, number] => [(x - view.x0) * view.s, (y - view.y0) * view.s];
    const cents = centroids(raster);
    // Country → market: each province's surplus to its own market town.
    if (marks.trade) for (const p of focus) {
      const r = rows.get(p); const c = cents.get(p); if (!r || !c || r.town_hub < 0) continue;
      const t = towns.find((tt) => tt.id === r.town_hub); if (!t) continue;
      const [ax, ay] = toPx(c[0], c[1]); const [bx, by] = toPx(t.x, t.y);
      ctx.strokeStyle = goodColor(r.exports[0]?.good ?? ""); ctx.lineWidth = 2.5; ctx.globalAlpha = 0.85;
      ctx.beginPath(); ctx.moveTo(ax, ay + 14); ctx.quadraticCurveTo((ax + bx) / 2 + 12, (ay + by) / 2, bx, by); ctx.stroke(); ctx.globalAlpha = 1;
    }
    ctx.textAlign = "center"; ctx.textBaseline = "middle";
    for (const p of focus) {
      const c = cents.get(p); const r = rows.get(p); if (!c) continue;
      const [px, py] = toPx(c[0], c[1]); if (px < -20 || py < -20 || px > W + 20 || py > H + 20) continue;
      if (marks.goods && r) {
        const ex = r.exports.slice(0, 3);
        ex.forEach((e, k) => {
          const gx = px + (k - (ex.length - 1) / 2) * 26, gy = py + 14;
          ctx.fillStyle = "rgba(245,239,226,.85)"; ctx.beginPath(); ctx.arc(gx, gy, 12, 0, 7); ctx.fill();
          drawPixelIcon(ctx, gx, gy, 20, goodColor(e.good), e.good);
        });
      }
      const name = r?.name ?? "";
      ctx.font = `600 11px ${SERIF}`; ctx.lineWidth = 3; ctx.strokeStyle = "rgba(10,8,5,.8)"; ctx.fillStyle = "#f5efe2";
      ctx.strokeText(name.toUpperCase(), px, py - 8); ctx.fillText(name.toUpperCase(), px, py - 8);
    }
    if (marks.cities) for (const t of towns) {
      const [x, y] = toPx(t.x, t.y); if (x < 0 || y < 0 || x > W || y > H) continue;
      ctx.fillStyle = "#14110d"; ctx.beginPath(); ctx.arc(x, y, t.capital ? 6 : 4, 0, 7); ctx.fill();
      ctx.fillStyle = t.capital ? T.gold : T.ink; ctx.beginPath(); ctx.arc(x, y, t.capital ? 4 : 2.5, 0, 7); ctx.fill();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [raster, owner, focus, fill, marks, rows, towns, hover, view]);
  const local = (e: React.MouseEvent<HTMLCanvasElement>): [number, number] => {
    const b = e.currentTarget.getBoundingClientRect();
    return [((e.clientX - b.left) / b.width) * W, ((e.clientY - b.top) / b.height) * H];
  };
  return (
    <canvas ref={ref} width={W} height={H} data-no-drag
      style={{ width: "100%", aspectRatio: `${W} / ${H}`, display: "block", border: `1px solid ${T.line}`, borderRadius: 3, cursor: "pointer" }}
      onMouseMove={(e) => { const [x, y] = local(e); onHover(pidAt(x, y)); }}
      onMouseLeave={() => onHover(null)}
      onClick={(e) => {
        const [x, y] = local(e); const p = pidAt(x, y); if (p === null) return;
        const t = towns.find((tt) => { const tx = (tt.x - view.x0) * view.s, ty = (tt.y - view.y0) * view.s; return (tx - x) ** 2 + (ty - y) ** 2 < 100; });
        onPick(p, t ? t.id : null);
      }} />
  );
}

function Meter({ v }: { v: number }) {
  const n = Math.round(Math.max(0, Math.min(1, v)) * 4);
  return <span style={{ color: v > 0.5 ? T.goodInk : v > 0.25 ? T.warn : T.badInk, letterSpacing: 1 }}>{"▰".repeat(n)}{"▱".repeat(4 - n)} {Math.round(v * 100)}%</span>;
}

export function RealmWindow() {
  const open = useUIStore((s) => s.showStates);
  const close = () => useUIStore.getState().setShowStates(false);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const raster = useWorldStore((s) => s.provinceRaster);
  const provinces = useWorldStore((s) => s.provinces);
  const active = !!snapshot?.active;
  const year = Math.floor((snapshot?.clock?.tick ?? 0) / 365);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.province);

  const [states, setStates] = useState<StateRegion[]>([]);
  const [start, setStart] = useState<StartRealm[]>([]);
  const [mode, setMode] = useState("continue");
  const [realm, setRealm] = useState(0);
  const [ov, setOv] = useState<RealmOverview | null>(null);
  const [prov, setProv] = useState<number | null>(null);
  const [city, setCity] = useState<number | null>(null);
  const [tab, setTab] = useState("overview");
  const [fill, setFill] = useState<Fill>("realm");
  const [marks, setMarks] = useState({ borders: true, goods: true, cities: true, trade: false });
  const [hover, setHover] = useState<number | null>(null);
  const [assign, setAssign] = useState(false);
  const [palFor, setPalFor] = useState<number | null>(null);
  const [goods, setGoods] = useState<ProvinceGoodExploit[]>([]);
  const [gov, setGov] = useState<GovernmentBrief | null>(null);
  const [status, setStatus] = useState("");
  const [family, setFamily] = useState<PersonBrief[] | undefined>(undefined);
  const [watch, setWatch] = useState<RealmWatch | null>(null);

  useEffect(() => {
    if (!open) return;
    if (active) {
      computeStates().then(setStates).catch(() => setStates([]));
      campaignRealmWatch().then(setWatch).catch(() => setWatch(null));
    }
    else {
      getStartRealms().then(setStart).catch(() => setStart([]));
      getRealmStartMode().then(setMode).catch(() => {});
    }
  }, [open, active, year]);
  useEffect(() => {
    if (!open || !active || !states[realm]) { setOv(null); return; }
    campaignRealmOverview(states[realm].id).then(setOv).catch(() => setOv(null));
    campaignGetRealmFamily(states[realm].id).then(setFamily).catch(() => setFamily(undefined));
  }, [open, active, states, realm, year]);
  useEffect(() => { if (prov !== null && active) campaignProvinceGoods(prov).then(setGoods).catch(() => setGoods([])); }, [prov, active]);
  useEffect(() => { if (city !== null) campaignGetGovernment(city).then(setGov).catch(() => setGov(null)); }, [city]);

  const owner = useMemo(() => {
    const m = new Map<number, Owner>();
    if (active) states.forEach((st, k) => st.province_ids.forEach((p) => m.set(p, { realm: k, color: st.color })));
    else start.forEach((st, k) => st.provinces.forEach((p) => m.set(p, { realm: k, color: st.color })));
    return m;
  }, [active, states, start]);
  const focus = useMemo(() => new Set(prov !== null ? [prov] : active ? states[realm]?.province_ids ?? [] : start[realm]?.provinces ?? []), [active, states, start, realm, prov]);
  const rows = useMemo(() => {
    const m = new Map<number, RealmProvinceRow>();
    if (ov) ov.provinces.forEach((r) => m.set(r.id, r));
    else for (const p of provinces) m.set(p.id, { id: p.id, name: p.name, rural: p.rural_pop, urban: 0, tax: 0, unrest: 0.2, culture: p.culture, exports: [], governor: null, capital: false, town_hub: -1 });
    return m;
  }, [ov, provinces]);
  const towns = useMemo(() => (snapshot?.hubs ?? []).filter((h) => !h.is_estate && rows.get(raster ? (centroidOf(raster, h.x, h.y) ?? -1) : -1) && focus.has(centroidOf(raster!, h.x, h.y) ?? -1))
    .map((h) => ({ x: h.x, y: h.y, id: h.id, capital: h.id === ov?.capital_hub })), [snapshot, rows, focus, raster, ov]);

  if (!open) return null;
  const pickProvince = (p: number, town: number | null) => {
    if (!active && assign) {
      const next = start.map((r) => ({ ...r, provinces: r.provinces.filter((q) => q !== p) }));
      next[realm] = { ...next[realm], provinces: [...next[realm].provinces, p] };
      // A realm that lost its capital takes its largest remaining province instead.
      for (const r of next) if (!r.provinces.includes(r.capital) && r.provinces.length) r.capital = r.provinces[0];
      const kept = next.filter((r) => r.provinces.length);
      setStart(kept); setRealm(Math.max(0, kept.indexOf(next[realm]))); setStatus("unsaved changes");
      return;
    }
    if (active && town !== null && prov === p) { setCity(town); setTab("city"); return; }
    if (!owner.has(p) || owner.get(p)!.realm !== realm) { const o = owner.get(p); if (o) { setRealm(o.realm); setProv(null); setTab("overview"); } return; }
    setProv(p); setCity(null); setTab("province");
  };

  const head = active ? states[realm] : null;
  const sr = !active ? start[realm] : null;
  const title = head?.name ?? sr?.name ?? "Realms";
  const color = head?.color ?? sr?.color ?? [120, 112, 96];
  const prow = prov !== null ? rows.get(prov) : undefined;

  const tabs: (readonly [string, string])[] = !active ? [["edit", "Edit realms"], ["overview", "Overview"]]
    : city !== null ? [["city", "City government"], ["province", "Province"]]
    : prov !== null ? [["province", prow?.name ?? "Province"], ["pgov", "Government"], ["overview", "Realm"]]
    : [["overview", "Overview"], ["gov", "Government"], ["provinces", "Provinces"], ["goods", "Goods"]];
  const curTab = tabs.some(([k]) => k === tab) ? tab : tabs[0][0];

  return (
    <Panel onPointerDown={onPointerDown} width={1000} maxHeight="88vh" style={{ top: 50, right: 40, zIndex: 41, ...rootStyle }}>
      <PanelHeader icon="👑" title={title} onDragStart={onPointerDown} onClose={close}
        right={head ? <Badge tone="gold">{RANKS[head.rank] ?? "Realm"}</Badge> : undefined} />
      {/* Breadcrumb + realm picker */}
      <div style={{ display: "flex", gap: SPACE.md, alignItems: "center", padding: `4px ${SPACE.lg}px`, fontSize: FZ.small, color: T.inkDim, flexWrap: "wrap" }} data-no-drag>
        <span style={{ width: 14, height: 14, borderRadius: 2, background: `rgb(${color.join(",")})`, border: "1px solid #000" }} />
        <select value={realm} onChange={(e) => { setRealm(Number(e.target.value)); setProv(null); setCity(null); setTab(active ? "overview" : "edit"); }}
          style={{ background: T.panel, color: T.ink, border: `1px solid ${T.line}`, fontSize: FZ.small }}>
          {(active ? states.map((s) => s.name) : start.map((s) => s.name)).map((n, k) => <option key={k} value={k}>{n}</option>)}
        </select>
        {prov !== null && <span>› <a style={{ color: T.gold, cursor: "pointer" }} onClick={() => { setProv(null); setCity(null); setTab("overview"); }}>{title}</a> › {city !== null ? <><a style={{ color: T.gold, cursor: "pointer" }} onClick={() => { setCity(null); setTab("province"); }}>{prow?.name}</a> › town</> : prow?.name}</span>}
        <span style={{ flex: 1 }} />
        {!active && <span>Before the campaign: these are the world's starting realms.</span>}
      </div>
      {ov && prov === null && (
        <div style={{ display: "grid", gridTemplateColumns: "repeat(5, minmax(0,1fr))", borderTop: `1px solid ${T.line}`, borderBottom: `1px solid ${T.line}` }}>
          {[["Economy index", String(ov.economy_index), "world average = 100"], ["Treasury", fmt(ov.treasury), `+${fmt(ov.revenue)} a year`],
            ["Realm power", fmt(ov.power), ov.power_word], ["Population", fmt(ov.urban + ov.rural), `${fmt(ov.urban)} in towns`], ["Provinces", String(ov.provinces.length), `${ov.provinces.filter((p) => p.unrest > 0.4).length} restless`]]
            .map(([k, v, d]) => (
              <div key={k} style={{ padding: "6px 12px", borderRight: `1px solid ${T.lineSoft}` }}>
                <div style={{ fontSize: FZ.micro, letterSpacing: 1, textTransform: "uppercase", color: T.inkDim }}>{k}</div>
                <div style={{ fontFamily: SERIF, fontSize: 18, color: T.ink }}>{v}</div>
                <div style={{ fontSize: FZ.tiny, color: T.inkMid }}>{d}</div>
              </div>))}
        </div>
      )}
      <div style={{ display: "grid", gridTemplateColumns: "minmax(0,1.05fr) minmax(0,1fr)", minHeight: 0, overflow: "hidden" }}>
        <div style={{ padding: SPACE.lg, borderRight: `1px solid ${T.line}` }}>
          {raster ? <RealmMap raster={raster} owner={owner} focus={focus} fill={fill} marks={marks} rows={rows} towns={towns}
            hover={hover} onHover={setHover} onPick={pickProvince} /> : <div style={{ color: T.inkDim }}>Generate provinces to see the map.</div>}
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 6, alignItems: "center", fontSize: FZ.small, color: T.inkDim }} data-no-drag>
            <span>FILL</span>
            {(["realm", "culture", "order"] as const).map((f) => <Chip key={f} on={fill === f} onClick={() => setFill(f)}>{f === "realm" ? "Realms" : f === "culture" ? "Cultures" : "Public order"}</Chip>)}
            <span style={{ marginLeft: 6 }}>SHOW</span>
            {(["borders", "goods", "cities", "trade"] as const).map((k) => <Chip key={k} on={marks[k]} onClick={() => setMarks((m) => ({ ...m, [k]: !m[k] }))}>
              {k === "borders" ? "Borders" : k === "goods" ? "Goods exports" : k === "cities" ? "Cities" : "Country → market"}</Chip>)}
          </div>
          {hover !== null && <div style={{ marginTop: 4, fontSize: FZ.small, color: T.inkMid }}>{rows.get(hover)?.name}</div>}
        </div>
        <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
          <Tabs tabs={tabs} active={curTab} onSelect={(k) => { setTab(k); if (k === "overview") { setProv(null); setCity(null); } if (k === "province") setCity(null); }} />
          <Ledger style={{ margin: SPACE.md, overflow: "auto", maxHeight: "60vh" }}>
            {curTab === "overview" && ov && <OverviewPane ov={ov} />}
            {curTab === "overview" && !ov && <div>{active ? (states.length ? "Loading…" : "No realm yet.") : "Pick a realm in Edit realms to edit it."}</div>}
            {curTab === "overview" && active && watch && <WatchPane watch={watch} rows={rows} />}
            {curTab === "gov" && ov && <GovernmentPane ov={ov} family={family} />}
            {curTab === "provinces" && ov && <ProvincesPane ov={ov} hover={hover} onHover={setHover} onOpen={(p) => { setProv(p); setTab("province"); }} />}
            {curTab === "goods" && ov && <GoodsPane ov={ov} />}
            {curTab === "province" && prow && <ProvincePane row={prow} goods={goods} />}
            {curTab === "pgov" && prow && <ProvinceGovPane row={prow} />}
            {curTab === "city" && (gov ? <CouncilChamber brief={gov} compact /> : <div>Loading the city council…</div>)}
            {curTab === "edit" && !active && (
              <EditPane start={start} realm={realm} setRealm={setRealm} mode={mode} assign={assign} palFor={palFor} status={status}
                onAssign={() => setAssign((a) => !a)} onPal={setPalFor}
                onChange={(next) => { setStart(next); setStatus("unsaved changes"); }}
                onMode={(m) => { setMode(m); setRealmStartMode(m).catch(() => {}); }}
                onSave={() => setStartRealms(start).then(() => setStatus("saved")).catch((e) => setStatus(String(e)))}
                onRegenerate={() => regenerateStartRealms().then((r) => { setStart(r); setRealm(0); setStatus("new realms rolled and saved"); }).catch((e) => setStatus(String(e)))} />
            )}
          </Ledger>
        </div>
      </div>
    </Panel>
  );
}

/** Which province a world cell is in (the raster lookup), or null at sea. */
function centroidOf(r: ProvinceRaster, wx: number, wy: number): number | null {
  const rx = Math.floor((wx / r.gridW) * r.w), ry = Math.floor((wy / r.gridH) * r.h);
  if (rx < 0 || ry < 0 || rx >= r.w || ry >= r.h) return null;
  const id = r.data[ry * r.w + rx];
  return id === 0xffffffff ? null : id;
}

const H3 = ({ children }: { children: React.ReactNode }) => (
  <div style={{ fontFamily: SERIF, color: "#7d1f1f", letterSpacing: 2, textTransform: "uppercase", fontSize: FZ.base, margin: "8px 0 4px" }}>{children}</div>
);
const KV = ({ rows }: { rows: [string, React.ReactNode][] }) => (
  <div style={{ display: "grid", gridTemplateColumns: "max-content minmax(0,1fr)", gap: "3px 12px", fontSize: FZ.base }}>
    {rows.flatMap(([k, v]) => [<span key={k + "k"} style={{ color: "#6b5b3d" }}>{k}</span>, <span key={k + "v"}>{v}</span>])}
  </div>
);

function OverviewPane({ ov }: { ov: RealmOverview }) {
  return (<>
    <H3>Cultural composition</H3>
    <div style={{ display: "flex", height: 16, border: "1px solid #c9b993", borderRadius: 2, overflow: "hidden" }}>
      {ov.cultures.map((c) => <i key={c.good} title={`${c.good} ${Math.round(c.amount * 100)}%`} style={{ width: `${c.amount * 100}%`, background: `rgb(${hashColor(c.good).join(",")})` }} />)}
    </div>
    <div style={{ display: "flex", gap: 12, flexWrap: "wrap", marginTop: 4, fontSize: FZ.small }}>
      {ov.cultures.map((c) => <span key={c.good}><span style={{ display: "inline-block", width: 9, height: 9, background: `rgb(${hashColor(c.good).join(",")})`, marginRight: 4 }} />{c.good} {Math.round(c.amount * 100)}%</span>)}
    </div>
    <H3>Chief exports</H3>
    <div style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
      {ov.goods.filter((g) => g.amount > 0).slice(0, 5).map((g) => <span key={g.good} style={{ display: "flex", gap: 6, alignItems: "center" }}><GoodIcon name={g.good} color={goodColor(g.good)} size={24} /><span><b style={{ fontFamily: SERIF }}>{fmt(g.amount)}</b><br />{label(g.good)}</span></span>)}
    </div>
    <H3>Standing</H3>
    <KV rows={[["Ruler", ov.ruler || "—"], ["Heir", ov.heir || "—"], ["Legitimacy", <Meter v={ov.legitimacy} />], ["Cohesion", <Meter v={ov.cohesion} />]]} />
    {ov.recent.length > 0 && <><H3>Recent</H3>{ov.recent.map((r, k) => <div key={k} style={{ fontSize: FZ.small, color: "#4d4030" }}>{r}</div>)}</>}
  </>);
}

function GovernmentPane({ ov, family }: { ov: RealmOverview; family: PersonBrief[] | undefined }) {
  const L = ov.laws;
  return (<>
    <H3>The crown</H3>
    <KV rows={[["Ruler", ov.ruler || "a council"], ["Heir", ov.heir || "—"], ["Regent", ov.regent || "none"], ["Legitimacy", <Meter v={ov.legitimacy} />], ["Cohesion", <Meter v={ov.cohesion} />]]} />
    <H3>Royal council</H3>
    {ov.council.length === 0 && <div style={{ fontSize: FZ.small }}>No council seated yet.</div>}
    {ov.council.map((m) => (
      <div key={m.office} style={{ display: "grid", gridTemplateColumns: "100px minmax(0,1fr) 150px", gap: 8, padding: "4px 0", borderBottom: "1px solid #e3d8bf", fontSize: FZ.base }}>
        <span style={{ fontFamily: SERIF, color: "#7d1f1f" }}>{m.office}</span>
        <span><b>{m.name}</b><br /><span style={{ color: "#6b5b3d", fontSize: FZ.small }}>{m.from}</span></span>
        <span style={{ color: "#6b5b3d", fontSize: FZ.small, textAlign: "right" }}>{m.stance}</span>
      </div>))}
    <H3>Laws of the realm</H3>
    <KV rows={[["Crown tax", `${LEVEL[L.tax]} — more treasury, less loyalty and growth`], ["Levies", `${LEVEL[L.levy]} — men each province must raise`],
      ["Succession", L.succession], ["Trade policy", L.trade], ["Town privileges", `${LEVEL[L.town_fee]} fee — cities keep their own councils and pay this`], ["Minority peoples", L.tolerance]]} />
    {family && family.length > 0 && <><H3>The royal family</H3><GenealogyList family={family} /></>}
  </>);
}

function ProvincesPane({ ov, hover, onHover, onOpen }: { ov: RealmOverview; hover: number | null; onHover: (p: number | null) => void; onOpen: (p: number) => void }) {
  const [sort, setSort] = useState<"name" | "pop" | "tax" | "unrest">("pop");
  const rows = ov.provinces.slice().sort((a, b) => sort === "name" ? a.name.localeCompare(b.name) : sort === "pop" ? (b.rural + b.urban) - (a.rural + a.urban) : sort === "tax" ? b.tax - a.tax : b.unrest - a.unrest);
  const maxp = Math.max(1, ...ov.provinces.map((p) => p.rural + p.urban));
  const th = (k: typeof sort, t: string) => <th onClick={() => setSort(k)} style={{ cursor: "pointer", textAlign: "left", fontSize: FZ.micro, letterSpacing: 1, color: "#6b5b3d", textTransform: "uppercase", padding: 4 }}>{t}</th>;
  return (
    <table style={{ width: "100%", borderCollapse: "collapse", fontVariantNumeric: "tabular-nums", fontSize: FZ.base }}>
      <thead><tr>{th("name", "Province")}{th("pop", "Population")}{th("tax", "Tax / yr")}{th("unrest", "Order")}<th style={{ fontSize: FZ.micro, color: "#6b5b3d", textAlign: "left" }}>MAIN EXPORTS</th></tr></thead>
      <tbody>{rows.map((p) => (
        <tr key={p.id} onMouseEnter={() => onHover(p.id)} onMouseLeave={() => onHover(null)} onClick={() => onOpen(p.id)}
          style={{ cursor: "pointer", background: hover === p.id ? "#ece3cf" : undefined, borderBottom: "1px solid #e3d8bf" }}>
          <td style={{ padding: 4 }}>{p.capital ? "★ " : ""}{p.name}</td>
          <td>{fmt(p.rural + p.urban)} <span title={`towns ${fmt(p.urban)} · countryside ${fmt(p.rural)}`} style={{ display: "inline-block", width: 60, height: 7, background: `linear-gradient(90deg,#7d5f2e ${(p.urban / Math.max(1, p.rural + p.urban)) * 100}%,#c9b993 0)`, opacity: 0.4 + 0.6 * (p.rural + p.urban) / maxp }} /></td>
          <td>{fmt(p.tax)}</td>
          <td style={{ color: p.unrest > 0.4 ? "#a3262a" : "#2f7a4f" }}>{ORDER(p.unrest)}</td>
          <td>{p.exports.slice(0, 3).map((e) => <GoodIcon key={e.good} name={e.good} color={goodColor(e.good)} size={20} />)}</td>
        </tr>))}</tbody>
    </table>
  );
}

function GoodsPane({ ov }: { ov: RealmOverview }) {
  const mx = Math.max(1, ...ov.goods.map((g) => Math.abs(g.amount)));
  return (<>
    <H3>Goods surplus and deficit</H3>
    <div style={{ fontSize: FZ.small, color: "#6b5b3d", marginBottom: 6 }}>What the realm sends out minus what it brings in, per year. Trade between its own provinces cancels out.</div>
    {ov.goods.map((g) => (
      <div key={g.good} style={{ display: "grid", gridTemplateColumns: "24px 120px minmax(0,1fr) 60px", gap: 6, alignItems: "center", padding: "2px 0", borderBottom: "1px solid #e3d8bf", fontSize: FZ.base }}>
        <GoodIcon name={g.good} color={goodColor(g.good)} size={20} /><span>{label(g.good)}</span>
        <span style={{ position: "relative", height: 9 }}><span style={{ position: "absolute", left: "50%", top: -2, bottom: -2, width: 1, background: "#6b5b3d" }} />
          <i style={{ position: "absolute", top: 0, height: 9, ...(g.amount >= 0 ? { left: "50%", width: `${g.amount / mx * 50}%`, background: "#2f7a4f" } : { right: "50%", width: `${-g.amount / mx * 50}%`, background: "#a3262a" }) }} /></span>
        <span style={{ textAlign: "right", color: g.amount >= 0 ? "#2f7a4f" : "#a3262a" }}>{g.amount >= 0 ? "+" : "−"}{fmt(Math.abs(g.amount))}</span>
      </div>))}
  </>);
}

function ProvincePane({ row, goods }: { row: RealmProvinceRow; goods: ProvinceGoodExploit[] }) {
  // The campaign's goods are the year-0 economy's goods, in the same order.
  const names = useWorldStore((s) => s.economy?.goods ?? []);
  return (<>
    <div style={{ fontFamily: SERIF, fontSize: 20, color: "#7d1f1f" }}>{row.name}</div>
    <div style={{ fontSize: FZ.small, color: "#6b5b3d" }}>{row.culture} people · click its town on the map to open its council</div>
    <H3>Main exports</H3>
    <div style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>{row.exports.slice(0, 3).map((e) => <span key={e.good} style={{ display: "flex", gap: 6, alignItems: "center" }}><GoodIcon name={e.good} color={goodColor(e.good)} size={24} /><span><b style={{ fontFamily: SERIF }}>{fmt(e.amount)}</b><br />{label(e.good)} / yr</span></span>)}</div>
    <H3>People</H3>
    <KV rows={[["Countryside", `${fmt(row.rural)} — eat their own grain, sell the surplus in town, buy salt, tools and cloth back`], ["Towns", fmt(row.urban)], ["Order", ORDER(row.unrest)], ["Tax", `${fmt(row.tax)} a year`]]} />
    <H3>Goods of the land</H3>
    {goods.length === 0 && <div style={{ fontSize: FZ.small }}>Nothing worked here yet.</div>}
    {goods.map((g) => {
      const name = names[g.good] ?? String(g.good);
      const state = g.actual > 0 ? "worked" : "known, untapped";
      return (
        <div key={g.good} style={{ display: "grid", gridTemplateColumns: "24px 120px minmax(0,1fr)", gap: 6, alignItems: "center", fontSize: FZ.base, opacity: g.actual > 0 ? 1 : 0.55 }}>
          <GoodIcon name={name} color={goodColor(name)} size={20} /><span>{label(name)}</span>
          <span style={{ color: "#6b5b3d" }}>{state}{g.actual > 0 ? ` · ${Math.round(g.exploitation * 100)}% of what the land can give` : ""}</span>
        </div>);
    })}
    <div style={{ fontSize: FZ.small, color: "#6b5b3d", marginTop: 6 }}>Only goods of the land (grown, raised, gathered, caught, mined). Goods made in workshops never appear here.</div>
  </>);
}

function ProvinceGovPane({ row }: { row: RealmProvinceRow }) {
  const g = row.governor;
  if (!g) return <div>No governor: the province governs itself.</div>;
  return (<>
    <H3>Governor</H3>
    <KV rows={[["Governor", <><b>{g.name}</b> · {g.years} years in office</>], ["Loyalty", <Meter v={g.loyalty} />], ["Honesty", <Meter v={g.honesty} />]]} />
    <H3>Accounts</H3>
    <KV rows={[["Tax raised", `${fmt(row.tax)} a year`], ["Kept by the governor", g.skim > 0 ? `${Math.round(g.skim * 100)}% (${fmt(row.tax * g.skim)} a year)` : "nothing — an honest steward"], ["Sent to the crown", `${fmt(row.tax * (1 - g.skim))} a year`]]} />
    <H3>Risk of breaking away</H3>
    <KV rows={[["Risk", <span style={{ color: g.breakaway_risk > 0.5 ? "#a3262a" : g.breakaway_risk > 0.25 ? "#a8741a" : "#2f7a4f" }}>{Math.round(g.breakaway_risk * 100)}%</span>], ["Why", `loyalty ${Math.round(g.loyalty * 100)}% · order ${ORDER(row.unrest)}`]]} />
    <div style={{ fontSize: FZ.small, color: "#6b5b3d", marginTop: 6 }}>The crown replaces a governor who fails it; a disloyal one over restless people may break away and found his own realm.</div>
  </>);
}

function EditPane({ start, realm, setRealm, mode, assign, palFor, status, onAssign, onPal, onChange, onMode, onSave, onRegenerate }: {
  start: StartRealm[]; realm: number; setRealm: (k: number) => void; mode: string; assign: boolean; palFor: number | null; status: string;
  onAssign: () => void; onPal: (k: number | null) => void; onChange: (s: StartRealm[]) => void; onMode: (m: string) => void; onSave: () => void; onRegenerate: () => void;
}) {
  const set = (k: number, patch: Partial<StartRealm>) => onChange(start.map((r, j) => j === k ? { ...r, ...patch } : r));
  const btn: React.CSSProperties = { background: "#e9dfc8", border: "1px solid #c9b993", borderRadius: 3, padding: "3px 9px", cursor: "pointer", fontSize: FZ.small };
  return (<>
    <div style={{ fontSize: FZ.small, color: "#6b5b3d" }}>The world's countries, generated with its provinces in random sizes. Recolour, rename, assign provinces (click them on the map) or found a new realm.</div>
    <H3>When a campaign starts</H3>
    {[["continue", "Continue as built — these realms"], ["random", "Randomise — new realms of random sizes"], ["zero", "Start from nothing — no realms; the whole history is simulated"]].map(([k, l]) => (
      <label key={k} style={{ display: "block", fontSize: FZ.base }}><input type="radio" name="realmstart" checked={mode === k} onChange={() => onMode(k)} /> {l}</label>))}
    <div style={{ display: "flex", gap: 6, flexWrap: "wrap", margin: "8px 0" }}>
      <button style={btn} onClick={onSave}>Save realms</button>
      <button style={btn} onClick={onRegenerate}>Regenerate starting realms</button>
      <button style={{ ...btn, background: assign ? "#7d5f2e" : btn.background, color: assign ? "#fff" : undefined }} onClick={onAssign}>{assign ? "Stop assigning" : "Assign provinces"}</button>
      <button style={btn} onClick={() => { onChange([...start, { name: "New Realm", color: PALETTE[start.length % PALETTE.length], capital: -1, provinces: [] }]); setRealm(start.length); }}>+ New realm</button>
      <span style={{ fontSize: FZ.small, color: "#6b5b3d", alignSelf: "center" }}>{status}</span>
    </div>
    {start.map((r, k) => (
      <div key={k}>
        <div style={{ display: "flex", gap: 8, alignItems: "center", padding: "3px 0", borderBottom: "1px solid #e3d8bf", background: k === realm ? "#ece3cf" : undefined }}>
          <span onClick={() => onPal(palFor === k ? null : k)} title="Change colour" style={{ width: 18, height: 18, borderRadius: 3, cursor: "pointer", background: `rgb(${r.color.join(",")})`, border: "1px solid #0006" }} />
          <input value={r.name} onChange={(e) => set(k, { name: e.target.value })} style={{ flex: 1, minWidth: 0, border: "1px solid #c9b993", background: "#fbf7ee", padding: "1px 5px" }} />
          <span style={{ fontSize: FZ.small, color: "#6b5b3d" }}>{r.provinces.length} prov.</span>
          <button style={btn} onClick={() => setRealm(k)}>{k === realm ? "selected" : "select"}</button>
        </div>
        {palFor === k && (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(8, 22px)", gap: 4, padding: 6, margin: "4px 0 4px 26px" }}>
            {PALETTE.map((c, j) => <span key={j} onClick={() => { set(k, { color: c }); onPal(null); }} style={{ width: 22, height: 22, borderRadius: 3, cursor: "pointer", background: `rgb(${c.join(",")})` }} />)}
            <input type="color" style={{ gridColumn: "span 8", width: "100%" }} value={`#${r.color.map((v) => v.toString(16).padStart(2, "0")).join("")}`}
              onChange={(e) => { const h = e.target.value; set(k, { color: [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) as [number, number, number] }); }} />
          </div>)}
      </div>))}
  </>);
}

function GenealogyList({ family }: { family: PersonBrief[] | undefined }) {
  if (family === undefined) {
    return <div style={{ color: T.inkFaint, fontSize: FZ.tiny, fontStyle: "italic" }}>loading…</div>;
  }
  if (family.length === 0) {
    return <div style={{ color: T.inkFaint, fontSize: FZ.tiny, fontStyle: "italic" }}>no record yet</div>;
  }
  const byIdx = new Map(family.map((p) => [p.idx, p]));
  return (
    <div style={{ marginBottom: 4 }}>
      {family.map((p) => {
        const relation = p.father >= 0 ? `child of ${byIdx.get(p.father)?.name ?? "?"}`
          : p.spouse >= 0 && !p.is_ruler ? `spouse of ${byIdx.get(p.spouse)?.name ?? "?"}`
          : "founder";
        return (
          <div key={p.idx} style={{ display: "flex", alignItems: "baseline", gap: 6, padding: "1px 0" }}>
            <span style={{ color: T.inkFaint, fontSize: FZ.micro, width: 12 }}>{p.female ? "♀" : "♂"}</span>
            <span style={{ flex: 1, minWidth: 0, color: p.alive ? T.inkMid : T.inkFaint, fontSize: FZ.small,
              overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {p.name}{p.epithet && ` "${p.epithet}"`}
            </span>
            {p.is_ruler && <Badge tone="gold">ruler</Badge>}
            {p.is_regent && <Badge tone="warn">regent</Badge>}
            <span style={{ color: T.inkFaint, fontSize: FZ.micro, width: 78, textAlign: "right" }}>
              {p.alive ? `age ${p.age}` : `d. age ${p.age}`} · {relation}
            </span>
          </div>
        );
      })}
    </div>
  );
}

/** Free provinces closest to a crown: the strongest private house in each, and why it
 *  cannot proclaim yet (`campaign_realm_watch`). */
function WatchPane({ watch, rows }: { watch: RealmWatch; rows: Map<number, RealmProvinceRow> }) {
  return (<>
    <H3>Rising powers</H3>
    <div style={{ fontSize: FZ.small, color: watch.eligible_count > 0 ? "#7d5f2e" : "#6b5b3d" }}>{watch.summary}</div>
    {watch.entries.map((e) => (
      <div key={e.province_id} style={{ display: "flex", gap: 6, alignItems: "baseline", fontSize: FZ.small, padding: "2px 0", borderBottom: "1px solid #e3d8bf" }}>
        <span style={{ flex: 1, minWidth: 0 }}>
          <b>{rows.get(e.province_id)?.name ?? `Province ${e.province_id}`}</b> — {e.house_name} {(e.share * 100).toFixed(0)}%
          {!e.eligible && <div style={{ color: "#6b5b3d", fontSize: FZ.tiny }}>{e.reason}</div>}
        </span>
        {e.eligible && <Badge tone="gold">eligible</Badge>}
      </div>))}
  </>);
}
