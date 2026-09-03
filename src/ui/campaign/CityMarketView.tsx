import { useMemo, useState } from "react";
import type { HubDetail, HubGoodDetail, ShipmentRow } from "@types";
import { useGoodsStore } from "@state/goodsStore";
import { MarketSquare } from "@ui/campaign/MarketSquare";

/** ─────────────────────────────────────────────────────────────────────────────
 *  CITY MARKET — VARIANT C, "the quay" (`docs/TRADE_AND_MARKET_REVIEW.md` Part 3,
 *  `docs/MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` §2).
 *
 *  The organising unit is the PARTNER CITY, not the voyage and not the good: cargo
 *  lines nest under the city they came from or are going to, the way a port book is
 *  actually kept. Chosen over the by-voyage and by-good variants because it is the
 *  only one where a city you hold an office in reads differently from one you touched
 *  once — which is what makes the office and exploration mechanics legible when they
 *  land.
 *
 *  **What this deliberately does NOT show.** The design's "IN PORT, LOADING" and
 *  "READY TO SAIL" strips are absent, because a vessel is not a thing yet:
 *  `fleet_sea`/`_river`/`_caravan` are three counters on `House` with no identity or
 *  location, and `dispatch` decrements one slot per shipment regardless of quantity.
 *  Vessels in port is missing STATE, not a missing query, and faking it from in-flight
 *  legs would report cargoes as hulls. It arrives with stage 1 of the vessel plan.
 *  For the same reason a lane shows its CARGO count, never a vessel count.
 *
 *  The "by good" grouping keeps the previous book view (spread, days held, trend) —
 *  nothing was removed to make room for C.
 *  ─────────────────────────────────────────────────────────────────────────────
 *
 *  (Original 2.0 notes, still true of the "by good" grouping and the row detail:)
 *
 *  Keeps the view on its buy/sell arrivals ⇢ market ⇢ departures basis (the
 *  maintainer's call over an earlier per-good balance-table design) and rebuilds
 *  the centre column as a MERCHANT'S BOOK: what the city bought a good for, what
 *  it sold it for, the SPREAD between them, how many days of need it is sitting
 *  on, and where the price has been.
 *
 *  Four things here exist nowhere else in the app:
 *    · the SPREAD — mean sell price less mean buy price. Both sides were already
 *      carried per deal; nothing ever aggregated them.
 *    · HELD in days of need rather than units. "38 days of grain" carries meaning;
 *      "820 units" does not.
 *    · a price trend that SURVIVES closing the panel, from the persisted
 *      `TradeHist.prices` series (`HubGoodDetail.price_hist`). The old sparklines
 *      were accumulated in a React ref that reset on every hub switch.
 *    · rows sorted by what is UNUSUAL about this market rather than by production
 *      order, so the row worth reading is at the top.
 *
 *  It is deliberately a SHARED component: the settlement window's Trade tab and
 *  the floating Markets window render the same one, the same way ProvinceMiniMap
 *  is shared between the province browser and the province inspector.
 *  ───────────────────────────────────────────────────────────────────────────── */

const C = {
  ink: "#c0d0e0", inkMid: "#9ab0c8", inkDim: "#6a86a6", faint: "#56708e",
  gold: "#e0c060", good: "#7fd0a0", warn: "#e08080", buy: "#7fd0a0", sell: "#e0a080",
  line: "#1e2e42", lineSoft: "#131f2c", card: "#0b1622", raised: "#16202c",
  head: "#e8d8b0",
};

const fmt = (v: number) =>
  Math.abs(v) >= 1000 ? `${(v / 1000).toFixed(1)}k` : Math.abs(v) >= 10 ? v.toFixed(0) : v.toFixed(1);

/** A ×-world price, coloured: dear reads warm, cheap reads cool, par is neutral. */
function priceColor(xw: number): string {
  if (!isFinite(xw) || xw <= 0) return C.faint;
  if (xw > 1.3) return C.warn;
  if (xw < 0.77) return C.good;
  return C.ink;
}

/** DAYS OF COVER, as a word. `days` is stock ÷ this city's own daily need, so a
 *  good nobody here wants has no cover figure at all — printing a huge number for
 *  it would read as abundance when it actually means "not consumed". */
function coverLabel(days: number, need: number): string {
  if (need <= 1e-6) return "—";
  if (days < 1) return "<1d";
  if (days >= 999) return "999+";
  return `${Math.round(days)}d`;
}
/** Thin reads as a warning, deep reads as quiet. A middling stock says nothing —
 *  the same "quiet unless it matters" rule the verdict phrases follow. */
function coverColor(days: number, need: number): string {
  if (need <= 1e-6) return C.faint;
  if (days < 2) return C.warn;
  if (days < 6) return "#d9a441";
  if (days > 150) return C.inkDim;
  return C.inkMid;
}

/** THE VESSEL REGISTRY — hulls of the houses seated here, and what they carry.
 *
 *  Labelled a REGISTRY on purpose. A vessel is not an entity in this sim
 *  (`fleet_sea`/`_river`/`_caravan` are three counters on `House` with no
 *  identity or position), so "what is berthed here" has no answer; "whose hulls
 *  are registered to this city, and how many of those slots are out" does, and
 *  the backend computes it with the same arithmetic `dispatch` uses. A city with
 *  no seated merchant family says so, which is the ordinary case and is itself
 *  the most informative thing on the block. */
function VesselRegistry({ v }: { v: NonNullable<HubDetail["vessels"]> }) {
  // Folded state persists per browser, not per panel instance: a reader who does
  // not want the registry does not want it again on the next city either. Guarded
  // because storage throws outright in some embeddings.
  const [open, setOpen] = useState<boolean>(() => {
    try { return localStorage.getItem("wf.market.vessels") !== "0"; } catch { return true; }
  });
  const toggle = () => setOpen((o) => {
    try { localStorage.setItem("wf.market.vessels", o ? "0" : "1"); } catch { /* ignore */ }
    return !o;
  });
  const CLASS_META: Record<string, { icon: string; word: string; out: string }> = {
    sea: { icon: "⛵", word: "sea ships", out: "at sea" },
    river: { icon: "🛶", word: "river boats", out: "afloat" },
    caravan: { icon: "🐫", word: "caravans", out: "on the road" },
  };
  const classes = (v.classes ?? []).filter((c) => c.registered > 0 || c.away > 0);
  const anyHulls = classes.length > 0;
  return (
    <div style={{ border: `1px solid ${C.line}`, borderRadius: 4, padding: "4px 7px",
      margin: "4px 0", background: C.card }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 8, flexWrap: "wrap", fontSize: 9 }}>
        <span onClick={toggle} style={{ color: C.head, fontWeight: 700, cursor: "pointer", userSelect: "none" }}
          title={open ? "hide the registry" : "show the registry"}>
          <span style={{ color: C.faint, fontSize: 8 }}>{open ? "▾ " : "▸ "}</span>
          ⚓ In port &amp; on the road
        </span>
        {open && anyHulls ? classes.map((c) => {
          const m = CLASS_META[c.kind] ?? { icon: "•", word: c.kind, out: "away" };
          return (
            <span key={c.kind} style={{ color: C.inkMid }}
              title={`${c.registered} ${m.word} registered to houses seated here · ${c.away} ${m.out} · ${c.idle} idle`}>
              {m.icon} <b style={{ color: C.gold }}>{c.registered}</b>
              <span style={{ color: C.faint }}> · </span>
              <span style={{ color: "#5fd0ff" }}>{c.away} {m.out}</span>
              <span style={{ color: C.faint }}> · </span>
              <span style={{ color: c.idle > 0 ? C.good : C.faint }}>{c.idle} idle</span>
            </span>
          );
        }) : open ? (
          <span style={{ color: C.faint }}>
            {v.houses > 0 ? "no house seated here owns a hull" : "no merchant house is seated here"}
          </span>
        ) : null}
        <span style={{ flex: 1 }} />
        {/* The cargo count stays visible when folded — it is the live figure, and
            a fold that hides everything gives no reason to unfold. */}
        <span style={{ color: C.inkDim }}>
          {v.inbound_cargoes} cargo{v.inbound_cargoes === 1 ? "" : "es"} inbound
          {v.inbound_cargoes > 0 ? ` · soonest ${v.inbound_eta}d` : ""} · {v.outbound_cargoes} out
        </span>
      </div>
      {open && (
        <div style={{ fontSize: 8, color: C.faint, marginTop: 1 }}>
          Hulls belonging to the {v.houses} house{v.houses === 1 ? "" : "s"} seated here — a registry, not
          a berth count: the sim gives a vessel no position.
          {v.land_pooled ? " Caravan and river capacity are pooled, so the land split is indicative." : ""}
        </div>
      )}
    </div>
  );
}

/** One aggregated side of the book for a good (what we bought / what we sold). */
type Side = { units: number; value: number; deals: number };
const emptySide = (): Side => ({ units: 0, value: 0, deals: 0 });
const avgOf = (s: Side) => (s.units > 1e-6 ? s.value / s.units : 0);

/** A book row: one good, with both sides of its trade and its live market state. */
type BookRow = {
  g: HubGoodDetail;
  buy: Side;
  sell: Side;
  /** Mean sell price less mean buy price (×-world). Null when one side is empty —
   *  a spread needs two sides, and inventing one from a single side would be a
   *  made-up number, not a thin one. */
  spread: number | null;
  /** Stock expressed in DAYS of this city's own daily need for the good. */
  days: number;
  xw: number;
  /** Sort key: how UNUSUAL this good is here — how far the local price sits from
   *  the world average, weighted by how much the city actually wants it. */
  odd: number;
  verdict: { text: string; tone: string } | null;
};

function sideFrom(rows: ShipmentRow[], good: string): Side {
  const s = emptySide();
  for (const r of rows) {
    if (r.good !== good) continue;
    // Fall back to the local quote only when the deal price is genuinely unknown
    // (a save whose in-flight legs predate `InTransit.price`).
    const p = r.deal_price && r.deal_price > 0 ? r.deal_price : r.price;
    s.units += r.amount;
    s.value += r.amount * p;
    s.deals += 1;
  }
  return s;
}

/** The one-line reading of a good's position here. Derived ONLY from state the
 *  panel actually holds — nothing about dispatch's internals is guessed at. */
function verdictOf(g: HubGoodDetail, buy: Side, sell: Side, days: number, xw: number) {
  const traded = buy.units > 0.01 || sell.units > 0.01;
  if (g.production <= 0.01 && !traded && g.stock <= 0.01) {
    return { text: "absent — nothing reaches this market", tone: C.faint };
  }
  if (xw > 1.6) return { text: sell.units > buy.units ? "DEAR — and still leaving" : "DEAR", tone: C.warn };
  if (days > 45 && xw < 0.6) return { text: "glut — more than anyone here will buy", tone: C.inkDim };
  if (g.production > 0.01 && sell.units > buy.units * 2) return { text: "we export", tone: C.good };
  if (buy.units > sell.units * 2 && buy.units > 0.01) return { text: "we import", tone: C.inkMid };
  if (days < 5 && g.need > 0.01) return { text: "thin — days of stock, not weeks", tone: C.warn };
  return null;
}

/** Sparkline over the PERSISTED yearly series. Draws nothing rather than a flat
 *  line when there is no history — an invented trend is worse than an absent one. */
function Spark({ vals, base, w = 62, h = 14 }: { vals: number[]; base: number; w?: number; h?: number }) {
  if (!vals || vals.length < 2) {
    return <span style={{ color: C.faint, fontSize: 8, letterSpacing: 1 }}>·······</span>;
  }
  const xs = vals.map((v) => v / Math.max(1e-6, base));
  const lo = Math.min(...xs, 1), hi = Math.max(...xs, 1);
  const span = Math.max(1e-6, hi - lo);
  const pts = xs.map((v, i) =>
    `${((i / (xs.length - 1)) * w).toFixed(1)},${(h - ((v - lo) / span) * (h - 2) - 1).toFixed(1)}`).join(" ");
  const oneY = h - ((1 - lo) / span) * (h - 2) - 1;
  const rising = xs[xs.length - 1] > xs[0] * 1.15;
  const falling = xs[xs.length - 1] < xs[0] * 0.87;
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 2 }}>
      <svg width={w} height={h} style={{ display: "block", background: C.card, borderRadius: 2 }}>
        <line x1={0} y1={oneY} x2={w} y2={oneY} stroke="#2a3a4e" strokeWidth={0.5} strokeDasharray="2 2" />
        <polyline points={pts} fill="none" stroke={C.gold} strokeWidth={1} />
      </svg>
      {rising ? <span style={{ color: C.warn, fontSize: 8 }}>▲</span>
        : falling ? <span style={{ color: C.good, fontSize: 8 }}>▼</span> : null}
    </span>
  );
}

/** Who supplied this good here, as a five-class stacked bar. Silent (a dash) when
 *  nothing has arrived — an empty bar would read as "supplied by nobody" rather than
 *  "nothing to attribute". */
function SupplyBar({ shares }: { shares?: [number, number, number, number, number] }) {
  const total = (shares ?? []).reduce((a, b) => a + b, 0);
  if (!shares || total < 0.01) return <span style={{ color: C.faint }}>—</span>;
  const top = shares.indexOf(Math.max(...shares));
  return (
    <span title={shares.map((v, i) => `${SUPPLY_LABEL[i]} ${Math.round(v * 100)}%`).filter((_, i) => shares[i] > 0.005).join(" · ")}
      style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
      <span style={{ display: "inline-flex", width: 34, height: 6, borderRadius: 2, overflow: "hidden", background: C.raised }}>
        {shares.map((v, i) => v > 0.005
          ? <span key={i} style={{ width: `${v * 100}%`, background: SUPPLY_TINT[i] }} />
          : null)}
      </span>
      <span style={{ color: SUPPLY_TINT[top], fontSize: 8 }}>{SUPPLY_LABEL[top]}</span>
    </span>
  );
}

/** VARIANT C's side column: one collapsible block per partner city, cargo nested. */
function QuayColumn({ quays, side, openCity, setOpenCity, icon, label, focus, labelOf }: {
  quays: Quay[]; side: "in" | "out";
  openCity: string | null; setOpenCity: (c: string | null) => void;
  icon: (id: string) => string; label: (id: string) => string;
  focus: string | null; labelOf: (id: string) => string;
}) {
  const tint = side === "in" ? C.buy : C.sell;
  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ color: tint, fontSize: 9, fontWeight: 700, marginBottom: 3,
        textAlign: side === "out" ? "right" : "left" }}>
        {side === "in" ? "\u21E2 ARRIVING FROM" : "SOLD TO \u21E2"}
        {focus ? <span style={{ color: C.faint, fontWeight: 400 }}> · {labelOf(focus)}</span> : null}
      </div>
      {quays.length === 0 && (
        <div style={{ color: C.faint, fontSize: 9 }}>
          {side === "in" ? "nothing inbound" : "nothing outbound"}
        </div>
      )}
      {quays.map((q) => {
        const isOpen = openCity === q.city;
        return (
          <div key={q.city} style={{ marginBottom: 3 }}>
            <div onClick={() => setOpenCity(isOpen ? null : q.city)}
              style={{ display: "flex", alignItems: "center", gap: 4, cursor: "pointer",
                fontSize: 9.5, padding: "1px 2px", borderRadius: 3,
                background: isOpen ? "#16263a" : "transparent" }}
              title={`${q.lines.length} cargo${q.lines.length === 1 ? "" : "es"} \u00b7 ${fmt(q.value)} gr-eq`}>
              <span style={{ color: C.faint, fontSize: 8 }}>{isOpen ? "\u25BE" : "\u25B8"}</span>
              <span style={{ flex: 1, color: "#cfe0f4", fontWeight: 600, overflow: "hidden",
                textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{q.city}</span>
              {q.eta !== null && <span style={{ color: C.inkDim, fontSize: 8 }}>{"\u23F3"}{q.eta}d</span>}
              <span style={{ fontSize: 9 }}>{q.sea ? "\u26F5" : "\u{1F42B}"}</span>
              {q.inflight > 0 && (
                <span style={{ color: C.inkDim, fontSize: 8 }} title="cargoes in flight on this lane">
                  ×{q.inflight}
                </span>
              )}
              <span style={{ color: tint, fontSize: 9 }}>{fmt(q.value)}</span>
            </div>
            {isOpen && q.lines.slice(0, 8).map((r, i) => {
              const deal = r.deal_price && r.deal_price > 0 ? r.deal_price : null;
              const gap = deal !== null ? deal - r.price : null;
              return (
                <div key={i} style={{ display: "flex", gap: 4, alignItems: "baseline",
                  fontSize: 8.5, paddingLeft: 12, color: C.inkMid }}>
                  <span style={{ width: 7, height: 7, borderRadius: 2, background: r.color, flex: "0 0 auto" }} />
                  <span style={{ flex: 1, color: C.ink, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {icon(r.good)} {label(r.good)} ×{fmt(r.amount)}
                  </span>
                  <span style={{ color: C.gold }}>{deal !== null ? deal.toFixed(2) : "\u2014"}</span>
                  {gap !== null && Math.abs(gap) > 0.02 && (
                    <span style={{ color: gap < 0 ? C.good : C.warn, fontSize: 8 }}>
                      {gap < 0 ? "\u25BC" : "\u25B2"}
                    </span>
                  )}
                </div>
              );
            })}
            {isOpen && (
              <div style={{ paddingLeft: 12, fontSize: 8, color: C.faint }}>
                {q.lines.length} cargo{q.lines.length === 1 ? "" : "es"} · {fmt(q.units)} units
                {q.lines.some((l) => l.is_guild) ? " \u00b7 incl. guild" : ""}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

/** One deal, on either side of the book. Shows the DEAL price and its gap to the
 *  local quote — the buyer's actual position — plus when it lands or how long ago
 *  it was struck. */
function DealRow({ s, side, icon, label }: {
  s: ShipmentRow; side: "in" | "out";
  icon: (id: string) => string; label: (id: string) => string;
}) {
  const deal = s.deal_price && s.deal_price > 0 ? s.deal_price : null;
  const gap = deal !== null ? deal - s.price : null;
  const when = s.eta_days && s.eta_days > 0
    ? <span style={{ color: C.inkDim }}>⏳{s.eta_days}d</span>
    : <span style={{ color: C.faint }}>{s.age_days ? `${s.age_days}d ago` : "landed"}</span>;
  return (
    <div style={{ fontSize: 8.5, marginBottom: 3, lineHeight: 1.3 }}
      title={`${s.owner}${s.is_guild ? " (guild)" : ""} · ${side === "in" ? "from" : "to"} ${s.other} · ${label(s.good)} ${fmt(s.amount)} · value ${fmt(s.value)}`}>
      <div style={{ display: "flex", alignItems: "center", gap: 3 }}>
        {when}
        <span style={{ width: 7, height: 7, borderRadius: 2, background: s.color, flex: "0 0 auto" }} />
        <span style={{ flex: 1, color: s.is_guild ? C.inkMid : "#cfe0f4", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {s.returning_home ? "↩ " : ""}{s.owner}
        </span>
        <span style={{ color: C.faint }}>{side === "in" ? "←" : "→"}</span>
        <span style={{ color: C.inkMid, maxWidth: 62, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{s.other}</span>
      </div>
      <div style={{ display: "flex", gap: 4, color: C.inkMid, paddingLeft: 12 }}>
        <span style={{ color: C.ink }}>{icon(s.good)} ×{fmt(s.amount)}</span>
        <span style={{ color: C.gold }}>{deal !== null ? `@ ${deal.toFixed(2)}` : "@ —"}</span>
        {gap !== null && Math.abs(gap) > 0.02 && (
          <span style={{ color: gap < 0 ? C.good : C.warn }}>
            {gap < 0 ? "▼" : "▲"} {gap > 0 ? "+" : ""}{gap.toFixed(2)}
          </span>
        )}
        <span style={{ flex: 1 }} />
        <span>{s.sea ? "⛵" : "🐫"}</span>
        <span style={{ color: C.faint }}>{fmt(s.value)}</span>
      </div>
    </div>
  );
}

type Sort = "odd" | "spread" | "traded" | "name";
const SORTS: [Sort, string][] = [
  ["odd", "unusual"], ["spread", "spread"], ["traded", "traded"], ["name", "name"],
];

/** The five seller classes of `SUPPLY_*`, in payload order. */
const SUPPLY_LABEL = ["city", "houses", "guilds", "local", "foreign"] as const;
const SUPPLY_TINT = ["#8aa0c0", "#c9a04a", "#7fa0c0", "#8fbf8f", "#c07a5a"];

/** Estate kind → the word a reader knows it by (1 farm … 6 manufactory). */
const ESTATE_WORD = ["works", "farm", "mine", "plantation", "fishery", "vineyard", "manufactory"];

/** One partner city on either side of the quay, with the cargo that moved. */
type Quay = {
  city: string;
  lines: ShipmentRow[];
  units: number;
  value: number;
  /** Soonest ETA among in-flight cargoes, or null when everything here has landed. */
  eta: number | null;
  sea: boolean;
  /** Distinct in-flight CARGOES on this lane. Deliberately not called vessels — one
   *  shipment consumes one fleet slot whatever its size, so a cargo count is the only
   *  honest figure until vessels are real. */
  inflight: number;
};

/** Group one side's shipments by the partner city, busiest first. */
function quaysOf(rows: ShipmentRow[], focus: string | null): Quay[] {
  const m = new Map<string, Quay>();
  for (const r of rows) {
    if (focus && r.good !== focus) continue;
    const q = m.get(r.other) ?? { city: r.other, lines: [], units: 0, value: 0, eta: null, sea: false, inflight: 0 };
    q.lines.push(r);
    q.units += r.amount;
    q.value += r.value;
    q.sea = q.sea || r.sea;
    if (r.eta_days && r.eta_days > 0) {
      q.inflight += 1;
      q.eta = q.eta === null ? r.eta_days : Math.min(q.eta, r.eta_days);
    }
    m.set(r.other, q);
  }
  for (const q of m.values()) q.lines.sort((a, b) => b.value - a.value);
  return [...m.values()].sort((a, b) => b.value - a.value);
}

export function CityMarketView({ detail, compact, onFocusGood }: {
  detail: HubDetail;
  compact?: boolean;
  /** Called with the good whose book was opened (null when closed). The settlement
   *  window uses it to trace that good's supply road on the map — the job the old
   *  Imports list did before the book absorbed it. */
  onFocusGood?: (good: string | null) => void;
}) {
  const goodMeta = useGoodsStore((s) => s.meta);
  const icon = (id: string) => goodMeta(id).icon;
  const label = (id: string) => goodMeta(id).name;
  const [sort, setSort] = useState<Sort>("odd");
  const [open, setOpen] = useState<string | null>(null);
  const [focus, setFocus] = useState<string | null>(null); // filter deals to one good
  const [group, setGroup] = useState<"city" | "good">("city");
  const [openQuay, setOpenQuay] = useState<string | null>(null);

  const buys = useMemo(
    () => [...(detail.arrivals ?? []), ...(detail.recent_arrivals ?? [])], [detail]);
  const sells = useMemo(
    () => [...(detail.departures ?? []), ...(detail.recent_departures ?? [])], [detail]);
  const inQuays = useMemo(() => quaysOf(buys, focus), [buys, focus]);
  const outQuays = useMemo(() => quaysOf(sells, focus), [sells, focus]);
  // MADE HERE · the city's own fields plus every estate and manufactory in its
  // hinterland. `estates_here` are separate hubs parented to this one, so a good's
  // `production` on the city itself and its estates' output are different figures
  // and are shown as different lines.
  const madeHere = useMemo(() => {
    // Which of this city's own products actually LEAVE it. Manufactured goods in
    // particular are sized against local population and so tend to clear the
    // export reserve rarely — that is a real property of the model, and a view
    // that shows output without showing whether any of it ships hides it.
    const leaves = new Set(sells.filter((s) => s.amount > 0.01).map((s) => s.good));
    const own = detail.goods
      .filter((g) => g.production > 0.01)
      .sort((a, b) => b.production - a.production)
      .map((g) => ({
        good: g.name, output: g.production, source: "city fields", grade: g.grade ?? "",
        tier: 0, damage: 0, exported: leaves.has(g.name),
      }));
    const est = (detail.estates_here ?? [])
      .filter((e) => e.output > 0.01)
      .sort((a, b) => b.output - a.output)
      .map((e) => ({
        good: e.good,
        output: e.output,
        source: e.name || ESTATE_WORD[Math.min(e.kind, 6)] || "works",
        grade: "",
        tier: e.tier ?? 0,
        damage: e.damage ?? 0,
        exported: leaves.has(e.good),
      }));
    const all = [...own, ...est];
    return {
      rows: all.slice(0, 14),
      works: (detail.estates_here ?? []).length,
      stuck: all.filter((m) => !m.exported).length,
    };
  }, [detail, sells]);

  const rows = useMemo<BookRow[]>(() => {
    const out: BookRow[] = [];
    for (const g of detail.goods) {
      const buy = sideFrom(buys, g.name);
      const sell = sideFrom(sells, g.name);
      const traded = buy.units > 0.01 || sell.units > 0.01;
      if (!traded && g.production <= 0.01 && g.stock <= 0.01) continue;
      const base = Math.max(1e-6, g.base_value);
      const xw = g.price / base;
      const ab = avgOf(buy), as = avgOf(sell);
      const spread = buy.units > 0.01 && sell.units > 0.01 ? as - ab : null;
      // `need` is the sim's own PER-TICK (daily) demand, so stock ÷ need is days.
      const days = g.need > 1e-6 ? g.stock / g.need : 0;
      const weight = Math.max(0.15, g.need / Math.max(1e-6, detail.population));
      const odd = Math.abs(xw - (g.world_avg ?? 1)) * weight;
      out.push({ g, buy, sell, spread, days, xw, odd, verdict: verdictOf(g, buy, sell, days, xw) });
    }
    const key: Record<Sort, (r: BookRow) => number | string> = {
      odd: (r) => -r.odd,
      spread: (r) => -(r.spread ?? -Infinity),
      traded: (r) => -(r.buy.units + r.sell.units),
      name: (r) => label(r.g.name),
    };
    return out.sort((a, b) => {
      const ka = key[sort](a), kb = key[sort](b);
      return ka < kb ? -1 : ka > kb ? 1 : 0;
    });
  }, [detail, buys, sells, sort, goodMeta]);

  // The market's own headline: turnover, the balance of trade, and the one thing
  // worth knowing. A healthy market says nothing — the same "quiet unless it
  // matters" rule the house stability gauges follow.
  const bought = detail.bought ?? 0, sold = detail.sold ?? 0;
  const standout = rows.find((r) => r.verdict && (r.verdict.tone === C.warn));

  const dealsFor = (side: ShipmentRow[]) =>
    (focus ? side.filter((s) => s.good === focus) : side).slice(0, compact ? 10 : 16);

  return (
    <>
      {/* ── the square itself: the stalls, their keepers and the crowd ─────── */}
      {!compact && <MarketSquare detail={detail} />}
      {/* ── the market's vital line ─────────────────────────────────────────── */}
      <div style={{ display: "flex", alignItems: "baseline", gap: 8, flexWrap: "wrap",
        padding: "3px 0 5px", borderBottom: `1px solid ${C.line}`, fontSize: 10 }}>
        <span style={{ color: C.head, fontWeight: 700 }}>⚖ Market</span>
        <span style={{ color: C.buy }}>bought {fmt(bought)}</span>
        <span style={{ color: C.sell }}>sold {fmt(sold)}</span>
        <span style={{ color: sold - bought >= 0 ? C.good : C.warn }}>
          net {sold - bought >= 0 ? "+" : ""}{fmt(sold - bought)}
        </span>
        <span style={{ flex: 1 }} />
        {(["city", "good"] as const).map((gmode) => (
          <span key={gmode} onClick={() => setGroup(gmode)} style={{
            cursor: "pointer", fontSize: 9, padding: "1px 6px", borderRadius: 3,
            background: group === gmode ? "#21344a" : "transparent",
            color: group === gmode ? "#cfe2f6" : C.inkDim,
            border: `1px solid ${group === gmode ? "#3a80c0" : "transparent"}`,
          }}>{gmode === "city" ? "by city" : "by good"}</span>
        ))}
        <span style={{ width: 6 }} />
        {SORTS.map(([id, lbl]) => (
          <span key={id} onClick={() => setSort(id)} style={{
            cursor: "pointer", fontSize: 9, padding: "1px 6px", borderRadius: 3,
            background: sort === id ? "#21344a" : "transparent",
            color: sort === id ? "#cfe2f6" : C.inkDim,
            border: `1px solid ${sort === id ? "#3a80c0" : "transparent"}`,
          }}>{lbl}</span>
        ))}
      </div>
      {detail.vessels && <VesselRegistry v={detail.vessels} />}
      {standout && (
        <div style={{ fontSize: 9.5, color: C.warn, padding: "3px 0" }}>
          ⚠ {label(standout.g.name)} at {standout.xw.toFixed(2)}× — {standout.verdict!.text.toLowerCase()}
        </div>
      )}

      {/* ── VARIANT C · THE QUAY: partner cities left and right, the market between ── */}
      {group === "city" && (
        <div style={{ display: "flex", gap: 8, marginTop: 4, alignItems: "flex-start" }}>
          <QuayColumn quays={inQuays} side="in" openCity={openQuay} setOpenCity={setOpenQuay}
            icon={icon} label={label} focus={focus} labelOf={label} />

          <div style={{ flex: 1.35, minWidth: 0, padding: "0 8px",
            borderLeft: `1px solid ${C.line}`, borderRight: `1px solid ${C.line}` }}>
            {/* MADE HERE — the city's own fields, then every estate and manufactory */}
            <div style={{ color: C.head, fontSize: 9, fontWeight: 700, textAlign: "center" }}>── MADE HERE ──</div>
            {madeHere.rows.length === 0 && <div style={{ color: C.faint, fontSize: 9 }}>produces nothing of note</div>}
            {madeHere.rows.map((m, i) => (
              <div key={i} style={{ display: "flex", gap: 4, fontSize: 9, alignItems: "baseline" }}>
                <span style={{ flex: 1, color: C.ink, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {icon(m.good)} {label(m.good)}
                </span>
                {/* A works that never ships anything is the interesting case, so it
                    is the one that gets a mark. Silent when the good does leave —
                    the same "quiet unless it matters" rule the rest of the panel keeps. */}
                {!m.exported && (
                  <span style={{ color: C.faint, fontSize: 8 }} title="none of this leaves the city">⌂</span>
                )}
                {m.damage > 0.05 && (
                  <span style={{ color: C.warn, fontSize: 8 }} title={`${Math.round(m.damage * 100)}% damaged`}>⚠</span>
                )}
                <span style={{ color: C.inkMid }}>{fmt(m.output)}</span>
                <span style={{ color: C.faint, fontSize: 8, maxWidth: 92, overflow: "hidden",
                  textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {m.source}{m.tier > 1 ? ` t${m.tier}` : ""}
                </span>
              </div>
            ))}
            {madeHere.rows.length > 0 && (
              <div style={{ fontSize: 8, color: madeHere.stuck > 0 ? C.inkDim : C.faint, paddingTop: 1 }}>
                {madeHere.works} works · {madeHere.stuck === 0
                  ? "everything made here also ships"
                  : `⌂ ${madeHere.stuck} of ${madeHere.rows.length} never leave the city`}
              </div>
            )}

            {/* ON THE MARKET — what is on offer, how long it lasts, and who supplied it */}
            <div style={{ color: C.head, fontSize: 9, fontWeight: 700, textAlign: "center", marginTop: 6 }}>
              ── ON THE MARKET ──
            </div>
            {/* The four numeric columns were unlabelled except for "held", which
                showed DAYS while reading like a quantity. Units and days are now
                separate columns with their own headers: "39,284 · 312d" says glut
                without needing the word. */}
            <div style={{ display: "flex", gap: 4, fontSize: 8, color: C.faint,
              borderBottom: `1px solid ${C.line}`, paddingBottom: 1 }}>
              <span style={{ flex: 1 }}>good</span>
              <span style={{ width: 42, textAlign: "right" }}>on stall</span>
              <span style={{ width: 30, textAlign: "right" }}>cover</span>
              <span style={{ width: 38, textAlign: "right" }}>price</span>
              <span style={{ width: 72 }}>supplied by</span>
            </div>
            {rows.length === 0 && <div style={{ color: C.faint, fontSize: 9 }}>no market here yet</div>}
            {rows.map((r) => {
              const isOpen = open === r.g.name;
              return (
                <div key={r.g.good}>
                  <div
                    onClick={() => {
                      const next = isOpen ? null : r.g.name;
                      setOpen(next); setFocus(next); onFocusGood?.(next);
                    }}
                    style={{ display: "flex", gap: 4, alignItems: "center", fontSize: 9,
                      padding: "1px 2px", cursor: "pointer", borderRadius: 3,
                      borderBottom: `1px solid ${C.lineSoft}`,
                      background: isOpen ? "#16263a" : "transparent" }}
                    title="Click for this good's book — who supplied it, where it went, and its price history"
                  >
                    <span style={{ flex: 1, color: C.ink, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      <span style={{ color: C.faint, fontSize: 8 }}>{isOpen ? "▾ " : "▸ "}</span>
                      {icon(r.g.name)} {label(r.g.name)}
                    </span>
                    {/* UNITS — the quantity actually standing on the stall. It has
                        always been in `HubGoodDetail.stock` and was never shown. */}
                    <span style={{ width: 42, textAlign: "right", color: C.inkMid,
                      fontVariantNumeric: "tabular-nums" }}
                      title={r.g.depot_stock && r.g.depot_stock > 0.5
                        ? `${Math.round(r.g.depot_stock).toLocaleString()} more held off-market, in a house's own depot`
                        : undefined}>
                      {r.g.stock > 0.5 ? Math.round(r.g.stock).toLocaleString() : "—"}
                      {r.g.depot_stock && r.g.depot_stock > 0.5 && (
                        <span style={{ color: C.faint, fontWeight: 400 }}>
                          {" "}+{Math.round(r.g.depot_stock).toLocaleString()}
                        </span>
                      )}
                    </span>
                    <span style={{ width: 30, textAlign: "right", fontVariantNumeric: "tabular-nums",
                      color: coverColor(r.days, r.g.need) }}>
                      {coverLabel(r.days, r.g.need)}
                    </span>
                    <span style={{ width: 38, textAlign: "right", fontWeight: 600, color: priceColor(r.xw) }}>
                      {r.xw.toFixed(2)}×
                    </span>
                    <span style={{ width: 72 }}><SupplyBar shares={r.g.supply_shares} /></span>
                  </div>
                  {r.verdict && !isOpen && (
                    <div style={{ fontSize: 8, color: r.verdict.tone, paddingLeft: 14 }}>{r.verdict.text}</div>
                  )}
                  {isOpen && <BookDetail r={r} buys={buys} sells={sells} icon={icon} label={label} />}
                </div>
              );
            })}
          </div>

          <QuayColumn quays={outQuays} side="out" openCity={openQuay} setOpenCity={setOpenQuay}
            icon={icon} label={label} focus={focus} labelOf={label} />
        </div>
      )}
      {group === "city" && (
        <div style={{ fontSize: 8, color: C.faint, marginTop: 4 }}>
          Prices are ×-world-standard, and a cargo's own price is what the deal was struck at.
          Lane counts are CARGOES, not vessels — a shipment takes one fleet slot whatever its
          size. ⌂ marks a good this city makes and never ships.
        </div>
      )}


      {group === "good" && (<>
      {/* ── THE BOOK ────────────────────────────────────────────────────────── */}
      <div style={{ display: "flex", fontSize: 8, color: C.faint, padding: "4px 2px 2px" }}>
        <span style={{ flex: "0 0 108px" }}>good</span>
        <span style={{ width: 74, textAlign: "right", color: C.buy }}>bought @</span>
        <span style={{ width: 74, textAlign: "right", color: C.sell }}>sold @</span>
        <span style={{ width: 42, textAlign: "right" }}>spread</span>
        <span style={{ width: 34, textAlign: "right" }}>held</span>
        <span style={{ width: 40, textAlign: "right" }}>local</span>
        <span style={{ width: 74, textAlign: "right" }}>price, yearly</span>
      </div>
      {rows.length === 0 && (
        <div style={{ color: C.faint, fontSize: 9, padding: "6px 2px" }}>no market here yet</div>
      )}
      {rows.map((r) => {
        const isOpen = open === r.g.name;
        return (
          <div key={r.g.good}>
            <div
              onClick={() => {
                const next = isOpen ? null : r.g.name;
                setOpen(next); setFocus(next); onFocusGood?.(next);
              }}
              style={{ display: "flex", alignItems: "center", fontSize: 9.5, padding: "2px 2px",
                borderBottom: `1px solid ${C.lineSoft}`, cursor: "pointer",
                background: isOpen ? "#16263a" : "transparent" }}
              title="Click to open this good's book — who supplied it, where it went, and its history"
            >
              <span style={{ flex: "0 0 108px", color: C.ink, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                <span style={{ color: C.faint, fontSize: 8 }}>{isOpen ? "▾ " : "▸ "}</span>
                {icon(r.g.name)} {label(r.g.name)}
              </span>
              <span style={{ width: 74, textAlign: "right", color: r.buy.units > 0 ? C.buy : C.faint }}>
                {r.buy.units > 0.01 ? `${fmt(r.buy.units)} @ ${avgOf(r.buy).toFixed(2)}` : "—"}
              </span>
              <span style={{ width: 74, textAlign: "right", color: r.sell.units > 0 ? C.sell : C.faint }}>
                {r.sell.units > 0.01 ? `${fmt(r.sell.units)} @ ${avgOf(r.sell).toFixed(2)}` : "—"}
              </span>
              <span style={{ width: 42, textAlign: "right", fontWeight: 600,
                color: r.spread === null ? C.faint : r.spread >= 0 ? C.good : C.warn }}>
                {r.spread === null ? "—" : `${r.spread >= 0 ? "+" : ""}${r.spread.toFixed(2)}`}
              </span>
              <span style={{ width: 34, textAlign: "right", color: r.days < 5 ? C.warn : C.inkMid }}>
                {r.g.need > 1e-6 ? `${Math.round(r.days)}d` : "—"}
              </span>
              <span style={{ width: 40, textAlign: "right", fontWeight: 600, color: priceColor(r.xw) }}>
                {r.xw.toFixed(2)}×
              </span>
              <span style={{ width: 74, textAlign: "right" }}>
                <Spark vals={r.g.price_hist ?? []} base={r.g.base_value} />
              </span>
            </div>
            {r.verdict && !isOpen && (
              <div style={{ fontSize: 8, color: r.verdict.tone, paddingLeft: 122, marginTop: -1, marginBottom: 1 }}>
                {r.verdict.text}
              </div>
            )}
            {isOpen && <BookDetail r={r} buys={buys} sells={sells} icon={icon} label={label} />}
          </div>
        );
      })}

      {/* ── the deals themselves, still the buy/sell columns ─────────────────── */}
      <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ color: C.buy, fontSize: 9, fontWeight: 700, marginBottom: 2 }}>
            ⇢ WE BUY {focus ? <span style={{ color: C.faint, fontWeight: 400 }}>· {label(focus)}</span> : null}
          </div>
          {dealsFor(buys).length === 0 && <div style={{ color: C.faint, fontSize: 9 }}>nothing inbound</div>}
          {dealsFor(buys).map((s, i) => <DealRow key={"b" + i} s={s} side="in" icon={icon} label={label} />)}
        </div>
        <div style={{ flex: 1, minWidth: 0, borderLeft: `1px solid ${C.line}`, paddingLeft: 8 }}>
          <div style={{ color: C.sell, fontSize: 9, fontWeight: 700, marginBottom: 2 }}>
            WE SELL ⇢ {focus ? <span style={{ color: C.faint, fontWeight: 400 }}>· {label(focus)}</span> : null}
          </div>
          {dealsFor(sells).length === 0 && <div style={{ color: C.faint, fontSize: 9 }}>nothing outbound</div>}
          {dealsFor(sells).map((s, i) => <DealRow key={"s" + i} s={s} side="out" icon={icon} label={label} />)}
        </div>
      </div>
      <div style={{ fontSize: 8, color: C.faint, marginTop: 4 }}>
        Prices are ×-world-standard. “bought @ / sold @” aggregate the deals shown —
        in-flight plus recently completed — at the price each was actually struck at.
      </div>
      </>)}
    </>
  );
}

/** The expanded book row: who supplied it, where it went, and the persisted
 *  price/volume history — absorbing what used to be three separate sections. */
function BookDetail({ r, buys, sells, icon, label }: {
  r: BookRow; buys: ShipmentRow[]; sells: ShipmentRow[];
  icon: (id: string) => string; label: (id: string) => string;
}) {
  const byCity = (rows: ShipmentRow[]) => {
    const m = new Map<string, { units: number; value: number }>();
    for (const s of rows) {
      if (s.good !== r.g.name) continue;
      const p = s.deal_price && s.deal_price > 0 ? s.deal_price : s.price;
      const e = m.get(s.other) ?? { units: 0, value: 0 };
      e.units += s.amount; e.value += s.amount * p;
      m.set(s.other, e);
    }
    return [...m.entries()].sort((a, b) => b[1].units - a[1].units).slice(0, 5);
  };
  const from = byCity(buys), to = byCity(sells);
  const g = r.g;
  const base = Math.max(1e-6, g.base_value);
  const cheapest = g.world_min ?? 0, dearest = g.world_max ?? 0;
  const multiple = cheapest > 1e-6 ? r.xw / cheapest : 0;

  const Line = ({ k, children }: { k: string; children: React.ReactNode }) => (
    <div style={{ display: "flex", gap: 6, fontSize: 9, padding: "1px 0" }}>
      <span style={{ flex: "0 0 92px", color: C.faint, textTransform: "uppercase", fontSize: 8, letterSpacing: 0.4, paddingTop: 1 }}>{k}</span>
      <span style={{ flex: 1, minWidth: 0, color: C.inkMid }}>{children}</span>
    </div>
  );

  return (
    <div style={{ background: C.card, border: `1px solid ${C.line}`, borderRadius: 4, padding: "5px 7px", margin: "2px 0 4px" }}>
      <div style={{ color: C.head, fontSize: 10, marginBottom: 3 }}>
        {icon(g.name)} {label(g.name)} — {r.xw.toFixed(2)}× world standard
        {g.grade ? <span style={{ color: C.inkMid }}> · we make it {g.grade.toLowerCase()}</span> : null}
      </div>
      <Line k="bought from">
        {from.length === 0 ? <span style={{ color: C.faint }}>nothing arrives</span> : from.map(([city, e]) => (
          <span key={city} style={{ marginRight: 10 }}>
            {city} <b style={{ color: C.buy }}>{fmt(e.units)}</b> @ {(e.value / Math.max(1e-6, e.units)).toFixed(2)}
          </span>
        ))}
      </Line>
      <Line k="sold to">
        {to.length === 0 ? <span style={{ color: C.faint }}>nothing leaves</span> : to.map(([city, e]) => (
          <span key={city} style={{ marginRight: 10 }}>
            {city} <b style={{ color: C.sell }}>{fmt(e.units)}</b> @ {(e.value / Math.max(1e-6, e.units)).toFixed(2)}
          </span>
        ))}
      </Line>
      <Line k="on the stall">
        <b style={{ color: C.ink }}>{Math.round(g.stock).toLocaleString()}</b> units
        {g.need > 1e-6
          ? <> · this city eats <b>{fmt(g.need)}</b> a day, so <b style={{ color: coverColor(r.days, g.need) }}>
              {coverLabel(r.days, g.need)}</b> of cover</>
          : <span style={{ color: C.faint }}> · nobody here consumes it</span>}
      </Line>
      <Line k="made here">
        {g.production > 0.01
          ? <>{fmt(g.production)} a day{g.grade ? `, ${g.grade.toLowerCase()}` : ""}
              {r.sell.units > 0.01
                ? <> · <span style={{ color: C.good }}>{fmt(r.sell.units)} shipped out</span></>
                : <> · <span style={{ color: C.inkDim }}>none of it leaves the city</span></>}</>
          : <span style={{ color: C.faint }}>not produced here</span>}
      </Line>
      <Line k="world">
        cheapest <b style={{ color: C.good }}>{cheapest.toFixed(2)}×</b> at {g.world_min_hub || "—"} ·
        dearest <b style={{ color: C.warn }}>{dearest.toFixed(2)}×</b> at {g.world_max_hub || "—"} ·
        average {(g.world_avg ?? 1).toFixed(2)}×
        {multiple > 1.5 && (
          <span style={{ color: C.warn }}> — we pay <b>{multiple.toFixed(1)}×</b> the cheapest market</span>
        )}
      </Line>
      {(g.price_hist?.length ?? 0) > 1 && (
        <Line k="price, yearly">
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
            <Spark vals={g.price_hist!} base={base} w={150} h={22} />
            <span style={{ color: C.faint, fontSize: 8 }}>
              {(g.price_hist![0] / base).toFixed(2)}× → {(g.price_hist![g.price_hist!.length - 1] / base).toFixed(2)}×
              {" "}over {g.price_hist!.length} yr
              {(g.vol_hist?.length ?? 0) > 1
                ? ` · volume ${fmt(g.vol_hist![g.vol_hist!.length - 1])} last year`
                : ""}
            </span>
          </span>
        </Line>
      )}
      {r.verdict && (
        <Line k="reading"><span style={{ color: r.verdict.tone }}>{r.verdict.text}</span></Line>
      )}
    </div>
  );
}
