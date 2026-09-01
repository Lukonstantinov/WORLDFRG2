/** Trade ▸ Flows subtab 2.0 — the REALIZED trade at a settlement once a campaign
 *  has run, rebuilt on the shared `@ui/kit` primitives + `chronicleTheme` tokens
 *  (the same system `CityMarketView` and the Province Inspector use) so the Market
 *  and Flows tabs read as one designed surface rather than two.
 *
 *  `docs/TRADE_AND_MARKET_REVIEW.md` Part 3 gave the MARKET tab a merchant's book:
 *  a balance line at the top, rows sorted by what is UNUSUAL rather than by size,
 *  a verdict phrase instead of a raw number, and a healthy row that stays quiet.
 *  Flows is the same tab's other half and had none of it — it was a volume-ordered
 *  list of bars in ad-hoc hexes, which answers "what moves most" (rarely the
 *  question) and never "what is this city's trading position".
 *
 *  Four things the 2.0 view adds, each from that plan's own reasoning:
 *
 *  1. **A BALANCE header.** The market tab opens with turnover/bought/sold/net; the
 *     flows tab now opens with the same for physical volume — what comes in, what
 *     goes out, and the net — plus a one-line verdict naming the city's position
 *     (a net exporter, import-dependent, a balanced entrepôt).
 *  2. **Sorted by what is UNUSUAL by default.** The plan's repeated rule. Here
 *     "unusual" is the IMBALANCE (|in − out| ÷ total): a good this city is lopsided
 *     in is the good worth reading, and it is invisible in a volume ordering where
 *     a large balanced staple always sits on top. Volume order is still one click
 *     away.
 *  3. **A VERDICT PHRASE, not a raw number** — same discipline as the house
 *     stability gauges. "we export", "import-dependent", "⚠ collapsed" — and a good
 *     that is simply traded steadily says nothing at all, so a warning still means
 *     something.
 *  4. **The DEPENDENCE reading on a partner.** A partner's share of all trade is
 *     shown today; what matters is whether losing them would hurt, so the top
 *     partner carries a concentration warning when a single city carries a large
 *     share of everything this settlement trades.
 *
 *  Selecting a good, a direction, a single route or a partner highlights it on the
 *  map exactly as before — that behaviour is unchanged and deliberately so. */
import { useEffect, useMemo, useState } from "react";
import { campaignTradeFlows } from "@bridge";
import type { TradeFlows, TradeFlowGood } from "@types";
import { GOOD_DEFS } from "@goods";
import { Section, Card, Badge, Meter, Chip, EmptyNote, FootNote, StatGrid, Stat } from "@ui/kit";
import { T, SPACE, FZ, RADIUS, SERIF, type Tone } from "@ui/campaign/chronicleTheme";

type Seg = { ax: number; ay: number; bx: number; by: number; dir: number; w: number };

const GOOD_META = new Map(GOOD_DEFS.map((g) => [g.name, g]));
function fmt(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(n >= 10000 ? 0 : 1)}k`;
  return n.toFixed(n >= 100 ? 0 : 1);
}

/** IMPORT blue / EXPORT gold — one pair, used for every directional mark in the
 *  view (sub-rows, route rows, the balance bar) so direction is legible without
 *  reading a label. */
const DIR_IN = "#5fd0ff";
const DIR_OUT = "#ffce5f";

/** A good's trading VERDICT: the phrase a merchant would use, derived from the
 *  in/out split and the trend. `tone` is undefined for an unremarkable good — a
 *  healthy row must stay quiet, or a coloured one stops meaning anything. */
function verdictOf(g: TradeFlowGood): { text: string; tone?: Tone } {
  const total = g.in_volume + g.out_volume;
  const peak = g.history.length >= 2 ? Math.max(...g.history) : 0;
  // A collapse outranks the in/out reading: a good that used to move and no longer
  // does is the most important thing a flows view can say.
  if (peak > 0 && g.last_volume < peak * 0.4) return { text: "collapsed", tone: "bad" };
  if (peak > 0 && g.last_volume < peak * 0.65) return { text: "falling", tone: "warn" };
  if (total <= 0) return { text: "idle" };
  const share = (g.out_volume - g.in_volume) / total;
  if (share > 0.75) return { text: "we export", tone: "good" };
  if (share > 0.25) return { text: "net export" };
  if (share < -0.75) return { text: "import-dependent", tone: "warn" };
  if (share < -0.25) return { text: "net import" };
  return { text: "entrepôt" };
}

/** HOW A GOOD TRAVELS — the sea/overland split, as a phrase plus the share.
 *  `log_trade` is handed each shipment's `sea` flag and the yearly fold used to
 *  discard it, so "did this come by ship or by caravan" was unanswerable a year
 *  later even though the tick knew it every day.
 *
 *  Only TWO modes are offered, deliberately. The sim decides sea travel by
 *  `coastal_a && coastal_b` alone, so a river or lake city's trade reads as
 *  overland however it really moved; a third "river" chip would be inventing a
 *  distinction the model does not make. */
function transportOf(seaVol: number, total: number): { label: string; icon: string; seaPct: number } | null {
  if (total <= 0) return null;
  const seaPct = Math.max(0, Math.min(1, seaVol / total)) * 100;
  if (seaPct >= 85) return { label: "by sea", icon: "⛵", seaPct };
  if (seaPct <= 15) return { label: "overland", icon: "🐫", seaPct };
  return { label: `${seaPct.toFixed(0)}% by sea`, icon: "⛵🐫", seaPct };
}

/** How lopsided a good's trade is, 0 (perfectly balanced) → 1 (one-way only).
 *  The default sort key: a good this city is lopsided in is the one worth reading,
 *  and it is invisible in a volume ordering. Weighted by volume so a one-way trickle
 *  does not outrank a one-way staple. */
function unusualness(g: TradeFlowGood): number {
  const total = g.in_volume + g.out_volume;
  if (total <= 0) return 0;
  return (Math.abs(g.out_volume - g.in_volume) / total) * Math.log1p(total);
}

/** A tiny trend sparkline of a good's yearly trade volume. Green when rising into
 *  the last year, red when it has fallen from its peak. */
function Spark({ vals }: { vals: number[] }) {
  if (vals.length < 2) return <span style={{ color: T.inkFaint, fontSize: FZ.micro }}>no history yet</span>;
  const w = 150, h = 30, max = Math.max(...vals, 1e-6);
  const pts = vals.map((v, i) => `${(i / (vals.length - 1)) * w},${h - (v / max) * (h - 3) - 1.5}`).join(" ");
  const last = vals[vals.length - 1];
  const fallen = last < max * 0.6;
  const color = fallen ? "#e06a5a" : last >= vals[vals.length - 2] ? "#6fce8f" : "#d9c46a";
  return (
    <svg width={w} height={h} style={{ display: "block" }}>
      <polyline points={pts} fill="none" stroke={color} strokeWidth={1.4} />
      <circle cx={w} cy={h - (last / max) * (h - 3) - 1.5} r={2} fill={color} />
    </svg>
  );
}

/** The in/out split of one good as a single two-tone bar — the shape of the trade,
 *  not just its size. Replaces the old single-colour volume bar, which could not
 *  show direction at all without expanding the row. */
function BalanceBar({ inV, outV, max }: { inV: number; outV: number; max: number }) {
  const total = Math.max(inV + outV, 1e-6);
  const width = Math.max(2, (total / Math.max(max, 1e-6)) * 100);
  return (
    <div style={{ flex: 1, minWidth: 60, height: 7, background: T.card, borderRadius: RADIUS.sm, overflow: "hidden" }}>
      <div style={{ width: `${width}%`, height: "100%", display: "flex" }}>
        <div style={{ width: `${(inV / total) * 100}%`, background: DIR_IN }} />
        <div style={{ width: `${(outV / total) * 100}%`, background: DIR_OUT }} />
      </div>
    </div>
  );
}

type Sort = "unusual" | "volume";

export function FlowsView({ hubId, active, tick, setFlowHighlight }: {
  hubId: number; active: boolean; tick: number; setFlowHighlight: (s: Seg[]) => void;
}) {
  const [flows, setFlows] = useState<TradeFlows | null>(null);
  const [loading, setLoading] = useState(false);
  const [selGood, setSelGood] = useState<number | null>(null);
  const [selDir, setSelDir] = useState<number | null>(null); // null=both · 0=import · 1=export
  const [selPartner, setSelPartner] = useState<number | null>(null);
  const [sort, setSort] = useState<Sort>("unusual");
  // A SINGLE isolated route (one partner→here / here→partner for one good), shown on the
  // map on its own with its direction arrow.
  const [selRoute, setSelRoute] = useState<{ good: number; partner: number; dir: number } | null>(null);

  // Fetch on open / when the campaign year ticks over (flows refresh yearly). Track
  // a real loading flag so a resolved-but-empty result ("no trade recorded") is told
  // apart from "still fetching" — otherwise a null reply hung on "Loading…" forever.
  useEffect(() => {
    if (!active) { setFlows(null); setLoading(false); return; }
    let alive = true;
    setLoading(true);
    campaignTradeFlows(hubId)
      .then((f) => { if (alive) { setFlows(f); setLoading(false); } })
      .catch(() => { if (alive) { setFlows(null); setLoading(false); } });
    return () => { alive = false; };
  }, [hubId, active, tick]);

  // Reset selection when the settlement changes.
  useEffect(() => { setSelGood(null); setSelDir(null); setSelPartner(null); setSelRoute(null); }, [hubId]);

  // Drive the map highlight from the current selection. A good can be narrowed to
  // just its IMPORT (dir 0) or EXPORT (dir 1) routes via the sub-rows (#16).
  useEffect(() => {
    if (!flows) { setFlowHighlight([]); return; }
    const ax = flows.hub_x + 0.5, ay = flows.hub_y + 0.5;
    let segs: Seg[] = [];
    if (selRoute) {
      // A single isolated route → one segment, drawn thick with its direction arrow.
      const r = flows.routes.find((x) => x.good === selRoute.good && x.partner === selRoute.partner && x.dir === selRoute.dir);
      if (r) segs = [{ ax, ay, bx: r.px + 0.5, by: r.py + 0.5, dir: r.dir, w: 3.5 }];
      setFlowHighlight(segs);
      return;
    }
    if (selGood != null) {
      const rs = flows.routes.filter((r) => r.good === selGood && (selDir == null || r.dir === selDir));
      const max = Math.max(...rs.map((r) => r.amount), 1e-6);
      segs = rs.map((r) => ({ ax, ay, bx: r.px + 0.5, by: r.py + 0.5, dir: r.dir, w: 1 + (r.amount / max) * 3 }));
    } else if (selPartner != null) {
      const rs = flows.routes.filter((r) => r.partner === selPartner);
      const max = Math.max(...rs.map((r) => r.amount), 1e-6);
      segs = rs.map((r) => ({ ax, ay, bx: r.px + 0.5, by: r.py + 0.5, dir: r.dir, w: 1 + (r.amount / max) * 3 }));
    }
    setFlowHighlight(segs);
  }, [flows, selGood, selDir, selPartner, selRoute, setFlowHighlight]);

  const goodRoutes = useMemo(
    () => (flows && selGood != null
      ? flows.routes.filter((r) => r.good === selGood && (selDir == null || r.dir === selDir))
      : []),
    [flows, selGood, selDir]);

  // ── The city's whole trading position, from the per-good rows it already has ──
  const balance = useMemo(() => {
    if (!flows) return null;
    let inV = 0, outV = 0;
    for (const g of flows.goods) { inV += g.in_volume; outV += g.out_volume; }
    const total = inV + outV;
    const net = outV - inV;
    // The same vocabulary as a good's own verdict, one level up.
    let position = "balanced";
    let tone: Tone | undefined;
    if (total > 0) {
      const share = net / total;
      if (share > 0.35) { position = "a net exporter"; tone = "good"; }
      else if (share < -0.35) { position = "import-dependent"; tone = "warn"; }
      else position = "a balanced entrepôt";
    }
    return { inV, outV, net, total, position, tone };
  }, [flows]);

  const sortedGoods = useMemo(() => {
    if (!flows) return [];
    const gs = [...flows.goods];
    gs.sort(sort === "volume"
      ? (a, b) => b.avg_volume - a.avg_volume
      : (a, b) => unusualness(b) - unusualness(a));
    return gs;
  }, [flows, sort]);

  if (!active) return <EmptyNote>Realized trade appears once a campaign is running.</EmptyNote>;
  if (loading && !flows) return <EmptyNote>Loading trade flows…</EmptyNote>;
  if (!flows) return <EmptyNote>No trade data for this settlement yet.</EmptyNote>;
  if (flows.goods.length === 0) {
    return <EmptyNote>No trade recorded yet — let a campaign year or two pass.</EmptyNote>;
  }

  const maxTotal = Math.max(...flows.goods.map((g) => g.in_volume + g.out_volume), 1e-6);
  const maxPartner = Math.max(...flows.partners.map((p) => p.pct), 1e-6);
  const topPartner = flows.partners[0];

  return (
    <div style={{ fontSize: FZ.body, color: T.ink }}>
      {/* ── The balance: what this city's trade IS, before any detail ────────── */}
      {balance && (
        <Card style={{ marginBottom: SPACE.lg }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: SPACE.md, marginBottom: SPACE.sm }}>
            <span style={{ fontFamily: SERIF, fontSize: FZ.head, color: T.gold, fontWeight: 700 }}>
              {balance.position}
            </span>
            {balance.tone && (
              <Badge tone={balance.tone}>
                {balance.net >= 0 ? "+" : "−"}{fmt(Math.abs(balance.net))} net/yr
              </Badge>
            )}
          </div>
          <StatGrid cols={3}>
            <Stat label="Imported" value={fmt(balance.inV)} hint="per year" />
            <Stat label="Exported" value={fmt(balance.outV)} hint="per year" />
            <Stat label="Goods traded" value={String(flows.goods.length)} hint={`${flows.partners.length} partners`} />
          </StatGrid>
          {/* One bar for the whole city, same two-tone language as every good row. */}
          <div style={{ display: "flex", alignItems: "center", gap: SPACE.sm, marginTop: SPACE.md }}>
            <span style={{ color: DIR_IN, fontSize: FZ.micro }}>◀ in</span>
            <BalanceBar inV={balance.inV} outV={balance.outV} max={balance.total} />
            <span style={{ color: DIR_OUT, fontSize: FZ.micro }}>out ▶</span>
          </div>
        </Card>
      )}

      {/* ── Traded goods ─────────────────────────────────────────────────────── */}
      <Section
        title="Traded goods"
        right={
          <span style={{ display: "flex", gap: 4 }}>
            <Chip on={sort === "unusual"} onClick={() => setSort("unusual")}>unusual</Chip>
            <Chip on={sort === "volume"} onClick={() => setSort("volume")}>volume</Chip>
          </span>
        }
      >
        {sortedGoods.slice(0, 16).map((g) => {
          const meta = GOOD_META.get(g.name);
          const sel = selGood === g.good;
          const v = verdictOf(g);
          return (
            <div
              key={g.good}
              style={{
                display: "flex", flexWrap: "wrap", alignItems: "center", gap: SPACE.sm,
                padding: "3px 4px", borderRadius: RADIUS.sm, cursor: "pointer",
                background: sel ? T.card : "transparent",
              }}
            >
              <div
                data-no-drag
                style={{ display: "flex", alignItems: "center", gap: SPACE.sm, width: "100%" }}
                onClick={() => { setSelGood(sel ? null : g.good); setSelDir(null); setSelPartner(null); setSelRoute(null); }}
              >
                <span style={{ width: 12, color: T.inkFaint }}>{sel ? "▾" : "▸"}</span>
                <span style={{ width: 16 }}>{meta?.emoji ?? "•"}</span>
                <span style={{ flex: 1, minWidth: 70, color: sel ? T.gold : T.ink }}>{meta?.label ?? g.name}</span>
                <BalanceBar inV={g.in_volume} outV={g.out_volume} max={maxTotal} />
                <span style={{ width: 54, textAlign: "right", color: T.inkMid }}>{fmt(g.avg_volume)}/yr</span>
                {/* The verdict, not a number. A steady, balanced good gets a muted
                    phrase and no badge, so a coloured one still carries weight. */}
                <span style={{ width: 96, textAlign: "right" }}>
                  {v.tone
                    ? <Badge tone={v.tone}>{v.text}</Badge>
                    : <span style={{ color: T.inkFaint, fontSize: FZ.tiny }}>{v.text}</span>}
                </span>
              </div>
              {sel && (
                <>
                  <div style={{ width: "100%", display: "flex", gap: SPACE.sm, padding: "3px 0 0 28px" }}>
                    {([[1, "out ▶ export", DIR_OUT, g.out_volume], [0, "◀ in import", DIR_IN, g.in_volume]] as const).map(([d, lbl, col, vol]) => (
                      <div
                        key={d}
                        data-no-drag
                        onClick={(e) => { e.stopPropagation(); setSelGood(g.good); setSelDir(selDir === d ? null : d); setSelPartner(null); setSelRoute(null); }}
                        style={{
                          flex: 1, display: "flex", alignItems: "center", gap: 5, padding: "2px 6px",
                          borderRadius: RADIUS.sm, cursor: "pointer",
                          background: selDir === d ? T.card : "transparent",
                          border: `1px solid ${selDir === d ? col : T.lineSoft}`,
                        }}
                      >
                        <span style={{ color: col, fontSize: FZ.tiny }}>{lbl}</span>
                        <span style={{ flex: 1 }} />
                        <span style={{ color: T.inkMid, fontSize: FZ.tiny }}>{fmt(vol)}</span>
                      </div>
                    ))}
                  </div>
                  <div style={{ width: "100%", display: "flex", alignItems: "center", gap: SPACE.md, padding: "4px 0 2px 28px" }}>
                    <Spark vals={g.history} />
                    <span style={{ color: T.inkDim, fontSize: FZ.tiny }}>
                      last year {fmt(g.last_volume)} · {g.history.length}-year trend
                    </span>
                  </div>
                  {/* HOW IT TRAVELS + WHO CARRIES IT. Both are read straight off
                      state the tick already had per shipment. "Who carries it" is
                      the monopoly question: one house moving nearly all of a good
                      IS a monopoly on it, whatever the charter says, and that is
                      worth flagging rather than leaving to be read off a list. */}
                  <div style={{ width: "100%", padding: "2px 0 4px 28px" }}>
                    {(() => {
                      const tr = transportOf(g.sea_volume ?? 0, g.last_volume);
                      const top = g.carriers?.[0];
                      return (
                        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: SPACE.sm }}>
                          {tr && (
                            <Badge tone="neutral">{tr.icon} {tr.label}</Badge>
                          )}
                          {top && top.pct >= 60 && (
                            <Badge tone={top.house < 0 ? "neutral" : "warn"}>
                              {top.is_guild ? "🏛" : "⚜"} {top.name} carries {top.pct.toFixed(0)}%
                            </Badge>
                          )}
                        </div>
                      );
                    })()}
                    {g.carriers && g.carriers.length > 0 && (
                      <div style={{ marginTop: SPACE.sm }}>
                        <div style={{ color: T.inkFaint, fontSize: FZ.micro, marginBottom: 2 }}>
                          WHO CARRIES IT
                        </div>
                        {g.carriers.map((c) => (
                          <div key={`${c.house}:${c.name}`}
                            style={{ display: "flex", alignItems: "center", gap: SPACE.sm, padding: "1px 0" }}>
                            <span style={{ width: 14, fontSize: FZ.tiny }}>
                              {c.house < 0 ? "·" : c.is_guild ? "🏛" : "⚜"}
                            </span>
                            <span style={{
                              flex: 1, minWidth: 60, color: c.house < 0 ? T.inkDim : T.ink,
                              fontSize: FZ.tiny, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                            }}>{c.name}</span>
                            <Meter value={c.pct} max={100} color={c.is_guild ? "#7fb2d8" : "#c99a3a"} height={5} />
                            <span style={{ width: 34, textAlign: "right", color: T.inkMid, fontSize: FZ.tiny }}>
                              {c.pct.toFixed(0)}%
                            </span>
                          </div>
                        ))}
                        <FootNote>
                          ⚜ house · 🏛 guild · · unnamed local merchants. A single carrier
                          above 60% is an effective monopoly on this good here.
                        </FootNote>
                      </div>
                    )}
                  </div>
                </>
              )}
            </div>
          );
        })}
        <FootNote>Click a good to map its routes. Bar shows the in/out split; length is total volume.</FootNote>
      </Section>

      {/* ── Selected good's routes ───────────────────────────────────────────── */}
      {selGood != null && (
        <Section
          title={`${selDir === 1 ? "Export routes" : selDir === 0 ? "Import routes" : "Routes"} — ${
            GOOD_META.get(flows.goods.find((x) => x.good === selGood)?.name ?? "")?.label ?? "good"}`}
        >
          {goodRoutes.length === 0 && (() => {
            const g = flows.goods.find((x) => x.good === selGood);
            if (g && g.avg_volume > 0) {
              return (
                <EmptyNote>
                  Not traded last year — {g.history.length}-year average {fmt(g.avg_volume)}/yr.
                  Per-partner routes are recorded for the most recent trading year only.
                </EmptyNote>
              );
            }
            return <EmptyNote>No routed flows recorded.</EmptyNote>;
          })()}
          {goodRoutes.slice(0, 8).map((r, i) => {
            const isSel = !!selRoute && selRoute.good === r.good && selRoute.partner === r.partner && selRoute.dir === r.dir;
            const col = r.dir === 0 ? DIR_IN : DIR_OUT;
            return (
              <div
                key={i}
                data-no-drag
                onClick={() => setSelRoute(isSel ? null : { good: r.good, partner: r.partner, dir: r.dir })}
                style={{
                  display: "flex", alignItems: "center", gap: SPACE.sm, padding: "3px 4px",
                  cursor: "pointer", borderRadius: RADIUS.sm,
                  background: isSel ? T.card : "transparent",
                  borderLeft: `2px solid ${isSel ? col : "transparent"}`,
                }}
              >
                <span style={{ width: 34, color: col, fontSize: FZ.tiny }}>{r.dir === 0 ? "◀ in" : "out ▶"}</span>
                <span style={{
                  flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                  color: isSel ? T.gold : T.ink,
                }}>
                  {r.dir === 0 ? `${r.partner_name} → here` : `here → ${r.partner_name}`}
                </span>
                <span style={{ width: 20, textAlign: "center", fontSize: FZ.tiny }}
                  title={(r.sea_amount ?? 0) > r.amount * 0.5 ? "carried by sea" : "carried overland"}>
                  {(r.sea_amount ?? 0) > r.amount * 0.5 ? "⛵" : "🐫"}
                </span>
                <span style={{ width: 50, textAlign: "right", color: T.inkMid }}>{fmt(r.amount)}</span>
                <span style={{ width: 38, textAlign: "right", color: T.inkDim }}>{r.pct.toFixed(0)}%</span>
              </div>
            );
          })}
          {goodRoutes.length > 0 && <FootNote>Click a route to isolate it on the map.</FootNote>}
        </Section>
      )}

      {/* ── Partner cities ───────────────────────────────────────────────────── */}
      <Section title="Partner cities">
        {/* CONCENTRATION, not just share: a partner's percentage is on screen today,
            but the question a trading city actually has is whether losing one of
            them would hurt. Only shown when it genuinely would. */}
        {topPartner && topPartner.pct >= 30 && (
          <Card style={{ marginBottom: SPACE.sm }}>
            <span style={{ color: "#e0b45a", fontSize: FZ.tiny }}>
              ⚠ {topPartner.pct.toFixed(0)}% of all trade runs through {topPartner.name} — losing that
              partner would take most of this city's commerce with it.
            </span>
          </Card>
        )}
        {flows.partners.map((p) => {
          const sel = selPartner === p.hub;
          return (
            <div
              key={p.hub}
              data-no-drag
              onClick={() => { setSelPartner(sel ? null : p.hub); setSelGood(null); setSelRoute(null); }}
              style={{
                display: "flex", alignItems: "center", gap: SPACE.sm, padding: "3px 4px",
                borderRadius: RADIUS.sm, cursor: "pointer", background: sel ? T.card : "transparent",
              }}
            >
              <span style={{
                flex: 1, minWidth: 70, color: sel ? T.gold : T.ink,
                overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
              }}>{p.name}</span>
              <Meter value={p.pct} max={maxPartner} color="#c99a3a" height={7} />
              <span style={{ width: 36, textAlign: "right", color: T.inkMid }}>{p.pct.toFixed(0)}%</span>
              <span style={{
                width: 96, textAlign: "right", color: T.inkDim, fontSize: FZ.base,
                overflow: "hidden", whiteSpace: "nowrap",
              }}>
                {p.goods.map((gn) => GOOD_META.get(gn)?.emoji ?? "").join("")}
              </span>
            </div>
          );
        })}
        <FootNote>Share of all trade. Click a city to map every route to it.</FootNote>
      </Section>
    </div>
  );
}
