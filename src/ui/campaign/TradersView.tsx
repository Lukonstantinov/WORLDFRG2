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
 *  NEVER suppress the residual to make the house list look better (§0). */
import { useEffect, useMemo, useState } from "react";
import { campaignTradeFlows } from "@bridge";
import type { TradeFlows, CityTrader, CityEstablished } from "@types";
import { GOOD_DEFS } from "@goods";
import { Section, Card, Badge, Meter, Chip, EmptyNote, FootNote, StatGrid, Stat } from "@ui/kit";
import { T, SPACE, FZ, RADIUS, SERIF } from "@ui/campaign/chronicleTheme";

const GOOD_META = new Map(GOOD_DEFS.map((g) => [g.name, g]));
function fmt(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(n >= 10000 ? 0 : 1)}k`;
  return n.toFixed(n >= 100 ? 0 : 1);
}

type Rank = "volume" | "standing" | "route" | "carriage";
type Dir = "all" | "import" | "export";
type Carr = "all" | "sea" | "overland";

/** Standing rank, the same hierarchy the backend sorts `established` by:
 *  captor > council seat > bailo > office > nothing. */
function standingRank(t: { has_office: boolean; has_bailo: boolean; seats_council: boolean; is_captor: boolean }): number {
  return (t.is_captor ? 8 : 0) + (t.seats_council ? 4 : 0) + (t.has_bailo ? 2 : 0) + (t.has_office ? 1 : 0);
}

/** Standing badge — the same office/bailo/seat/captor vocabulary the House Dossier
 *  and the Established list both use, so a reader learns the icon once. */
function StandingBadges({ t }: { t: { has_office: boolean; has_bailo: boolean; seats_council: boolean; is_captor: boolean } }) {
  if (!t.has_office && !t.has_bailo && !t.seats_council && !t.is_captor) return null;
  return (
    <span style={{ display: "flex", gap: 3, flexWrap: "wrap" }}>
      {t.is_captor && <Badge tone="bad">seized by force</Badge>}
      {t.seats_council && <Badge tone="gold">seats the council</Badge>}
      {t.has_bailo && <Badge tone="accent">🏛 bailo</Badge>}
      {t.has_office && <Badge tone="neutral">office</Badge>}
    </span>
  );
}

function carriageOf(t: CityTrader): { icon: string; label: string } {
  if (t.volume <= 0) return { icon: "·", label: "" };
  const seaPct = (t.sea_volume / t.volume) * 100;
  if (seaPct >= 85) return { icon: "⛵", label: "by sea" };
  if (seaPct <= 15) return { icon: "🐫", label: "overland" };
  return { icon: "⛵🐫", label: `${seaPct.toFixed(0)}% sea` };
}

export function TradersView({ hubId, active, tick }: { hubId: number; active: boolean; tick: number }) {
  const [flows, setFlows] = useState<TradeFlows | null>(null);
  const [loading, setLoading] = useState(false);
  const [rank, setRank] = useState<Rank>("volume");
  const [dir, setDir] = useState<Dir>("all");
  const [carr, setCarr] = useState<Carr>("all");
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

  useEffect(() => { setDir("all"); setCarr("all"); setWhyOpen(false); }, [hubId]);

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

  const totals = useMemo(() => {
    if (!flows) return null;
    let carriedIn = 0, carriedOut = 0, reexport = 0;
    for (const t of flows.traders) { carriedIn += t.in_volume; carriedOut += t.out_volume; reexport += t.reexport; }
    const residual = flows.traders.find((t) => t.house < 0);
    const residualPct = residual ? residual.pct : 0;
    return { carriedIn, carriedOut, reexport, residualPct };
  }, [flows]);

  if (!active) return <EmptyNote>Traders appear once a campaign is running.</EmptyNote>;
  if (loading && !flows) return <EmptyNote>Loading traders…</EmptyNote>;
  if (!flows) return <EmptyNote>No trade data for this settlement yet.</EmptyNote>;
  if (flows.traders.length === 0) {
    return <EmptyNote>No trade recorded yet — let a campaign year or two pass.</EmptyNote>;
  }

  const maxVol = Math.max(...rows.map((t) => t.volume), 1e-6);
  const w = flows.carrier_why;

  return (
    <div style={{ fontSize: FZ.body, color: T.ink }}>
      {/* ── The finding: the residual, up front, never hidden ──────────────────── */}
      {totals && (
        <Card style={{ marginBottom: SPACE.lg }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: SPACE.md, marginBottom: SPACE.sm }}>
            <span style={{ fontFamily: SERIF, fontSize: FZ.head, color: T.gold, fontWeight: 700 }}>
              {totals.residualPct.toFixed(0)}% of trade here moves on no house's account
            </span>
          </div>
          <StatGrid cols={4}>
            <Stat label="Carried in" value={fmt(totals.carriedIn)} />
            <Stat label="Carried out" value={fmt(totals.carriedOut)} />
            <Stat label="Re-exported" value={fmt(totals.reexport)} hint="landed & shipped on" />
            <Stat label="Made here" value={fmt(flows.produced_here)} hint={`consumed ${fmt(flows.consumed_here)}`} />
          </StatGrid>
          <FootNote>
            {flows.traders.length} trader{flows.traders.length === 1 ? "" : "s"} · {flows.established.length} established here
          </FootNote>
        </Card>
      )}

      {/* ── Rank + filter controls ───────────────────────────────────────────── */}
      <Section
        title="Who trades here"
        right={filterActive ? <Badge tone="accent">{rows.length} of {flows.traders.length} shown</Badge> : undefined}
      >
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: SPACE.sm, marginBottom: SPACE.sm }}>
          <span style={{ color: T.inkFaint, fontSize: FZ.tiny }}>rank</span>
          {([["volume", "volume"], ["standing", "standing"], ["route", "route length"], ["carriage", "carriage"]] as const).map(([v, l]) => (
            <Chip key={v} on={rank === v} onClick={() => setRank(v)}>{l}</Chip>
          ))}
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: SPACE.sm, marginBottom: SPACE.md }}>
          <span style={{ color: T.inkFaint, fontSize: FZ.tiny }}>show</span>
          {([["all", "all"], ["import", "imports"], ["export", "exports"]] as const).map(([v, l]) => (
            <Chip key={v} on={dir === v} onClick={() => setDir(v)}>{l}</Chip>
          ))}
          <span style={{ width: 1, alignSelf: "stretch", background: T.lineSoft, margin: "0 2px" }} />
          {([["all", "all"], ["sea", "⛵ sea"], ["overland", "🐫 overland"]] as const).map(([v, l]) => (
            <Chip key={v} on={carr === v} onClick={() => setCarr(v)}>{l}</Chip>
          ))}
        </div>

        {rows.map((t) => {
          const c = carriageOf(t);
          const isResidual = t.house < 0;
          return (
            <div
              key={`${t.house}:${t.name}`}
              style={{
                display: "flex", flexDirection: "column", gap: 2, padding: "4px 4px",
                borderRadius: RADIUS.sm, borderBottom: `1px solid ${T.lineSoft}`,
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: SPACE.sm }}>
                <span style={{ width: 14, fontSize: FZ.tiny }}>{isResidual ? "·" : t.is_guild ? "🏛" : "⚜"}</span>
                <span style={{
                  flex: 1, minWidth: 70, color: isResidual ? T.inkDim : T.ink,
                  overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                }}>{t.name}</span>
                <Meter value={t.volume} max={maxVol} color={isResidual ? T.inkFaint : t.is_guild ? "#7fb2d8" : "#c99a3a"} height={7} style={{ maxWidth: 90 }} />
                <span style={{ width: 40, textAlign: "right", color: T.inkMid }}>{t.pct.toFixed(0)}%</span>
                <span style={{ width: 28, textAlign: "center", fontSize: FZ.tiny }} title={c.label}>{c.icon}</span>
                <span style={{ width: 70, textAlign: "right", color: T.inkDim, fontSize: FZ.tiny }}>
                  {t.mean_route_km > 0 ? `${fmt(t.mean_route_km)} km` : ""}
                </span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: SPACE.sm, paddingLeft: 22 }}>
                <span style={{ color: T.inkFaint, fontSize: FZ.tiny, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {t.goods.map((gn) => GOOD_META.get(gn)?.emoji ?? "").join(" ")}
                  {t.goods.length > 0 && <span style={{ marginLeft: 4 }}>{t.goods.slice(0, 3).map((gn) => GOOD_META.get(gn)?.label ?? gn).join(" · ")}</span>}
                </span>
                <span style={{ flex: 1 }} />
                {t.reexport > 0 && <span style={{ color: T.inkFaint, fontSize: FZ.micro }}>re-exported {fmt(t.reexport)}</span>}
                <StandingBadges t={t} />
              </div>
            </div>
          );
        })}
        <FootNote>
          · unnamed local merchants — the real trading capacity of this city. ⚜ house · 🏛 guild.
        </FootNote>
      </Section>

      {/* ── Established here — standing whether or not it carries ──────────────── */}
      <Section title="Who is established here">
        {flows.established.length === 0 && <EmptyNote>No house or guild holds an office, a bailo, or the council seat here.</EmptyNote>}
        {flows.established.map((e: CityEstablished) => (
          <div
            key={`${e.house}:${e.name}`}
            style={{ display: "flex", alignItems: "center", gap: SPACE.sm, padding: "3px 4px", borderBottom: `1px solid ${T.lineSoft}` }}
          >
            <span style={{ width: 14, fontSize: FZ.tiny }}>{e.is_guild ? "🏛" : "⚜"}</span>
            <span style={{ flex: 1, minWidth: 70, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{e.name}</span>
            <StandingBadges t={e} />
            <span style={{ width: 70, textAlign: "right", color: e.volume > 0 ? T.inkMid : T.inkFaint, fontSize: FZ.tiny }}>
              {e.volume > 0 ? `carries ${fmt(e.volume)}` : "carries 0"}
            </span>
          </div>
        ))}
      </Section>

      {/* ── The world-wide "why" note, folded away by default (decision 4) ─────── */}
      <div
        data-no-drag
        onClick={() => setWhyOpen((v) => !v)}
        style={{ cursor: "pointer", color: T.inkFaint, fontSize: FZ.tiny, padding: "3px 4px", userSelect: "none" }}
      >
        {whyOpen ? "▾" : "▸"} why {(100 - (w.shipments > 0 ? (w.by_house / w.shipments) * 100 : 0)).toFixed(0)}% moves on no house's account (world-wide)
      </div>
      {whyOpen && w.shipments > 0 && (
        <Card style={{ marginTop: SPACE.xs }}>
          <div style={{ fontSize: FZ.tiny, color: T.inkMid, marginBottom: SPACE.sm }}>
            of {w.shipments.toLocaleString()} shipments, {w.by_house.toLocaleString()} were financed by a house.
            the rest went ownerless because:
          </div>
          {([
            ["no house at either end", w.why_nohouse],
            ["no free vessel", w.why_slot],
            ["could not afford it", w.why_cash],
            ["barred from the market", w.why_barred],
          ] as const).map(([label, n]) => (
            <div key={label} style={{ display: "flex", gap: SPACE.sm, fontSize: FZ.tiny, padding: "1px 0" }}>
              <span style={{ flex: 1, color: T.inkDim }}>{label}</span>
              <span style={{ color: T.inkMid }}>{n.toLocaleString()}</span>
              <span style={{ width: 34, textAlign: "right", color: T.inkFaint }}>
                {w.ownerless > 0 ? `${((n / w.ownerless) * 100).toFixed(0)}%` : "0%"}
              </span>
            </div>
          ))}
          <FootNote>
            These counters are world-wide, not this city's — the sim keeps them globally, and
            attributing them to one place would be inventing a measurement never taken.
          </FootNote>
        </Card>
      )}
    </div>
  );
}
