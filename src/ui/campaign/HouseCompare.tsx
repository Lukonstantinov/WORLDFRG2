/** House Comparison window — search two houses (or guilds), see their ruler
 *  figures, a full side-by-side stat comparison (each row highlighting whichever
 *  side leads), their trading strategy (specialties/top goods/monopolies/
 *  archetype), and a minimal operations map plotting where each actually holds
 *  seats, offices and controlled settlements — so a rivalry reads at a glance
 *  instead of needing two separate dossiers open side by side. */
import { useEffect, useMemo, useRef, useState } from "react";
import { CoatOfArms } from "@ui/heraldry/CoatOfArms";
import { drawFigure, resolveKit } from "@ui/campaign/cultureDress";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { GOOD_DEFS, clarifyGemLabel } from "@goods";
import { useWorldStore } from "@state/worldStore";
import type { HouseBrief } from "@types";

const GOOD_ICON = new Map(GOOD_DEFS.map((g) => [g.name, g.emoji]));
const goodIcon = (name: string) => GOOD_ICON.get(name) ?? "\u{1F4E6}"; // 📦 fallback

const fmt = (v: number | undefined) => {
  const n = v ?? 0;
  return n >= 1000 ? `${(n / 1000).toFixed(1)}k` : n.toFixed(n >= 10 ? 0 : 2);
};

function HouseSearchField({ houses, value, onPick, placeholder }:
  { houses: HouseBrief[]; value: HouseBrief | null; onPick: (h: HouseBrief) => void; placeholder: string }) {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const matches = useMemo(() => {
    if (!query.trim()) return houses.filter((h) => !h.defunct).slice(0, 8);
    const q = query.toLowerCase();
    return houses.filter((h) => h.name.toLowerCase().includes(q)).slice(0, 8);
  }, [houses, query]);
  return (
    <div style={{ position: "relative", flex: 1 }}>
      <input
        data-no-drag
        value={open ? query : (value?.name ?? "")}
        placeholder={placeholder}
        onFocus={() => { setOpen(true); setQuery(""); }}
        onChange={(e) => setQuery(e.target.value)}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
        style={{
          width: "100%", background: "#0a1119", border: "1px solid #24364e", borderRadius: 4,
          color: "#e8dcc0", fontSize: 12, padding: "6px 9px", fontFamily: "inherit",
        }}
      />
      {open && matches.length > 0 && (
        <div style={{
          position: "absolute", top: "100%", left: 0, right: 0, zIndex: 60, marginTop: 2,
          background: "#0f1822", border: "1px solid #24364e", borderRadius: 4,
          maxHeight: 220, overflowY: "auto", boxShadow: "0 8px 20px rgba(0,0,0,0.5)",
        }}>
          {matches.map((h) => (
            <div
              key={h.idx}
              onMouseDown={() => { onPick(h); setOpen(false); }}
              style={{ display: "flex", alignItems: "center", gap: 6, padding: "6px 9px", cursor: "pointer", fontSize: 11 }}
              onMouseEnter={(e) => (e.currentTarget.style.background = "#16202c")}
              onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
            >
              <span style={{ width: 8, height: 8, borderRadius: 2, background: h.color ?? "#888", flex: "0 0 auto" }} />
              <span style={{ color: h.defunct ? "#5a6f82" : "#cfe0f4" }}>{h.name}{h.defunct ? " (defunct)" : ""}</span>
              <span style={{ flex: 1 }} />
              <span style={{ color: "#6a86a6", fontSize: 9.5 }}>{fmt(h.wealth)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/** One comparison row: label + both values, the leading side highlighted. Pass
 *  `lowerIsBetter` for a metric where less is the stronger position (none currently
 *  used, kept for future rows like debt). */
function Row({ label, a, b, format, lowerIsBetter }:
  { label: string; a: number; b: number; format?: (v: number) => string; lowerIsBetter?: boolean }) {
  const f = format ?? ((v: number) => v.toFixed(2));
  const aWins = lowerIsBetter ? a < b : a > b;
  const bWins = lowerIsBetter ? b < a : b > a;
  const tie = a === b;
  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 110px 1fr", alignItems: "center", padding: "4px 0", borderBottom: "1px solid #16202c" }}>
      <span style={{ textAlign: "right", fontSize: 11, color: aWins && !tie ? "#7fd0a0" : "#cfe0f4", fontWeight: aWins && !tie ? 700 : 400, fontVariantNumeric: "tabular-nums" }}>
        {f(a)}
      </span>
      <span style={{ textAlign: "center", fontSize: 9.5, color: "#6a86a6", textTransform: "uppercase", letterSpacing: 0.3 }}>{label}</span>
      <span style={{ fontSize: 11, color: bWins && !tie ? "#7fd0a0" : "#cfe0f4", fontWeight: bWins && !tie ? 700 : 400, fontVariantNumeric: "tabular-nums" }}>
        {f(b)}
      </span>
    </div>
  );
}

/** Minimal operations map: every point either house's own brief already carries
 *  (seat, offices, controlled settlements), normalised to the bounding box of the
 *  TWO houses together (not the whole world) so their actual working region fills
 *  the frame instead of being a speck on a full atlas. */
function OperationsMap({ a, b }: { a: HouseBrief; b: HouseBrief }) {
  type Pt = { x: number; y: number; kind: string; color: string; label: string; owner: 0 | 1 };
  const points: Pt[] = [];
  const add = (p: [number, number] | undefined, kind: string, color: string, label: string, owner: 0 | 1) => {
    if (p && (p[0] !== 0 || p[1] !== 0)) points.push({ x: p[0], y: p[1], kind, color, label, owner });
  };
  const cA = a.color ?? "#b32d2d", cB = b.color ?? "#2a5fa0";
  add(a.seat, "seat", cA, a.name, 0);
  add(b.seat, "seat", cB, b.name, 1);
  (a.offices ?? []).forEach(([nm, p]) => add(p, "office", cA, nm, 0));
  (b.offices ?? []).forEach(([nm, p]) => add(p, "office", cB, nm, 1));
  (a.controls ?? []).forEach((p) => add(p, "controls", cA, "", 0));
  (b.controls ?? []).forEach((p) => add(p, "controls", cB, "", 1));
  // Trade partners fill out the working region — the reason the map read empty
  // before was that guilds/young houses hold few offices, so only the seat plotted.
  // Cap each side so a very busy house doesn't bury the frame in dots.
  (a.partners ?? []).slice(0, 20).forEach((p) => add(p, "partner", cA, "", 0));
  (b.partners ?? []).slice(0, 20).forEach((p) => add(p, "partner", cB, "", 1));

  // The world basemap: a land/sea silhouette from the province raster (already loaded
  // for the province overlay). Land = a real province id, sea = the NO_PROVINCE
  // sentinel. Rendered once to a data URL and drawn behind the operations, so the
  // cities sit on the actual world map rather than a bare bounding box.
  const raster = useWorldStore((s) => s.provinceRaster);
  const base = useMemo(() => {
    if (!raster || raster.w === 0) return null;
    const { data, w, h } = raster;
    const cv = document.createElement("canvas");
    cv.width = w; cv.height = h;
    const ctx = cv.getContext("2d");
    if (!ctx) return null;
    const img = ctx.createImageData(w, h);
    for (let i = 0; i < w * h; i++) {
      const land = data[i] !== 0xffffffff;
      const o = i * 4;
      img.data[o] = land ? 32 : 8; img.data[o + 1] = land ? 44 : 12;
      img.data[o + 2] = land ? 36 : 20; img.data[o + 3] = 255;
    }
    ctx.putImageData(img, 0, 0);
    return { url: cv.toDataURL(), w, h, gridW: raster.gridW, gridH: raster.gridH };
  }, [raster]);

  if (points.length === 0) {
    return <div style={{ color: "#56708e", fontSize: 10, textAlign: "center", padding: 20 }}>No located operations to plot.</div>;
  }

  const seats = points.filter((p) => p.kind === "seat");
  const seatOf = (owner: 0 | 1) => seats.find((s) => s.owner === owner);
  // Connectors from each house's seat to everything it works — its trade network.
  const spokes = points.filter((p) => p.kind !== "seat").map((p) => ({ p, s: seatOf(p.owner) }))
    .filter((e): e is { p: Pt; s: Pt } => !!e.s);

  // ── World-basemap layout (preferred): plot every operation at its TRUE world
  //    position over the land silhouette, with connectors bowed into arcs so they
  //    read as sea-lanes/roads rather than a straight ruler line. ──
  if (base) {
    const VBW = base.w, VBH = base.h;
    const lx = (x: number) => (x / base.gridW) * VBW;
    const ly = (y: number) => (y / base.gridH) * VBH;
    const rFor = (kind: string) => (kind === "seat" ? VBW * 0.012 : kind === "office" ? VBW * 0.008 : kind === "controls" ? VBW * 0.008 : VBW * 0.005);
    // Wrap-aware arc: a quadratic bow perpendicular to the seat→point chord.
    const arc = (s: Pt, p: Pt) => {
      const x1 = lx(s.x), y1 = ly(s.y), x2 = lx(p.x), y2 = ly(p.y);
      const mx = (x1 + x2) / 2, my = (y1 + y2) / 2;
      const dx = x2 - x1, dy = y2 - y1;
      const bow = 0.14;
      return `M${x1.toFixed(1)} ${y1.toFixed(1)} Q${(mx - dy * bow).toFixed(1)} ${(my + dx * bow).toFixed(1)} ${x2.toFixed(1)} ${y2.toFixed(1)}`;
    };
    return (
      <svg viewBox={`0 0 ${VBW} ${VBH}`} width="100%" height="240" preserveAspectRatio="xMidYMid meet"
        style={{ background: "#06090d", borderRadius: 6, border: "1px solid #1c2b3c", display: "block" }}>
        <image href={base.url} x={0} y={0} width={VBW} height={VBH} preserveAspectRatio="none" style={{ imageRendering: "pixelated" }} />
        {spokes.map((e, i) => (
          <path key={"l" + i} d={arc(e.s, e.p)} fill="none" stroke={e.p.color} strokeWidth={VBW * 0.0018} opacity={0.5} />
        ))}
        {points.map((p, i) => (
          <g key={i}>
            <circle cx={lx(p.x)} cy={ly(p.y)} r={rFor(p.kind)} fill={p.color}
              opacity={p.kind === "partner" ? 0.55 : p.kind === "controls" ? 0.75 : 0.95}
              stroke={p.kind === "seat" ? "#e8dcc0" : "#06090d"} strokeWidth={p.kind === "seat" ? VBW * 0.003 : VBW * 0.001} />
            {p.kind === "seat" && (
              <text x={lx(p.x) + rFor("seat") + 2} y={ly(p.y) + 3} fontSize={VBW * 0.016} fill="#eaf2fb"
                stroke="#06090d" strokeWidth={VBW * 0.004} paintOrder="stroke">{p.label}</text>
            )}
          </g>
        ))}
      </svg>
    );
  }

  // ── Fallback (no province raster loaded): the old two-house bounding-box scatter. ──
  const xs = points.map((p) => p.x), ys = points.map((p) => p.y);
  const pad = Math.max(4, (Math.max(...xs) - Math.min(...xs)) * 0.12, (Math.max(...ys) - Math.min(...ys)) * 0.12);
  const minX = Math.min(...xs) - pad, maxX = Math.max(...xs) + pad;
  const minY = Math.min(...ys) - pad, maxY = Math.max(...ys) + pad;
  const w = Math.max(1, maxX - minX), h = Math.max(1, maxY - minY);
  const VB = 260;
  const px = (x: number) => ((x - minX) / w) * VB;
  const py = (y: number) => ((y - minY) / h) * VB;
  const rFor = (kind: string) => (kind === "seat" ? 6 : kind === "office" ? 4 : kind === "controls" ? 4 : 2.4);
  return (
    <svg viewBox={`0 0 ${VB} ${VB}`} width="100%" height="220" style={{ background: "#0a1119", borderRadius: 6, border: "1px solid #1c2b3c" }}>
      <rect x="1" y="1" width={VB - 2} height={VB - 2} fill="none" stroke="#16202c" />
      {spokes.map((e, i) => (
        <line key={"l" + i} x1={px(e.s.x)} y1={py(e.s.y)} x2={px(e.p.x)} y2={py(e.p.y)}
          stroke={e.p.color} strokeWidth={0.6} opacity={0.28} />
      ))}
      {points.map((p, i) => (
        <g key={i}>
          <circle cx={px(p.x)} cy={py(p.y)} r={rFor(p.kind)} fill={p.color}
            opacity={p.kind === "partner" ? 0.4 : p.kind === "controls" ? 0.6 : 0.9}
            stroke={p.kind === "seat" ? "#e8dcc0" : "none"} strokeWidth={p.kind === "seat" ? 1.4 : 0} />
          {p.kind === "seat" && (
            <text x={px(p.x) + 8} y={py(p.y) + 3} fontSize="9" fill="#cfe0f4">{p.label}</text>
          )}
        </g>
      ))}
    </svg>
  );
}

/** A house's trade ledger — the goods it moves the most (by shipped volume) with the
 *  amount seen, its single most profitable trade highlighted, each with its icon. */
function GoodsLedger({ h }: { h: HouseBrief }) {
  const rows = h.goods_ledger ?? [];
  if (rows.length === 0) {
    return <div style={{ color: "#56708e", fontSize: 10 }}>No trade recorded yet.</div>;
  }
  const bestProfit = rows.reduce((m, r) => (r.profit > m.profit ? r : m), rows[0]);
  const maxVol = Math.max(...rows.map((r) => r.volume), 1);
  return (
    <div>
      {rows.map((r, i) => {
        const top = r.good === bestProfit.good && bestProfit.profit > 0;
        return (
          <div key={i} style={{ display: "flex", alignItems: "center", gap: 5, padding: "1px 0", fontSize: 10.5 }}>
            <span style={{ width: 16, textAlign: "center" }}>{goodIcon(r.good)}</span>
            <span style={{ width: 88, color: top ? "#e6c878" : "#cfe0f4", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {clarifyGemLabel(r.good, h.gem_variety)}{top ? " ★" : ""}
            </span>
            <div style={{ flex: 1, height: 5, background: "#0f1822", borderRadius: 3, overflow: "hidden" }}>
              <div style={{ width: `${Math.round((r.volume / maxVol) * 100)}%`, height: "100%", background: h.color ?? "#6a86a6", opacity: 0.75 }} />
            </div>
            <span style={{ width: 46, textAlign: "right", color: "#9ab0c8", fontVariantNumeric: "tabular-nums" }}>{fmt(r.volume)}</span>
            <span style={{ width: 52, textAlign: "right", color: r.profit >= 0 ? "#7fd0a0" : "#e0a09a", fontVariantNumeric: "tabular-nums" }}
              title="cumulative profit on this good">{r.profit >= 0 ? "+" : ""}{fmt(r.profit)}</span>
          </div>
        );
      })}
      {bestProfit.profit > 0 && (
        <div style={{ color: "#e6c878", fontSize: 9.5, marginTop: 3 }}>
          ★ most profitable: {goodIcon(bestProfit.good)} {clarifyGemLabel(bestProfit.good, h.gem_variety)} (+{fmt(bestProfit.profit)})
        </div>
      )}
    </div>
  );
}

export function HouseCompareWindow({ houses, initialA, initialB, onClose }:
  { houses: HouseBrief[]; initialA?: HouseBrief | null; initialB?: HouseBrief | null; onClose: () => void }) {
  // Track the SELECTED houses by index, not by value — so every monthly advance
  // (which replaces the `houses` array with fresh briefs) re-reads their current
  // stats instead of freezing the numbers from when the window was opened.
  const [aIdx, setAIdx] = useState<number | null>(initialA?.idx ?? houses.find((h) => !h.defunct)?.idx ?? null);
  const [bIdx, setBIdx] = useState<number | null>(initialB?.idx ?? houses.filter((h) => !h.defunct)[1]?.idx ?? null);
  const a = useMemo(() => houses.find((h) => h.idx === aIdx) ?? null, [houses, aIdx]);
  const b = useMemo(() => houses.find((h) => h.idx === bIdx) ?? null, [houses, bIdx]);
  const setA = (h: HouseBrief) => setAIdx(h.idx ?? null);
  const setB = (h: HouseBrief) => setBIdx(h.idx ?? null);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.houses);

  // Same pixel-treated dress-plate art the Peoples panel's culture figures and
  // the House Dossier's own portrait use (`cultureDress.ts::drawFigure`) — a
  // house's figure is its seat culture's costume plate, in ceremonial dress
  // for this side-by-side view.
  const Figure = ({ h }: { h: HouseBrief }) => {
    const FIG_W = 96, FIG_SCALE = 2;
    const ref = useRef<HTMLCanvasElement | null>(null);
    useEffect(() => {
      const el = ref.current;
      if (!el) return;
      const K = resolveKit(h.kit != null && h.kit >= 0 ? h.kit : h.name, { region: "" });
      const figH = Math.round(FIG_W * 2.1);
      el.width = FIG_W * FIG_SCALE; el.height = figH * FIG_SCALE;
      el.style.width = FIG_W + "px"; el.style.height = figH + "px";
      const ctx = el.getContext("2d");
      if (!ctx) return;
      ctx.clearRect(0, 0, el.width, el.height);
      drawFigure(ctx, 0, 0, FIG_W * FIG_SCALE, K, { occasion: "ceremonial" });
    }, [h.kit, h.name]);
    return (
      <div style={{ position: "relative", width: 96, height: 218, margin: "0 auto" }}>
        <div style={{
          width: 96, height: 218, borderRadius: 6, overflow: "hidden", background: "#0a1119",
          border: `3px solid ${h.color ?? "#3a5570"}`,
          display: "flex", alignItems: "flex-end", justifyContent: "center", paddingBottom: 4,
        }}>
          <canvas ref={ref} style={{ display: "block" }} />
        </div>
        <div style={{ position: "absolute", top: -6, right: -6 }}><CoatOfArms name={h.name} size={24} guild={h.is_guild} /></div>
      </div>
    );
  };

  return (
    <div data-draggable style={{
      position: "absolute", top: 40, left: "50%", transform: "translateX(-50%)",
      width: 780, maxHeight: "88vh", overflowY: "auto",
      background: "#0c141e", border: "1px solid #24364e", borderRadius: 8,
      boxShadow: "0 8px 28px rgba(0,0,0,0.55)", zIndex: 50, padding: "12px 16px",
      ...rootStyle,
    }} onPointerDown={onPointerDown}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 10, cursor: "move" }}>
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13 }}>⚖ Compare Houses</span>
        <span style={{ flex: 1 }} />
        <span data-no-drag onClick={onClose} style={{ color: "#7090b0", cursor: "pointer", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 14 }}>
        <HouseSearchField houses={houses} value={a} onPick={setA} placeholder="Search a house or guild…" />
        <span style={{ color: "#c9a227", fontStyle: "italic", fontSize: 12 }}>vs.</span>
        <HouseSearchField houses={houses} value={b} onPick={setB} placeholder="Search a house or guild…" />
      </div>

      {!a || !b ? (
        <div style={{ color: "#56708e", fontSize: 11, textAlign: "center", padding: 30 }}>Pick two houses to compare.</div>
      ) : (
        <>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 14 }}>
            <div style={{ textAlign: "center" }}>
              <Figure h={a} />
              <div style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13, marginTop: 6 }}>{a.name}</div>
              <div style={{ color: "#9ab0c8", fontSize: 10 }}>{a.head_name} · {a.home_name} · gen {a.generation}</div>
            </div>
            <div style={{ textAlign: "center" }}>
              <Figure h={b} />
              <div style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13, marginTop: 6 }}>{b.name}</div>
              <div style={{ color: "#9ab0c8", fontSize: 10 }}>{b.head_name} · {b.home_name} · gen {b.generation}</div>
            </div>
          </div>

          <div style={{ color: "#6a86a6", fontSize: 9.5, textTransform: "uppercase", letterSpacing: 0.4, marginBottom: 4 }}>Standing</div>
          <Row label="Wealth" a={a.wealth} b={b.wealth} format={fmt} />
          <Row label="Prestige" a={a.prestige} b={b.prestige} format={(v) => v.toFixed(2)} />
          <Row label="Political power" a={a.political_power} b={b.political_power} format={(v) => v.toFixed(2)} />
          <Row label="Tier" a={5 - (a.tier || 4)} b={5 - (b.tier || 4)} format={() => (a.tier ? `${a.tier}` : "—")} />
          <Row label="Standing score" a={a.standing ?? 0} b={b.standing ?? 0} format={(v) => v.toFixed(2)} />
          <Row label="Founded" a={-(a.founded_year ?? 0)} b={-(b.founded_year ?? 0)} format={() => `${a.founded_year ?? "—"}`} />

          <div style={{ color: "#6a86a6", fontSize: 9.5, textTransform: "uppercase", letterSpacing: 0.4, margin: "10px 0 4px" }}>Trade &amp; transport</div>
          <Row label="Trade volume" a={a.volume ?? 0} b={b.volume ?? 0} format={fmt} />
          <Row label="Fleet (sea)" a={a.fleet_sea ?? 0} b={b.fleet_sea ?? 0} format={(v) => `${v}`} />
          <Row label="Fleet (river)" a={a.fleet_river ?? 0} b={b.fleet_river ?? 0} format={(v) => `${v}`} />
          <Row label="Fleet (caravan)" a={a.fleet_caravan ?? 0} b={b.fleet_caravan ?? 0} format={(v) => `${v}`} />
          <Row label="Offices" a={(a.offices ?? []).length} b={(b.offices ?? []).length} format={(v) => `${v}`} />
          <Row label="Estates" a={(a.estates ?? []).length} b={(b.estates ?? []).length} format={(v) => `${v}`} />
          <Row label="Monopolies" a={a.monopolies.length} b={b.monopolies.length} format={(v) => `${v}`} />

          <div style={{ color: "#6a86a6", fontSize: 9.5, textTransform: "uppercase", letterSpacing: 0.4, margin: "10px 0 4px" }}>Trading strategy</div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, fontSize: 10.5 }}>
            {[a, b].map((h, i) => (
              <div key={i}>
                <div style={{ color: "#9fd0c0" }}>{h.archetype_label ?? "—"}{h.archetype_perk ? ` · ${h.archetype_perk}` : ""}</div>
                <div style={{ color: "#7a90a8", marginTop: 3 }}>Specialises in: {h.specialties.length ? h.specialties.map((g) => clarifyGemLabel(g, h.gem_variety)).join(", ") : "—"}</div>
                <div style={{ color: "#7a90a8", marginTop: 2 }}>Best known for: {h.top_goods.length ? h.top_goods.slice(0, 4).map((g) => clarifyGemLabel(g, h.gem_variety)).join(", ") : "—"}</div>
                {h.monopolies.length > 0 && (
                  <div style={{ color: "#c9a227", marginTop: 2 }}>
                    Monopolies: {h.monopolies.slice(0, 4).map(([g, s]) => `${clarifyGemLabel(g, h.gem_variety)} (${Math.round(s * 100)}%)`).join(", ")}
                  </div>
                )}
              </div>
            ))}
          </div>

          <div style={{ color: "#6a86a6", fontSize: 9.5, textTransform: "uppercase", letterSpacing: 0.4, margin: "10px 0 4px" }}>
            Goods traded most <span style={{ color: "#56708e", textTransform: "none", letterSpacing: 0 }}>(amount shipped · profit · ★ most profitable)</span>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
            <GoodsLedger h={a} />
            <GoodsLedger h={b} />
          </div>

          <div style={{ color: "#6a86a6", fontSize: 9.5, textTransform: "uppercase", letterSpacing: 0.4, margin: "10px 0 4px" }}>
            Where they operate — {a.name} <span style={{ color: a.color }}>⬤</span> vs {b.name} <span style={{ color: b.color }}>⬤</span>
          </div>
          <OperationsMap a={a} b={b} />

          {a.rivals.includes(b.name) || b.rivals.includes(a.name) ? (
            <div style={{ color: "#e08a8a", fontSize: 10, marginTop: 8 }}>⚔ These houses are recorded rivals.</div>
          ) : null}
        </>
      )}
    </div>
  );
}
