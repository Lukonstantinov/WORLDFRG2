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
import type { TradeFlows, TradeFlowGood, TradePartner } from "@types";
import { GOOD_DEFS } from "@goods";
import { Section, Card, Badge, Meter, Chip, EmptyNote, FootNote, StatGrid, Stat,
         Donut, DonutKey, SplitBar, type Slice } from "@ui/kit";
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

/** HOW A GOOD TRAVELS — sea / river / caravan, as a phrase plus the shares.
 *  `log_trade` is handed each shipment's `sea`/`river` flags and the yearly
 *  fold used to discard them, so "did this come by ship, barge, or caravan"
 *  was unanswerable a year later even though the tick knew it every day.
 *
 *  Three modes, because the sim genuinely tracks three: a leg is a SEA voyage
 *  when both ends are coastal, a RIVER voyage when neither is but both ends
 *  are river-connected (`TickHub.river`), and caravan otherwise — the true
 *  residual, `total - seaVol - riverVol`. */
function transportOf(seaVol: number, riverVol: number, total: number):
    { label: string; icon: string; seaPct: number; riverPct: number } | null {
  if (total <= 0) return null;
  const seaPct = Math.max(0, Math.min(1, seaVol / total)) * 100;
  const riverPct = Math.max(0, Math.min(1, riverVol / total)) * 100;
  const landPct = Math.max(0, 100 - seaPct - riverPct);
  if (seaPct >= 85) return { label: "by sea", icon: "⛵", seaPct, riverPct };
  if (riverPct >= 85) return { label: "by river", icon: "\u{1F6F6}", seaPct, riverPct };
  if (landPct >= 85) return { label: "overland", icon: "🐫", seaPct, riverPct };
  const parts: string[] = [];
  const icons: string[] = [];
  if (seaPct >= 15) { parts.push(`${seaPct.toFixed(0)}% by sea`); icons.push("⛵"); }
  if (riverPct >= 15) { parts.push(`${riverPct.toFixed(0)}% by river`); icons.push("\u{1F6F6}"); }
  if (landPct >= 15) { icons.push("🐫"); }
  return { label: parts.join(", ") || "mixed", icon: icons.join(""), seaPct, riverPct };
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

  // ── THE SHAPE OF THE TRADE ──────────────────────────────────────────────
  // Three part-of-a-whole readings a ranked bar list answers badly: what this
  // city sells, what it buys, and WHO carries any of it. All three are folds of
  // `flows.goods`, which is already on hand — no extra query, no new state.
  const shape = useMemo(() => {
    if (!flows) return null;
    const slice = (pick: (g: TradeFlowGood) => number): Slice[] =>
      flows.goods
        .map((g) => ({
          label: GOOD_META.get(g.name)?.label ?? g.name,
          value: pick(g),
          color: GOOD_META.get(g.name)?.color ?? T.inkDim,
        }))
        .filter((x) => x.value > 0);
    // Carriage is summed ACROSS goods, so it answers "who moves this city's
    // commerce" rather than "who moves its pepper". The residual keeps whatever
    // share it has earned — never merged away to flatter the house list
    // (docs/CITY_TRADERS_PANEL_PLAN.md §0).
    const carr = new Map<string, Slice>();
    for (const g of flows.goods) {
      for (const c of g.carriers ?? []) {
        const key = c.name;
        const row = carr.get(key) ?? {
          label: c.house < 0 ? "local merchants" : c.name,
          value: 0,
          color: c.color ?? (c.house < 0 ? "#8fbf8f" : T.inkDim),
        };
        row.value += c.amount;
        carr.set(key, row);
      }
    }
    const carriers = [...carr.values()].filter((x) => x.value > 0);
    const named = carriers.filter((c) => c.label !== "local merchants")
      .reduce((a, b) => a + b.value, 0);
    const carrTotal = carriers.reduce((a, b) => a + b.value, 0);
    return {
      out: slice((g) => g.out_volume),
      inn: slice((g) => g.in_volume),
      carriers,
      /** Share of carriage on a NAMED house or guild's account, 0..1. */
      namedShare: carrTotal > 0 ? named / carrTotal : 0,
    };
  }, [flows]);

  // Partners, ranked TWICE — once per direction. A city you depend on for grain
  // and a city you sell cloth to are different relationships and rank
  // differently; one combined list by total share cannot show either.
  const partnerCols = useMemo(() => {
    if (!flows) return null;
    const has = flows.partners.some((p) => (p.in_volume ?? 0) + (p.out_volume ?? 0) > 0);
    if (!has) return null;  // a pre-split save: fall back to the combined list
    const byIn = flows.partners.filter((p) => (p.in_volume ?? 0) > 0)
      .sort((a, b) => (b.in_volume ?? 0) - (a.in_volume ?? 0)).slice(0, 10);
    const byOut = flows.partners.filter((p) => (p.out_volume ?? 0) > 0)
      .sort((a, b) => (b.out_volume ?? 0) - (a.out_volume ?? 0)).slice(0, 10);
    return {
      byIn, byOut,
      maxIn: Math.max(...byIn.map((p) => p.in_volume ?? 0), 1e-6),
      maxOut: Math.max(...byOut.map((p) => p.out_volume ?? 0), 1e-6),
    };
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
                      const tr = transportOf(g.sea_volume ?? 0, g.river_volume ?? 0, g.last_volume);
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
                {(() => {
                  const routeTr = transportOf(r.sea_amount ?? 0, r.river_amount ?? 0, r.amount);
                  return (
                    <span style={{ width: 20, textAlign: "center", fontSize: FZ.tiny }}
                      title={routeTr ? `carried ${routeTr.label}` : "carried overland"}>
                      {routeTr?.icon || "🐫"}
                    </span>
                  );
                })()}
                <span style={{ width: 50, textAlign: "right", color: T.inkMid }}>{fmt(r.amount)}</span>
                <span style={{ width: 38, textAlign: "right", color: T.inkDim }}>{r.pct.toFixed(0)}%</span>
              </div>
            );
          })}
          {goodRoutes.length > 0 && <FootNote>Click a route to isolate it on the map.</FootNote>}
        </Section>
      )}

      {/* ── The shape of the trade ───────────────────────────────────────────
          Three donuts, because each is a PART OF ONE WHOLE — a question the
          ranked bar lists above answer badly. What we sell, what we buy, and who
          carries any of it. A donut is deliberately not used for the partner
          lists below: those are rankings, and a donut makes two similar slices
          genuinely hard to order. */}
      {shape && (shape.out.length > 0 || shape.inn.length > 0) && (
        <Section title="The shape of the trade">
          <div style={{ display: "flex", gap: SPACE.lg, flexWrap: "wrap", alignItems: "flex-start" }}>
            {shape.out.length > 0 && (
              <div style={{ display: "flex", gap: SPACE.md, alignItems: "center", minWidth: 250, flex: 1 }}>
                <Donut slices={shape.out} size={104} center={fmt(balance?.outV ?? 0)} sub="SOLD /yr" />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: DIR_OUT, fontSize: FZ.micro, letterSpacing: 0.6, marginBottom: 2 }}>
                    EXPORTS BY GOOD
                  </div>
                  <DonutKey slices={shape.out} fmt={fmt} onPick={(lbl) => {
                    const g = flows.goods.find((x) => (GOOD_META.get(x.name)?.label ?? x.name) === lbl);
                    setSelGood(g && selGood !== g.good ? g.good : null);
                    setSelDir(g && selGood !== g.good ? 1 : null);
                    setSelPartner(null); setSelRoute(null);
                  }} />
                </div>
              </div>
            )}
            {shape.inn.length > 0 && (
              <div style={{ display: "flex", gap: SPACE.md, alignItems: "center", minWidth: 250, flex: 1 }}>
                <Donut slices={shape.inn} size={104} center={fmt(balance?.inV ?? 0)} sub="BOUGHT /yr" />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: DIR_IN, fontSize: FZ.micro, letterSpacing: 0.6, marginBottom: 2 }}>
                    IMPORTS BY GOOD
                  </div>
                  <DonutKey slices={shape.inn} fmt={fmt} onPick={(lbl) => {
                    const g = flows.goods.find((x) => (GOOD_META.get(x.name)?.label ?? x.name) === lbl);
                    setSelGood(g && selGood !== g.good ? g.good : null);
                    setSelDir(g && selGood !== g.good ? 0 : null);
                    setSelPartner(null); setSelRoute(null);
                  }} />
                </div>
              </div>
            )}
            {shape.carriers.length > 0 && (
              <div style={{ display: "flex", gap: SPACE.md, alignItems: "center", minWidth: 250, flex: 1 }}>
                <Donut slices={shape.carriers} size={104}
                  center={`${(shape.namedShare * 100).toFixed(0)}%`} sub="ON ACCOUNT" />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: T.gold, fontSize: FZ.micro, letterSpacing: 0.6, marginBottom: 2 }}>
                    WHO CARRIES IT
                  </div>
                  <DonutKey slices={shape.carriers} fmt={fmt} />
                  {/* The residual is the model's true state, not a defect to design
                      around — but a city where almost nothing moves on a named
                      account is worth SAYING, because it is why houses feel weak. */}
                  {shape.namedShare < 0.25 && (
                    <FootNote>
                      Only {(shape.namedShare * 100).toFixed(0)}% of this city&apos;s trade moves on a
                      named house or guild&apos;s account — the rest is unattached local merchants.
                    </FootNote>
                  )}
                </div>
              </div>
            )}
          </div>
        </Section>
      )}

      {/* ── Partner cities, ranked once per DIRECTION ────────────────────────
          A supplier and a customer are different relationships. Ranked together
          by total share (the old single list) the import book is invisible in any
          city that exports much more than it buys — which is most of them. Each
          column is a share of ITS OWN book for the same reason. */}
      <Section title="Partner cities">
        {partnerCols ? (
          <div style={{ display: "flex", gap: SPACE.lg, flexWrap: "wrap" }}>
            {([
              ["in", "◀ WE BUY FROM", DIR_IN, partnerCols.byIn, partnerCols.maxIn,
               (p: TradePartner) => p.in_volume ?? 0, (p: TradePartner) => p.in_pct ?? 0],
              ["out", "WE SELL TO ▶", DIR_OUT, partnerCols.byOut, partnerCols.maxOut,
               (p: TradePartner) => p.out_volume ?? 0, (p: TradePartner) => p.out_pct ?? 0],
            ] as const).map(([key, title, tint, list, max, vol, pctOf]) => {
              const top = list[0];
              return (
                <div key={key} style={{ flex: 1, minWidth: 250 }}>
                  <div style={{ color: tint, fontSize: FZ.micro, letterSpacing: 0.6, marginBottom: 3 }}>
                    {title} · {fmt(list.reduce((a, b) => a + vol(b), 0))}/yr
                  </div>
                  {list.length === 0 && <EmptyNote>nothing {key === "in" ? "arrives" : "leaves"}</EmptyNote>}
                  {list.map((p) => {
                    const sel = selPartner === p.hub;
                    return (
                      <div key={p.hub} data-no-drag
                        onClick={() => { setSelPartner(sel ? null : p.hub); setSelGood(null); setSelRoute(null); }}
                        style={{
                          display: "flex", alignItems: "center", gap: SPACE.sm, padding: "2px 4px",
                          borderRadius: RADIUS.sm, cursor: "pointer",
                          background: sel ? T.card : "transparent",
                        }}>
                        <span style={{ flex: 1, minWidth: 60, color: sel ? T.gold : T.ink,
                          overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{p.name}</span>
                        <Meter value={vol(p)} max={max} color={tint} height={7} />
                        <span style={{ width: 46, textAlign: "right", color: T.inkMid,
                          fontVariantNumeric: "tabular-nums" }}>{fmt(vol(p))}</span>
                        <span style={{ width: 32, textAlign: "right", color: T.inkDim,
                          fontVariantNumeric: "tabular-nums" }}>{pctOf(p).toFixed(0)}%</span>
                      </div>
                    );
                  })}
                  {/* CONCENTRATION reads per direction now. A city can be perfectly
                      spread on exports and utterly dependent on one grain supplier —
                      and that second case is the one worth warning about. */}
                  {top && pctOf(top) >= 35 && (
                    <FootNote style={{ color: key === "in" ? "#e0b45a" : T.inkDim }}>
                      {key === "in"
                        ? `⚠ ${pctOf(top).toFixed(0)}% of everything this city buys lands from ${top.name} — losing that supplier would bite.`
                        : `${pctOf(top).toFixed(0)}% of exports go to ${top.name}.`}
                    </FootNote>
                  )}
                </div>
              );
            })}
          </div>
        ) : (
          <>
            {/* A save recorded before the direction split: show what it does have
                rather than an empty column. Never back-fill a direction the data
                never carried. */}
            {topPartner && topPartner.pct >= 30 && (
              <Card style={{ marginBottom: SPACE.sm }}>
                <span style={{ color: "#e0b45a", fontSize: FZ.tiny }}>
                  ⚠ {topPartner.pct.toFixed(0)}% of all trade runs through {topPartner.name} — losing that
                  partner would take most of this city&apos;s commerce with it.
                </span>
              </Card>
            )}
            {flows.partners.map((p) => {
              const sel = selPartner === p.hub;
              return (
                <div key={p.hub} data-no-drag
                  onClick={() => { setSelPartner(sel ? null : p.hub); setSelGood(null); setSelRoute(null); }}
                  style={{
                    display: "flex", alignItems: "center", gap: SPACE.sm, padding: "3px 4px",
                    borderRadius: RADIUS.sm, cursor: "pointer", background: sel ? T.card : "transparent",
                  }}>
                  <span style={{ flex: 1, minWidth: 70, color: sel ? T.gold : T.ink,
                    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{p.name}</span>
                  <Meter value={p.pct} max={maxPartner} color="#c99a3a" height={7} />
                  <span style={{ width: 36, textAlign: "right", color: T.inkMid }}>{p.pct.toFixed(0)}%</span>
                  <span style={{ width: 96, textAlign: "right", color: T.inkDim, fontSize: FZ.base,
                    overflow: "hidden", whiteSpace: "nowrap" }}>
                    {p.goods.map((gn) => GOOD_META.get(gn)?.emoji ?? "").join("")}
                  </span>
                </div>
              );
            })}
          </>
        )}
        <FootNote>Click a city to map every route to it.</FootNote>
      </Section>
    </div>
  );
}
