/** Trade ▸ Traders subtab (docs/CITY_TRADERS_PANEL_PLAN.md) — the third view beside
 *  Market and Flows, answering two questions neither of those does: WHO MOVES CARGO
 *  through this city, and WHO IS ESTABLISHED here (an office, a bailo, the council
 *  seat, capture) whether or not they carried anything. They are deliberately two
 *  lists because they routinely disagree — a house can seat a council and move no
 *  cargo at all.
 *
 *  The finding this panel exists to surface (§0 of the plan):
 *  `econ_measure_carrier_mix` measures **~96% of all shipments move on no house's
 *  account** — the "ownerless residual" of local merchants. That is not a defect to
 *  design around; it is the model's true state, and the rule for this panel is to
 *  NEVER suppress the residual to make the house list look better (§0).
 *
 *  ── LAYOUT (2026-09 redesign) ────────────────────────────────────────────────
 *  The first cut packed nine fields into two cramped flex lines per row, all at
 *  one weight and one colour, so nothing lined up down the page and the eye could
 *  not tell a great carrier from a marginal one. Three rules now hold it together:
 *
 *  · **A table, not flex-packed spans.** One `grid-template-columns` string is
 *    shared by the `ColHead` and every `DataRow` under it (`@ui/kit`), so the
 *    numbers form real columns a reader compares DOWNWARD, and each column is
 *    labelled — the same move `CityMarketView`'s "on stall / cover / price"
 *    header already makes.
 *  · **Two zones: what a row IS, and what it DOES.** Identity (mark, name,
 *    standing) sits left of a hairline; carriage (share, direction, mode, route,
 *    re-export) sits right of it. The rule runs the whole height of the list.
 *  · **Weight follows VOLUME, never the current sort.** The top three named
 *    carriers read stronger than the tail — but they are picked by how much they
 *    actually move, so re-ranking by route length cannot promote a trivial house
 *    to hero type. The residual is not ranked among them at all: it gets its own
 *    dotted rail and its own voice, because it is not a house.
 *
 *  Nothing was dropped to make room. The per-good breakdown moved from a
 *  permanently-cramped second line into a click-to-open detail card, where it
 *  gains the columns it never had (brought in · sent out · total), the full
 *  standing badges, and a sea/river/overland split that was never shown at all. */
import { useEffect, useMemo, useState } from "react";
import { GoodIcon } from "@ui/goods/GoodIcon";
import type { CSSProperties, ReactNode } from "react";
import { campaignTradeFlows } from "@bridge";
import type { TradeFlows, CityTrader, CityEstablished } from "@types";
import { GOOD_DEFS } from "@goods";
import {
  Section, Card, Badge, Meter, Chip, EmptyNote, FootNote, StatGrid, Stat, SplitBar,
  ColHead, DataRow, NumCell,
} from "@ui/kit";
import { T, SPACE, FZ, RADIUS, SERIF } from "@ui/campaign/chronicleTheme";

const GOOD_META = new Map(GOOD_DEFS.map((g) => [g.name, g]));
function fmt(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(n >= 10000 ? 0 : 1)}k`;
  return n.toFixed(n >= 100 ? 0 : 1);
}
/** A house's own share, at real precision. Against a 90%+ ownerless residual
 *  (the panel's own deliberate headline stat — never suppressed, see the
 *  file doc comment) every individual house routinely holds under 1%, and
 *  `toFixed(0)` flattened every one of them to a misleading "0%" — a wall of
 *  zeroes that reads as "this house trades nothing" when it may carry real,
 *  measurable volume. One decimal below 10%, and "<0.1%" rather than a false
 *  "0.0%" for a share too small for even that digit to show. */
function fmtPct(pct: number): string {
  if (pct <= 0) return "0%";
  if (pct < 0.1) return "<0.1%";
  if (pct < 10) return `${pct.toFixed(1)}%`;
  return `${pct.toFixed(0)}%`;
}

type Rank = "volume" | "standing" | "route" | "carriage";
type Dir = "all" | "import" | "export";
type Carr = "all" | "sea" | "overland";

/** THE COLUMN GRID. One string, shared by the header and every row, which is the
 *  whole reason the numbers line up. Identity left of the rule, carriage right:
 *      ▸  mark  name+standing │ share  in⇄out  carriage  route  re-exp          */
const TRADER_COLS = "12px 15px minmax(84px,1fr) 56px 100px 58px 62px 56px";
/** The established list is a different question (standing, not volume), so it
 *  gets its own narrower grid rather than being forced into the one above. */
const EST_COLS = "15px minmax(80px,1fr) 238px 92px";
/** The per-good table inside an opened trader. */
const GOOD_COLS = "minmax(90px,1fr) 62px 62px 62px";

/** The vertical rule that separates "what this row IS" from "what it DOES".
 *  Applied to the first carriage cell of the header and of every row, which —
 *  because rows are flush — draws one continuous line down the list. */
const ZONE_RULE: CSSProperties = { borderLeft: `1px solid ${T.lineSoft}`, paddingLeft: SPACE.sm };

/** Standing rank, the same hierarchy the backend sorts `established` by:
 *  captor > council seat > bailo > office > nothing. */
function standingRank(t: { has_office: boolean; has_bailo: boolean; seats_council: boolean; is_captor: boolean }): number {
  return (t.is_captor ? 8 : 0) + (t.seats_council ? 4 : 0) + (t.has_bailo ? 2 : 0) + (t.has_office ? 1 : 0);
}

type Standing = { has_office: boolean; has_bailo: boolean; seats_council: boolean; is_captor: boolean };

/** The office/bailo/seat/captor vocabulary — the same words the House Dossier and
 *  the Established list use, so a reader learns each one once.
 *
 *  `top` shows only the HIGHEST standing, for the traders list where standing is
 *  secondary to carriage and the full set would crowd out the name. Nothing is
 *  lost: the whole set is in the row's hover title, in the row's opened detail,
 *  and in full in the Established list below, which is the view standing is
 *  actually the subject of. */
function StandingBadges({ t, top }: { t: Standing; top?: boolean }) {
  if (standingRank(t) === 0) return null;
  const all = [
    t.is_captor && <Badge key="c" tone="bad">seized by force</Badge>,
    t.seats_council && <Badge key="s" tone="gold">seats the council</Badge>,
    t.has_bailo && <Badge key="b" tone="accent">🏛 bailo</Badge>,
    t.has_office && <Badge key="o" tone="neutral">office</Badge>,
  ].filter(Boolean);
  return (
    <span style={{ display: "flex", gap: 3, flexWrap: "wrap", minWidth: 0 }}>
      {top ? all[0] : all}
    </span>
  );
}

/** Every standing a holder has, as one plain phrase — the hover title for a row
 *  showing only its highest badge. */
function standingWords(t: Standing): string {
  const w: string[] = [];
  if (t.is_captor) w.push("seized by force");
  if (t.seats_council) w.push("seats the council");
  if (t.has_bailo) w.push("holds a bailo");
  if (t.has_office) w.push("holds an office");
  return w.length > 0 ? w.join(" · ") : "no standing here";
}

/** Carriage as ONE aligned figure: the dominant mode's icon and its share. The
 *  old cell printed a bare icon (or two icons at once), which carried no
 *  magnitude and could not be compared with the row above it. */
function carriageOf(t: CityTrader): { icon: string; pct: string; label: string } {
  if (t.volume <= 0) return { icon: "·", pct: "", label: "carried nothing" };
  const seaPct = (t.sea_volume / t.volume) * 100;
  const label = `${seaPct.toFixed(0)}% by sea · ${(100 - seaPct).toFixed(0)}% overland`;
  return seaPct >= 50
    ? { icon: "⛵", pct: `${seaPct.toFixed(0)}%`, label }
    : { icon: "🐫", pct: `${(100 - seaPct).toFixed(0)}%`, label };
}

function Caret({ open }: { open: boolean }) {
  return <span style={{ color: T.inkFaint, fontSize: FZ.micro }}>{open ? "▾" : "▸"}</span>;
}

export function TradersView({ hubId, active, tick }: { hubId: number; active: boolean; tick: number }) {
  const [flows, setFlows] = useState<TradeFlows | null>(null);
  const [loading, setLoading] = useState(false);
  const [rank, setRank] = useState<Rank>("volume");
  const [dir, setDir] = useState<Dir>("all");
  const [carr, setCarr] = useState<Carr>("all");
  const [open, setOpen] = useState<string | null>(null);
  const [whyOpen, setWhyOpen] = useState(false);

  useEffect(() => {
    if (!active) { setFlows(null); setLoading(false); return; }
    let alive = true;
    setLoading(true);
    campaignTradeFlows(hubId)
      .then((f) => { if (alive) { setFlows(f); setLoading(false); } })
      .catch(() => { if (alive) { setFlows(null); setLoading(false); } });
    return () => { alive = false; };
  }, [hubId, active, tick]);

  useEffect(() => { setDir("all"); setCarr("all"); setWhyOpen(false); setOpen(null); }, [hubId]);

  // ── Filter, then rank. Filters narrow the list to traders that genuinely have
  // that kind of trade (real fields — in_volume/out_volume, sea_volume — never a
  // fabricated per-direction sea split). §3's rule: sorting reorders rows only;
  // a FILTER narrows the list, and the header must say a filter is active rather
  // than silently reporting a subtotal as the whole.
  const rows = useMemo(() => {
    if (!flows) return [];
    let list = flows.traders;
    if (dir === "import") list = list.filter((t) => t.in_volume > 0);
    else if (dir === "export") list = list.filter((t) => t.out_volume > 0);
    if (carr === "sea") list = list.filter((t) => t.sea_volume > 0);
    else if (carr === "overland") list = list.filter((t) => t.volume - t.sea_volume > 0);
    const sorted = [...list];
    sorted.sort((a, b) => {
      switch (rank) {
        case "standing": {
          const d = standingRank(b) - standingRank(a);
          return d !== 0 ? d : b.volume - a.volume;
        }
        case "route": return b.mean_route_km - a.mean_route_km;
        case "carriage": {
          const sa = a.volume > 0 ? a.sea_volume / a.volume : 0;
          const sb = b.volume > 0 ? b.sea_volume / b.volume : 0;
          return sb - sa;
        }
        default: return b.volume - a.volume;
      }
    });
    return sorted;
  }, [flows, rank, dir, carr]);

  const filterActive = dir !== "all" || carr !== "all";

  /** The three largest NAMED carriers, by volume and by volume only — so the
   *  visual emphasis says "these move the most cargo" whatever the list is
   *  currently sorted by. Sorting by route length must not be able to dress a
   *  marginal house up as a major one. The residual is excluded: it is not a
   *  house competing for rank, and it carries its own distinct treatment. */
  const majors = useMemo(() => {
    if (!flows) return new Set<string>();
    return new Set(
      flows.traders
        .filter((t) => t.house >= 0 && t.volume > 0)
        .sort((a, b) => b.volume - a.volume)
        .slice(0, 3)
        .map((t) => `${t.house}:${t.name}`),
    );
  }, [flows]);

  /** Standing-vs-carriage is the whole reason the two lists exist, so each
   *  established holder is matched against the carrier list once, here. */
  const carriedBy = useMemo(() => {
    const m = new Map<number, number>();
    for (const t of flows?.traders ?? []) if (t.house >= 0) m.set(t.house, t.pct);
    return m;
  }, [flows]);

  const totals = useMemo(() => {
    if (!flows) return null;
    let carriedIn = 0, carriedOut = 0, reexport = 0;
    for (const t of flows.traders) { carriedIn += t.in_volume; carriedOut += t.out_volume; reexport += t.reexport; }
    const residual = flows.traders.find((t) => t.house < 0);
    const residualPct = residual ? residual.pct : 0;
    const named = flows.traders.filter((t) => t.house >= 0 && t.volume > 0).length;
    return { carriedIn, carriedOut, reexport, residualPct, named };
  }, [flows]);

  if (!active) return <EmptyNote>Traders appear once a campaign is running.</EmptyNote>;
  if (loading && !flows) return <EmptyNote>Loading traders…</EmptyNote>;
  if (!flows) return <EmptyNote>No trade data for this settlement yet.</EmptyNote>;
  if (flows.traders.length === 0) {
    return <EmptyNote>No trade recorded yet — let a campaign year or two pass.</EmptyNote>;
  }

  const maxVol = Math.max(...rows.map((t) => t.volume), 1e-6);
  const w = flows.carrier_why;
  const noStanding = flows.established.filter((e) => e.volume <= 0).length;

  return (
    <div style={{ fontSize: FZ.body, color: T.ink }}>
      {/* ── THE FINDING, up front and never hidden (§0). It gets the largest
            type on the page, a plain-language second line, and a bar so the
            proportion reads before any number is parsed. ─────────────────── */}
      {totals && (
        <Card style={{ marginBottom: SPACE.lg, borderColor: T.lineGold }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: SPACE.sm, flexWrap: "wrap" }}>
            <span style={{ fontFamily: SERIF, fontSize: 26, lineHeight: 1.1, color: T.gold, fontWeight: 700 }}>
              {totals.residualPct.toFixed(0)}%
            </span>
            <span style={{ fontFamily: SERIF, fontSize: FZ.head, color: T.ink }}>
              of trade here moves on no house&apos;s account
            </span>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: SPACE.sm, margin: `${SPACE.sm}px 0 2px` }}>
            <Meter value={totals.residualPct} max={100} color={T.inkFaint} track={T.gold} height={9} />
          </div>
          <div style={{ display: "flex", gap: SPACE.md, fontSize: FZ.tiny, color: T.inkDim, marginBottom: SPACE.md }}>
            <span><span style={{ color: T.inkFaint }}>■</span> unnamed local merchants</span>
            <span><span style={{ color: T.gold }}>■</span> {totals.named} named house{totals.named === 1 ? "" : "s"} &amp; guild{totals.named === 1 ? "" : "s"}</span>
          </div>
          <StatGrid cols={4}>
            <Stat label="Carried in" value={fmt(totals.carriedIn)} />
            <Stat label="Carried out" value={fmt(totals.carriedOut)} />
            <Stat label="Re-exported" value={fmt(totals.reexport)} hint="landed & shipped on" />
            <Stat label="Made here" value={fmt(flows.produced_here)} hint={`consumed ${fmt(flows.consumed_here)}`} />
          </StatGrid>
          <FootNote>
            {flows.traders.length} trader{flows.traders.length === 1 ? "" : "s"} moved cargo here ·
            {" "}{flows.established.length} hold standing here
          </FootNote>
        </Card>
      )}

      {/* ── WHO TRADES HERE ─────────────────────────────────────────────────── */}
      <Section
        title="Who trades here"
        right={filterActive ? <Badge tone="accent">{rows.length} of {flows.traders.length} shown</Badge> : undefined}
      >
        {/* Controls, on one line each and labelled, so a chip's job is obvious
            without reading the list to work out what changed. */}
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: SPACE.sm, marginBottom: SPACE.xs }}>
          <span style={{ width: 34, color: T.inkFaint, fontSize: FZ.tiny, textTransform: "uppercase", letterSpacing: 0.5 }}>rank</span>
          {([["volume", "volume"], ["standing", "standing"], ["route", "route length"], ["carriage", "carriage"]] as const).map(([v, l]) => (
            <Chip key={v} on={rank === v} onClick={() => setRank(v)}>{l}</Chip>
          ))}
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: SPACE.sm, marginBottom: SPACE.md }}>
          <span style={{ width: 34, color: T.inkFaint, fontSize: FZ.tiny, textTransform: "uppercase", letterSpacing: 0.5 }}>show</span>
          {([["all", "all"], ["import", "imports"], ["export", "exports"]] as const).map(([v, l]) => (
            <Chip key={v} on={dir === v} onClick={() => setDir(v)}>{l}</Chip>
          ))}
          <span style={{ width: 1, alignSelf: "stretch", background: T.lineSoft, margin: "0 2px" }} />
          {([["all", "all"], ["sea", "⛵ sea"], ["overland", "🐫 overland"]] as const).map(([v, l]) => (
            <Chip key={v} on={carr === v} onClick={() => setCarr(v)}>{l}</Chip>
          ))}
        </div>

        <ColHead cols={TRADER_COLS}>
          <span />
          <span />
          <span>trader</span>
          <span style={{ ...ZONE_RULE, textAlign: "right" }}>share</span>
          <span>in ⇄ out</span>
          <span style={{ textAlign: "right" }}>carriage</span>
          <span style={{ textAlign: "right" }}>route</span>
          <span style={{ textAlign: "right" }}>re-exp</span>
        </ColHead>

        {rows.map((t, i) => {
          const c = carriageOf(t);
          const isResidual = t.house < 0;
          const key = `${t.house}:${t.name}`;
          const isOpen = open === key;
          const isMajor = majors.has(key);
          // The house's OWN colour — the same `distinct_color(index)` its shipment
          // swatches, its offices and its lanes on the map already use, so a family
          // is one identity everywhere. A left rail carries it rather than the text,
          // which keeps names legible at any hue.
          const tint = t.color ?? (isResidual ? T.inkFaint : t.is_guild ? "#7fb2d8" : "#c99a3a");
          const nameInk = isResidual ? T.inkDim : isMajor ? tint : T.inkMid;
          return (
            <div key={key}>
              <DataRow
                cols={TRADER_COLS}
                rail={tint}
                railDashed={isResidual}
                zebra={i % 2 === 1}
                selected={isOpen}
                onClick={() => setOpen(isOpen ? null : key)}
                title={`${t.name} — ${standingWords(t)}. ${c.label}. Click for what it moves here.`}
              >
                <Caret open={isOpen} />
                <span style={{ fontSize: FZ.tiny }}>{isResidual ? "·" : t.is_guild ? "🏛" : "⚜"}</span>
                {/* ZONE A — what this row IS: mark, name, standing. */}
                <span style={{ display: "flex", alignItems: "center", gap: SPACE.sm, minWidth: 0 }}>
                  <span style={{
                    minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                    color: nameInk,
                    fontSize: isMajor ? FZ.base : FZ.body,
                    fontWeight: isMajor ? 700 : 400,
                    fontStyle: isResidual ? "italic" : undefined,
                  }}>{t.name}</span>
                  <StandingBadges t={t} top />
                </span>
                {/* ZONE B — what it DOES. The share is the row's headline number,
                    so it is the only one carrying weight; the rest stay quiet. */}
                <NumCell
                  style={ZONE_RULE}
                  strong={isMajor || isResidual}
                  tone={isResidual ? T.gold : isMajor ? T.ink : T.inkMid}
                >{fmtPct(t.pct)}</NumCell>
                {/* The BAR carries direction, not just size: a two-tone in/out split
                    says whether this house supplies the city or buys it out, which a
                    single-colour volume bar cannot. */}
                <SplitBar inV={t.in_volume} outV={t.out_volume} max={maxVol} height={7} />
                <NumCell tone={T.inkMid}>
                  <span style={{ fontSize: FZ.tiny }}>{c.icon}</span> {c.pct}
                </NumCell>
                <NumCell dim>{t.mean_route_km > 0 ? `${fmt(t.mean_route_km)} km` : "—"}</NumCell>
                <NumCell dim>{t.reexport > 0 ? fmt(t.reexport) : "—"}</NumCell>
              </DataRow>
              {isOpen && <TraderDetail t={t} />}
            </div>
          );
        })}

        <FootNote>
          ⚜ house · 🏛 guild · <span style={{ fontStyle: "italic" }}>·</span> unnamed local merchants, the real
          trading capacity of this city. The rail is the holder&apos;s own colour, the same one its lanes carry
          on the map. The bar reads <span style={{ color: "#5fd0ff" }}>brought in</span> against{" "}
          <span style={{ color: "#ffce5f" }}>sent out</span>, to one scale down the list. Click any row for
          what it moves.
        </FootNote>
      </Section>

      {/* ── WHO IS ESTABLISHED HERE — standing whether or not it carries ─────── */}
      <Section
        title="Who is established here"
        right={noStanding > 0
          ? <Badge tone="neutral">{noStanding} carr{noStanding === 1 ? "ies" : "y"} nothing</Badge>
          : undefined}
      >
        <FootNote style={{ marginTop: 0, marginBottom: SPACE.sm }}>
          A separate list on purpose: standing and carriage routinely disagree — a house can seat the
          council here and move no cargo at all.
        </FootNote>
        {flows.established.length === 0
          ? <EmptyNote>No house or guild holds an office, a bailo, or the council seat here.</EmptyNote>
          : (
            <>
              <ColHead cols={EST_COLS}>
                <span />
                <span>holder</span>
                <span style={ZONE_RULE}>standing here</span>
                <span style={{ textAlign: "right" }}>carries here</span>
              </ColHead>
              {flows.established.map((e: CityEstablished, i) => {
                const pct = carriedBy.get(e.house);
                return (
                  <DataRow
                    key={`${e.house}:${e.name}`}
                    cols={EST_COLS}
                    rail={e.color ?? T.lineSoft}
                    zebra={i % 2 === 1}
                    title={`${e.name} — ${standingWords(e)}. ${
                      e.volume > 0
                        ? `Carries ${fmt(e.volume)} here${pct !== undefined ? ` (${fmtPct(pct)} of this city's trade)` : ""}.`
                        : "Moved no cargo here this year."}`}
                  >
                    <span style={{ fontSize: FZ.tiny }}>{e.is_guild ? "🏛" : "⚜"}</span>
                    <span style={{
                      minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                      color: e.volume > 0 ? T.ink : T.inkMid,
                    }}>{e.name}</span>
                    <span style={ZONE_RULE}><StandingBadges t={e} /></span>
                    <NumCell tone={e.volume > 0 ? T.inkMid : T.inkFaint} strong={e.volume > 0}>
                      {e.volume > 0 ? fmt(e.volume) : "nothing"}
                    </NumCell>
                  </DataRow>
                );
              })}
            </>
          )}
      </Section>

      {/* ── The world-wide "why" note, folded away by default (decision 4) ─────── */}
      <div
        data-no-drag
        onClick={() => setWhyOpen((v) => !v)}
        style={{ cursor: "pointer", color: T.inkDim, fontSize: FZ.tiny, padding: "3px 4px", userSelect: "none" }}
      >
        <Caret open={whyOpen} />{" "}
        why {(100 - (w.shipments > 0 ? (w.by_house / w.shipments) * 100 : 0)).toFixed(0)}% moves on no house&apos;s account (world-wide)
      </div>
      {whyOpen && w.shipments > 0 && (
        <Card style={{ marginTop: SPACE.xs }}>
          <div style={{ fontSize: FZ.tiny, color: T.inkMid, marginBottom: SPACE.sm }}>
            of {w.shipments.toLocaleString()} shipments, {w.by_house.toLocaleString()} were financed by a house.
            the rest went ownerless because:
          </div>
          <ColHead cols="minmax(120px,1fr) 72px 44px">
            <span>reason</span>
            <span style={{ textAlign: "right" }}>shipments</span>
            <span style={{ textAlign: "right" }}>share</span>
          </ColHead>
          {([
            ["no house at either end", w.why_nohouse],
            ["no free vessel", w.why_slot],
            ["could not afford it", w.why_cash],
            ["barred from the market", w.why_barred],
          ] as const).map(([label, n], i) => (
            <DataRow key={label} cols="minmax(120px,1fr) 72px 44px" zebra={i % 2 === 1}>
              <span style={{ color: T.inkDim, fontSize: FZ.tiny }}>{label}</span>
              <NumCell>{n.toLocaleString()}</NumCell>
              <NumCell dim>{w.ownerless > 0 ? `${((n / w.ownerless) * 100).toFixed(0)}%` : "0%"}</NumCell>
            </DataRow>
          ))}
          <FootNote>
            These counters are world-wide, not this city&apos;s — the sim keeps them globally, and
            attributing them to one place would be inventing a measurement never taken.
          </FootNote>
        </Card>
      )}
    </div>
  );
}

/** One opened trader: WHAT it moves here, in the columns the cramped inline line
 *  could never give it, plus the facts that used to fight the row for space.
 *
 *  `good_rows` carries the per-(trader, good) totals the row above is folded
 *  from; the whole list is shown here rather than the first four, because an
 *  expansion is exactly the place completeness belongs. A save from before the
 *  split falls back to the bare good list, which is all it has. */
function TraderDetail({ t }: { t: CityTrader }) {
  const goods = t.good_rows ?? [];
  const sea = t.sea_volume;
  const river = t.river_volume ?? 0;
  const land = Math.max(0, t.volume - sea - river);
  const Line = ({ k, children }: { k: string; children: ReactNode }) => (
    <div style={{ display: "flex", gap: SPACE.md, fontSize: FZ.tiny, padding: "1px 0", alignItems: "baseline" }}>
      <span style={{ flex: "0 0 86px", color: T.inkFaint, textTransform: "uppercase", letterSpacing: 0.4, fontSize: FZ.micro }}>{k}</span>
      <span style={{ flex: 1, minWidth: 0, color: T.inkMid }}>{children}</span>
    </div>
  );
  return (
    <Card style={{ margin: `2px 0 ${SPACE.md}px 10px`, borderRadius: RADIUS.sm }}>
      <div style={{ color: T.gold, fontSize: FZ.small, fontWeight: 700, marginBottom: SPACE.xs }}>
        {t.name} — what it moves here
      </div>
      {goods.length > 0 ? (
        <>
          <ColHead cols={GOOD_COLS}>
            <span>good</span>
            <span style={{ textAlign: "right", color: "#5fd0ff" }}>brought in</span>
            <span style={{ textAlign: "right", color: "#ffce5f" }}>sent out</span>
            <span style={{ textAlign: "right" }}>total</span>
          </ColHead>
          {goods.map((g, i) => (
            <DataRow key={g.name} cols={GOOD_COLS} zebra={i % 2 === 1}>
              <span style={{ color: T.ink, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                <GoodIcon name={g.name} size={14} style={{ display: "inline-block", verticalAlign: "middle" }} /> {GOOD_META.get(g.name)?.label ?? g.name}
              </span>
              <NumCell tone={g.in_amount > 0 ? "#5fd0ff" : T.inkFaint}>{g.in_amount > 0 ? fmt(g.in_amount) : "—"}</NumCell>
              <NumCell tone={g.out_amount > 0 ? "#ffce5f" : T.inkFaint}>{g.out_amount > 0 ? fmt(g.out_amount) : "—"}</NumCell>
              <NumCell strong tone={T.ink}>{fmt(g.amount)}</NumCell>
            </DataRow>
          ))}
        </>
      ) : (
        <Line k="goods">
          {t.goods.length === 0
            ? <span style={{ color: T.inkFaint }}>nothing recorded</span>
            : t.goods.map((gn, i) => <span key={gn}>{i > 0 && " · "}<GoodIcon name={gn} size={14} style={{ display: "inline-block", verticalAlign: "middle" }} /> {GOOD_META.get(gn)?.label ?? gn}</span>)}
        </Line>
      )}
      <div style={{ marginTop: SPACE.sm }}>
        {/* The sea/river/overland split is shown here for the first time — the row
            can only carry the dominant mode, and `river_volume` had no reader at
            all. Omitted rather than printed as "0" where a mode is unused. */}
        <Line k="carriage">
          {t.volume > 0 ? [
            sea > 0 ? `⛵ ${fmt(sea)} by sea` : null,
            river > 0 ? `🛶 ${fmt(river)} by river` : null,
            land > 0 ? `🐫 ${fmt(land)} overland` : null,
          ].filter(Boolean).join(" · ") : <span style={{ color: T.inkFaint }}>carried nothing</span>}
        </Line>
        <Line k="route">
          {t.mean_route_km > 0
            ? <>{fmt(t.mean_route_km)} km to its partners, on average, weighted by volume</>
            : <span style={{ color: T.inkFaint }}>no route recorded</span>}
        </Line>
        <Line k="re-exported">
          {t.reexport > 0
            ? <>{fmt(t.reexport)} landed here and shipped on — a proxy for entrepôt trade, not a
                multi-leg voyage, which the sim does not model</>
            : <span style={{ color: T.inkFaint }}>nothing landed here was shipped on</span>}
        </Line>
        <Line k="standing">
          {standingRank(t) > 0 ? <StandingBadges t={t} /> : <span style={{ color: T.inkFaint }}>no standing here</span>}
        </Line>
      </div>
    </Card>
  );
}
