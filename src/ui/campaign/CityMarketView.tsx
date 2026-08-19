import { useMemo, useState } from "react";
import type { HubDetail, HubGoodDetail, ShipmentRow } from "@types";
import { useGoodsStore } from "@state/goodsStore";

/** ─────────────────────────────────────────────────────────────────────────────
 *  CITY MARKET 2.0 — `docs/TRADE_AND_MARKET_REVIEW.md` Part 3.
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

  const buys = useMemo(
    () => [...(detail.arrivals ?? []), ...(detail.recent_arrivals ?? [])], [detail]);
  const sells = useMemo(
    () => [...(detail.departures ?? []), ...(detail.recent_departures ?? [])], [detail]);

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
        {SORTS.map(([id, lbl]) => (
          <span key={id} onClick={() => setSort(id)} style={{
            cursor: "pointer", fontSize: 9, padding: "1px 6px", borderRadius: 3,
            background: sort === id ? "#21344a" : "transparent",
            color: sort === id ? "#cfe2f6" : C.inkDim,
            border: `1px solid ${sort === id ? "#3a80c0" : "transparent"}`,
          }}>{lbl}</span>
        ))}
      </div>
      {standout && (
        <div style={{ fontSize: 9.5, color: C.warn, padding: "3px 0" }}>
          ⚠ {label(standout.g.name)} at {standout.xw.toFixed(2)}× — {standout.verdict!.text.toLowerCase()}
        </div>
      )}

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
      <Line k="made here">
        {g.production > 0.01
          ? <>{fmt(g.production)} a day · holding <b>{Math.round(r.days)}</b> days of need</>
          : <span style={{ color: C.faint }}>not produced here · holding {Math.round(r.days)} days of need</span>}
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
