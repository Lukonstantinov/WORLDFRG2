/** Trade ▸ Internal subtab — user request: "I need a tab for internal market
 *  which is linked with external market." Answers a question Market and Flows
 *  each answer half of: of everything this city MAKES, how much never leaves
 *  (claimed by its own manufactories, its council's civic reserve, and its own
 *  households) versus how much reaches the external market Flows already shows
 *  (its own produce exported, plus whatever merely passes through). One row per
 *  good carries BOTH readings side by side — literally linking the two rather
 *  than asking the reader to flip between tabs and reconcile two payloads by
 *  eye.
 *
 *  Every number here is derived from state the sim already carries — no new
 *  tick mechanism, per rule 14 (a view is never a decision):
 *  - `goodSplit` (shared with `FlowsView`) already derives own-export / transit
 *    / bought-for-us from `TradeFlowGood`; "kept here" is simply what's left of
 *    `own_production` once the exported share is taken out.
 *  - `demand_shares`/`civic_goods` (`HubGoodDetail`) are the sim's own booked
 *    buyer ledger (`docs/CONSUMPTION_REBUILD_PLAN.md` S6) and the council's own
 *    right-of-first-buy reserve (`council_provision_pass`) — read, not guessed.
 *  - `charter_holder`/`charter_share` surface the STAPLE RIGHT mechanism that
 *    already runs live in `dispatch` (`CHARTER_EXCLUSIVE_DOSE`) — a political
 *    house or guild that dominates its own seat is already granted an
 *    exclusive charter on its specialty goods there; this tab is the first
 *    place a player can actually SEE one.
 *
 *  The one number this tab does NOT invent is a household purchase figure:
 *  ordinary consumption (`eat = need.min(stock)`) books no counterparty, so
 *  `demand_shares[2]` reads honestly as 0 — the same "no buyers in this
 *  economy" finding `docs/CONSUMPTION_AND_GOODS_REVIEW.md` names, restated
 *  here rather than papered over with an inferred split. */
import { useEffect, useMemo, useState } from "react";
import { campaignTradeFlows } from "@bridge";
import type { TradeFlows, TradeFlowGood, HubDetail, HubGoodDetail } from "@types";
import { GOOD_DEFS } from "@goods";
import { Section, Card, Badge, Chip, EmptyNote, FootNote, SplitBar } from "@ui/kit";
import { T, SPACE, FZ, RADIUS } from "@ui/campaign/chronicleTheme";
import { goodSplit } from "@ui/campaign/FlowsView";

const GOOD_META = new Map(GOOD_DEFS.map((g) => [g.name, g]));
const TIER_LABELS = ["Basic needs", "Comfort", "Luxury"] as const;

function fmt(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(n >= 10000 ? 0 : 1)}k`;
  return n.toFixed(n >= 100 ? 0 : 1);
}

// LOCAL (stays in this city) reads teal; EXTERNAL (reaches the market Flows
// shows) reuses Flows' own "out" gold — the same colour means the same thing
// on both tabs, which is the whole point of a "linked" reading.
const LOCAL_TINT = "#5fd0a0";
const EXTERNAL_TINT = "#ffce5f";

export function InternalMarketView({ hubId, active, tick, detail }: {
  hubId: number; active: boolean; tick: number;
  /** Already fetched by the parent for the Market tab — reused here rather
   *  than a second `campaign_get_hub` round trip for the same data. */
  detail: HubDetail | null;
}) {
  const [flows, setFlows] = useState<TradeFlows | null>(null);
  const [loading, setLoading] = useState(false);
  const [tierFilter, setTierFilter] = useState<0 | 1 | 2 | null>(null);
  const [expanded, setExpanded] = useState<number | null>(null);

  useEffect(() => {
    if (!active) { setFlows(null); setLoading(false); return; }
    let alive = true;
    setLoading(true);
    campaignTradeFlows(hubId)
      .then((f) => { if (alive) { setFlows(f); setLoading(false); } })
      .catch(() => { if (alive) { setFlows(null); setLoading(false); } });
    return () => { alive = false; };
  }, [hubId, active, tick]);

  const detailByGood = useMemo(() => {
    const m = new Map<number, HubGoodDetail>();
    for (const g of detail?.goods ?? []) m.set(g.good, g);
    return m;
  }, [detail]);

  const rows = useMemo(() => {
    if (!flows) return [];
    return flows.goods
      .map((g: TradeFlowGood) => {
        const { transit, ownExport, forUs } = goodSplit(g);
        const keptHere = Math.max(0, (g.own_production ?? 0) - ownExport);
        const local = keptHere + forUs;
        const external = ownExport + transit;
        return { g, keptHere, ownExport, transit, forUs, local, external, hd: detailByGood.get(g.good) };
      })
      .filter((r) => tierFilter == null || (r.g.need_tier ?? 0) === tierFilter)
      .filter((r) => r.local + r.external > 0)
      .sort((a, b) => b.local + b.external - (a.local + a.external));
  }, [flows, detailByGood, tierFilter]);

  const maxTotal = Math.max(...rows.map((r) => r.local + r.external), 1e-6);

  if (!active) return <EmptyNote>The internal market appears once a campaign is running.</EmptyNote>;
  if (loading && !flows) return <EmptyNote>Loading…</EmptyNote>;
  if (!flows) return <EmptyNote>No trade data for this settlement yet.</EmptyNote>;

  return (
    <div style={{ fontSize: FZ.body, color: T.ink }}>
      <Section
        title="Internal market"
        right={
          <span style={{ display: "flex", flexWrap: "wrap", justifyContent: "flex-end", gap: 4 }}>
            {TIER_LABELS.map((label, t) => (
              <Chip key={t} on={tierFilter === t}
                onClick={() => setTierFilter((f) => f === t ? null : (t as 0 | 1 | 2))}>
                {label}
              </Chip>
            ))}
          </span>
        }
      >
        <FootNote>
          <span style={{ color: LOCAL_TINT }}>■</span> stays in this city (claimed by its own manufactories,
          its council&apos;s reserve, or its own households) · <span style={{ color: EXTERNAL_TINT }}>■</span> reaches
          the external market — this city&apos;s own produce exported, plus whatever merely passes through.
        </FootNote>
        {rows.length === 0 && (
          <EmptyNote>
            {tierFilter != null
              ? `Nothing on the ${TIER_LABELS[tierFilter].toLowerCase()} tier moves through this city yet.`
              : "No production or trade recorded yet."}
          </EmptyNote>
        )}
        {rows.map(({ g, keptHere, ownExport, transit, forUs, local, external, hd }) => {
          const meta = GOOD_META.get(g.name);
          const sel = expanded === g.good;
          const householdShare = hd?.demand_shares?.[2] ?? 0;
          const manufactoryShare = hd?.demand_shares?.[0] ?? 0;
          const councilShare = hd?.demand_shares?.[1] ?? 0;
          const hasDemandReading = manufactoryShare + councilShare + householdShare > 1e-3;
          return (
            <div key={g.good} style={{
              display: "flex", flexWrap: "wrap", alignItems: "center", gap: SPACE.sm,
              padding: "3px 4px", borderRadius: RADIUS.sm, cursor: "pointer",
              background: sel ? T.card : "transparent",
            }}>
              <div data-no-drag
                style={{ display: "flex", alignItems: "center", gap: SPACE.sm, width: "100%" }}
                onClick={() => setExpanded(sel ? null : g.good)}
              >
                <span style={{ width: 12, color: T.inkFaint }}>{sel ? "▾" : "▸"}</span>
                <span style={{ width: 16 }}>{meta?.emoji ?? "•"}</span>
                <span style={{ flex: 1, minWidth: 70, color: sel ? T.gold : T.ink, display: "flex", alignItems: "center", gap: 5 }}>
                  {meta?.label ?? g.name}
                  {(hd?.charter_holder ?? "") !== "" && (
                    <span title={`${hd!.charter_is_guild ? "guild" : "house"} chartered staple right — ${hd!.charter_holder} holds ${(((hd!.charter_share ?? 0) * 100)).toFixed(0)}% of local trade`}
                      style={{
                        display: "inline-flex", alignItems: "center", gap: 2, fontSize: FZ.tiny,
                        color: "#e0b45a", background: "rgba(224,180,90,0.14)", border: "1px solid rgba(224,180,90,0.4)",
                        borderRadius: RADIUS.sm, padding: "0 4px", lineHeight: "14px", flex: "0 0 auto",
                      }}>🔒 chartered</span>
                  )}
                </span>
                <SplitBar inV={local} outV={external} max={maxTotal} inColor={LOCAL_TINT} outColor={EXTERNAL_TINT} height={7} />
                <span style={{ width: 54, textAlign: "right", color: T.inkMid }}>{fmt(local + external)}/yr</span>
              </div>
              {sel && (
                <div style={{ width: "100%", padding: "4px 0 4px 28px", display: "flex", flexDirection: "column", gap: 4 }}>
                  <div style={{ fontSize: FZ.small, color: T.inkMid }}>
                    <span style={{ color: LOCAL_TINT }}>{fmt(local)}/yr stays here</span>
                    {keptHere > 0 && <> — {fmt(keptHere)} of its own production kept</>}
                    {forUs > 0 && <>{keptHere > 0 ? ", " : " — "}{fmt(forUs)} bought in and consumed here</>}
                  </div>
                  <div style={{ fontSize: FZ.small, color: T.inkMid }}>
                    <span style={{ color: EXTERNAL_TINT }}>{fmt(external)}/yr reaches the external market</span>
                    {ownExport > 0 && <> — {fmt(ownExport)} of its own produce exported</>}
                    {transit > 0 && <>{ownExport > 0 ? ", " : " — "}{fmt(transit)} passing through untouched</>}
                  </div>
                  {hasDemandReading && (
                    <div style={{ fontSize: FZ.small, color: T.inkDim }}>
                      Of what&apos;s recently been claimed here: {(manufactoryShare * 100).toFixed(0)}% by manufactories,
                      {" "}{(councilShare * 100).toFixed(0)}% secured by the council
                      {householdShare > 0.01 ? `, ${(householdShare * 100).toFixed(0)}% by households` : ""}.
                    </div>
                  )}
                  {(hd?.civic_goods ?? 0) > 0.5 && (
                    <div style={{ fontSize: FZ.small, color: T.inkDim }}>
                      {fmt(hd!.civic_goods!)} sits secured in the civic warehouse — the council&apos;s own
                      right-of-first-buy reserve, not on the open market.
                    </div>
                  )}
                  {(hd?.charter_holder ?? "") !== "" && (
                    <Card style={{ padding: "4px 6px" }}>
                      <span style={{ fontSize: FZ.small, color: "#e0b45a" }}>
                        🔒 {hd!.charter_is_guild ? "Guild" : "House"} <b>{hd!.charter_holder}</b> holds a chartered
                        staple right on this good here — a rival&apos;s cargo has a real chance of being turned away
                        from this market rather than sold.
                        {(hd!.charter_share ?? 0) > 0 && <> It carries {((hd!.charter_share ?? 0) * 100).toFixed(0)}% of this good&apos;s local trade.</>}
                      </span>
                    </Card>
                  )}
                </div>
              )}
            </div>
          );
        })}
        <FootNote>
          Click a good to expand it. Ordinary household consumption has no price and no recorded buyer in this
          model (<Badge tone="neutral">no counterparty</Badge>) — it is folded into &quot;stays here&quot; but never
          shown as a household share above a token amount.
        </FootNote>
      </Section>
    </div>
  );
}
