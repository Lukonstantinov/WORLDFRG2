import { useEffect, useMemo, useState } from "react";
import { campaignWorksCard } from "@bridge";
import type { WorksCardInfo } from "@types";
import { useGoodsStore } from "@state/goodsStore";
import { T, FZ, RADIUS } from "@ui/campaign/chronicleTheme";
import { Meter, Tabs } from "@ui/kit";
import { CoatOfArms, houseColor } from "@ui/heraldry/CoatOfArms";

/** ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.6/9.2 (A10) · a shareholder or
 *  tenant row carries its holder's ACTUAL arms colour, not the generic
 *  `distinct_color` golden-angle hash the backend still tags every row with
 *  (kept there for banks/realms, which have no heraldry). A house or guild
 *  (`holder_kind` 1 or 2) resolves through the SAME `houseColor` the House
 *  Dossier's own shield renders with, so an ownership bar reads as heraldry
 *  rather than as a chart legend — the whole point of A10. */
function ownerColor(kind: number, name: string, fallback: string): string {
  return kind === 1 || kind === 2 ? houseColor(name) : fallback;
}

function fmt(n: number): string {
  const a = Math.abs(n);
  if (a >= 10_000) return `${(n / 1000).toFixed(1)}k`;
  if (a >= 100) return Math.round(n).toString();
  return n.toFixed(1);
}

/** A tiny sparkline over up to 12 points, no library — a handful of line
 *  segments in an inline SVG. */
function Sparkline({ values, color }: { values: number[]; color: string }) {
  if (values.length < 2) return null;
  const max = Math.max(...values, 1e-6);
  const min = Math.min(...values, 0);
  const span = Math.max(max - min, 1e-6);
  const w = 100, h = 24;
  const pts = values.map((v, i) => {
    const x = (i / (values.length - 1)) * w;
    const y = h - ((v - min) / span) * h;
    return `${x},${y}`;
  }).join(" ");
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} style={{ display: "block" }}>
      <polyline points={pts} fill="none" stroke={color} strokeWidth={1.5} />
    </svg>
  );
}

const rowStat: React.CSSProperties = {
  background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: RADIUS.sm,
  padding: "5px 7px", flex: 1, minWidth: 0,
};

/** ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.6 (D15/D16/§8.2), rebuilt on the
 *  House Dossier's own tab pattern (schematic "Option B — The Dossier",
 *  chosen because a mine and a manufactory genuinely need different
 *  Production tabs rather than both fighting for room in one fixed grid).
 *  Identity strip (icon, name, brand) stays outside the tabs since it never
 *  differs by tab; Overview / Production / Ownership below it do. Fetches on
 *  mount and whenever `hub`/`tick` changes; renders nothing while loading or
 *  if the hub isn't (or is no longer) an estate. */
export function WorksCard({ hub, tick }: { hub: number; tick: number }) {
  const [card, setCard] = useState<WorksCardInfo | null>(null);
  const [tab, setTab] = useState<"overview" | "production" | "ownership">("overview");
  const goodMeta = useGoodsStore((s) => s.meta);
  const specs = useGoodsStore((s) => s.specs);

  useEffect(() => {
    let alive = true;
    campaignWorksCard(hub).then((c) => { if (alive) setCard(c); }).catch(() => { if (alive) setCard(null); });
    return () => { alive = false; };
  }, [hub, tick]);
  // A new works or a different one selected — land back on Overview rather
  // than stranding the reader on whichever tab the LAST card happened to show.
  useEffect(() => { setTab("overview"); }, [hub]);

  // A manufactory turns raw INPUTS into its good — surface the recipe so the card
  // reads as a workshop (what it consumes → what it makes), not just another farm.
  const recipe = useMemo(() => {
    const g = card?.good_name;
    if (!g) return [] as { good: string; qty: number }[];
    return specs.find((s) => s.id === g || s.name === g)?.inputs ?? [];
  }, [specs, card?.good_name]);

  if (!card) return <div style={{ fontSize: FZ.small, color: T.inkFaint, padding: "6px 2px" }}>Loading…</div>;

  const meta = goodMeta(card.good_name);
  const outputs = card.monthly.map((m) => m.output);
  const qualities = card.monthly.map((m) => m.quality);
  const prices = card.monthly.map((m) => m.price);
  const conditionColor = card.condition > 0.7 ? T.good : card.condition > 0.35 ? T.warn : T.bad;
  const conditionWord = card.damage <= 0.01 ? "sound" : card.damage < 0.4 ? "worn" : card.damage < 0.75 ? "damaged" : "ruined";

  // ── production Sankey (manufactories only): raw INPUTS → workshop → OUTPUT,
  // bar widths carrying TONNAGE (qty × output). Margin = output value vs input cost;
  // waste = mass that doesn't leave as product. All derived from the recipe + output
  // + prices already on the card, so it's live once a workshop actually trades. ──
  const baseVal = (id: string) => specs.find((s) => s.id === id || s.name === id)?.base_value ?? 1;
  const out = card.monthly_output;
  const outPrice = prices.length ? prices[prices.length - 1] : baseVal(card.good_name);
  const inFlows = recipe.map((r) => {
    const m = goodMeta(r.good);
    const ton = r.qty * out;
    return { name: m.name, icon: m.icon, color: m.color, ton, cost: ton * baseVal(r.good) };
  });
  const inCost = inFlows.reduce((s, f) => s + f.cost, 0);
  const marginPct = inCost > 1e-6 ? Math.round(((out * outPrice - inCost) / inCost) * 100) : null;
  const totalInTon = inFlows.reduce((s, f) => s + f.ton, 0);
  const lossPct = totalInTon > out ? Math.round(((totalInTon - out) / totalInTon) * 100) : 0;
  const maxTon = Math.max(out, ...inFlows.map((f) => f.ton), 1e-6);
  const outMeta = goodMeta(card.good_name);

  return (
    <div style={{ border: `1px solid ${T.line}`, borderRadius: RADIUS.md, padding: 8, background: T.raised, marginTop: 4 }}>
      {/* ── Identity strip — the same on every tab ─────────────────────────── */}
      <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
        <span style={{ fontSize: 18 }}>{meta.icon}</span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ color: T.gold, fontWeight: 700, fontSize: FZ.body }}>{card.name}</div>
          <div style={{ color: T.inkDim, fontSize: FZ.small }}>
            {card.kind_label.toLowerCase()} · tier {card.tier} · {card.good_name}
          </div>
          {card.brand && (
            <div style={{ color: T.parchment, fontSize: FZ.small, fontStyle: "italic", marginTop: 1 }}
              title="A toponymic brand — this works' name for its good, carried with the cargo and known in distant markets">
              known abroad as "{card.brand}"
            </div>
          )}
        </div>
      </div>

      <Tabs
        tabs={[["overview", "Overview"], ["production", "Production"], ["ownership", "Ownership"]] as const}
        active={tab}
        onSelect={setTab}
        style={{ marginTop: 6 }}
      />

      {/* ── OVERVIEW — condition, rank, age, workforce, a mine's real body ──── */}
      {tab === "overview" && (
        <div style={{ marginTop: 6 }}>
          <div style={{ fontSize: FZ.small, color: T.inkMid }}>
            ⭐ {card.rank}{card.rank === 1 ? "st" : card.rank === 2 ? "nd" : card.rank === 3 ? "rd" : "th"} of {card.rank_of} ·
            {" "}yield {card.yield_index.toFixed(1)}× · <span style={{ color: T.gold, fontWeight: 700 }}>{card.yield_label.toUpperCase()}</span>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 6 }}>
            <span style={{ fontSize: FZ.small, color: conditionColor, width: 70 }}>{conditionWord}</span>
            <Meter value={card.condition} max={1} color={conditionColor} height={5} />
            <span style={{ fontSize: FZ.small, color: T.inkMid, minWidth: 60, textAlign: "right" }}>
              {fmt(card.monthly_output)}/mo {card.output_delta !== 0 && (
                <span style={{ color: card.output_delta > 0 ? T.goodInk : T.badInk }}>
                  {card.output_delta > 0 ? "▲" : "▼"}
                </span>
              )}
            </span>
          </div>

          {/* Age / workforce — fields the sim already tracked (founding tick,
              the works' own population) but never surfaced anywhere before. */}
          <div style={{ display: "flex", gap: 6, marginTop: 6 }}>
            <div style={rowStat}>
              <div style={{ fontSize: 8, color: T.inkFaint, textTransform: "uppercase" }}>Age</div>
              <div style={{ fontSize: FZ.body, color: T.ink, fontWeight: 600 }}>
                {card.age_years.toFixed(0)} {card.age_years === 1 ? "year" : "years"}
              </div>
            </div>
            <div style={rowStat}>
              <div style={{ fontSize: 8, color: T.inkFaint, textTransform: "uppercase" }}>Workforce</div>
              <div style={{ fontSize: FZ.body, color: T.ink, fontWeight: 600 }}>
                ~{Math.round(card.workforce).toLocaleString()}
              </div>
            </div>
          </div>

          {/* A mine/quarry's real body — grade × depth, already computed for
              the Deposits panel and never read by this card before. */}
          {card.deposit_extent_label && (
            <div style={{
              marginTop: 6, padding: "5px 8px", borderRadius: RADIUS.sm,
              background: "rgba(217,164,65,0.07)", border: `1px solid rgba(217,164,65,0.3)`,
              fontSize: FZ.small, color: "#e6c06a",
            }}>
              ⛏ {card.deposit_extent_label} body, {card.deposit_depth_label} working
            </div>
          )}
        </div>
      )}

      {/* ── PRODUCTION — the input→output flow for a manufactory, the raw
          twelve-month curves for everything else (a farm/mine has no recipe
          to draw a Sankey from, so the curves ARE its production story). ──── */}
      {tab === "production" && (
        <div style={{ marginTop: 6 }}>
          {inFlows.length > 0 && (
            <div style={{ padding: "6px 4px", background: T.card, borderRadius: RADIUS.sm, border: `1px solid ${T.line}` }}
              title="Production flow — raw inputs worked into the finished good; bar length is monthly tonnage">
              {inFlows.map((f, i) => (
                <div key={i} style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 2 }}>
                  <span style={{ width: 74, fontSize: FZ.small, color: T.inkMid, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{f.icon} {f.name}</span>
                  <div style={{ flex: 1, height: 8, background: "#0a1018", borderRadius: 2, overflow: "hidden" }}>
                    <div style={{ width: `${Math.max(4, (f.ton / maxTon) * 100)}%`, height: "100%", background: f.color, opacity: 0.85 }} />
                  </div>
                  <span style={{ width: 40, textAlign: "right", fontSize: FZ.small, color: T.inkDim }}>{fmt(f.ton)}</span>
                </div>
              ))}
              <div style={{ textAlign: "center", fontSize: FZ.small, color: T.inkFaint, margin: "1px 0" }}>⚒ works into ▼</div>
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span style={{ width: 74, fontSize: FZ.small, color: T.gold, fontWeight: 700, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{outMeta.icon} {outMeta.name}</span>
                <div style={{ flex: 1, height: 8, background: "#0a1018", borderRadius: 2, overflow: "hidden" }}>
                  <div style={{ width: `${Math.max(4, (out / maxTon) * 100)}%`, height: "100%", background: T.gold }} />
                </div>
                <span style={{ width: 40, textAlign: "right", fontSize: FZ.small, color: T.gold }}>{fmt(out)}/mo</span>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", marginTop: 3, fontSize: FZ.small }}>
                {marginPct !== null && (
                  <span style={{ color: marginPct >= 0 ? T.goodInk : T.badInk }} title="output value vs input cost">
                    margin {marginPct >= 0 ? "+" : ""}{marginPct}%
                  </span>
                )}
                {lossPct > 0 && <span style={{ color: T.inkFaint }} title="mass that does not leave as product">waste {lossPct}%</span>}
              </div>
            </div>
          )}

          {outputs.length >= 2 && (
            <div style={{ display: "flex", gap: 10, marginTop: inFlows.length > 0 ? 8 : 0 }}>
              <div>
                <div style={{ fontSize: 8, color: T.inkFaint }}>output</div>
                <Sparkline values={outputs} color={T.accent} />
              </div>
              <div>
                <div style={{ fontSize: 8, color: T.inkFaint }}>quality</div>
                <Sparkline values={qualities} color={T.gold} />
              </div>
              <div>
                <div style={{ fontSize: 8, color: T.inkFaint }}>price</div>
                <Sparkline values={prices} color={T.goodInk} />
              </div>
            </div>
          )}

          {inFlows.length === 0 && outputs.length < 2 && (
            <div style={{ fontSize: FZ.small, color: T.inkFaint, padding: "4px 2px" }}>
              Too young to show a trend yet — check back after its first few months.
            </div>
          )}
        </div>
      )}

      {/* ── OWNERSHIP ────────────────────────────────────────────────────────── */}
      {tab === "ownership" && (
        <div style={{ marginTop: 6 }}>
          {card.owners.length > 0 ? (
            <>
              <div style={{ display: "flex", width: "100%", height: 8, borderRadius: 3, overflow: "hidden" }}>
                {card.owners.map((o, i) => {
                  const c = ownerColor(o.holder_kind, o.name, o.color);
                  return (
                    <div key={i} title={`${o.name} ${Math.round(o.frac * 100)}%`}
                      style={{
                        width: `${o.frac * 100}%`,
                        background: o.holder_kind === 0
                          ? `repeating-linear-gradient(45deg, ${c}, ${c} 3px, transparent 3px, transparent 6px)`
                          : c,
                      }} />
                  );
                })}
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 5 }}>
                {card.owners.map((o, i) => (
                  <span key={i} style={{ fontSize: FZ.small, color: T.inkMid, display: "inline-flex", alignItems: "center", gap: 3 }}>
                    {o.holder_kind === 1 || o.holder_kind === 2 ? (
                      <CoatOfArms name={o.name} size={12} guild={o.holder_kind === 2} />
                    ) : (
                      <span style={{ color: ownerColor(o.holder_kind, o.name, o.color) }}>■</span>
                    )}
                    {o.name} {Math.round(o.frac * 100)}%
                    {o.instrument === 1 && o.term_years > 0 && (
                      <span style={{ color: T.inkFaint }}> ({o.term_years}yr tenancy)</span>
                    )}
                  </span>
                ))}
              </div>
            </>
          ) : (
            <div style={{ fontSize: FZ.small, color: T.inkFaint, padding: "4px 2px" }}>No ownership record.</div>
          )}
        </div>
      )}
    </div>
  );
}
