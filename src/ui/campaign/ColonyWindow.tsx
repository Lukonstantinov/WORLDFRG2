import { useEffect, useMemo, useState } from "react";
import type { ColonyDetail, ColonyGateStatus, ColonySummary, HubDetail } from "@types";
import { campaignGetColony, campaignGetHub } from "@bridge";
import { useCampaignStore } from "@state/campaignStore";
import { GOOD_DEFS } from "@goods";
import { GoodIcon } from "@ui/goods/GoodIcon";
import { koppenCode } from "@ui/world/climate";
import { STYLES, landmarkIcon, vesselIcon, toHex, type CityCfg, type StyleKey } from "@canvas/cityArt";
import { VESSEL_ICONS } from "@ui/campaign/settlementWindowData";
import {
  K, fk, fc, gcol, Card, CardGrid, Bar, Stack, Tag, Swatch, Sub, KV, Big, Pips, HdrRow, Note, Empty, Blit,
  WindowFrame, IsoThumb, useCultureKits, cityScene,
} from "@ui/campaign/windowKit";

// ── The colonial city view (design handoff "Turn 5", 5a colony · 5b outpost) ──
// Header · a growth strip (one seeded city drawn at the four colony stages) ·
// the stage ladder with its real growth gate · five cards. Everything reads
// ColonyDetail / ColonySummary / HubDetail. Not in data and so not shown: the
// capital sunk into the venture, per-contract "% met", distance to the founder.

const STAGES = ["", "Outpost", "Colony", "Town", "City"];
/** The landmark set each stage's strip panel draws (the handoff's STAGE_B). */
const STAGE_B: string[][] = [[], ["Warehouse", "Harbor"], ["Warehouse", "Harbor", "Granary", "Temple"],
  ["Warehouse", "Harbor", "Granary", "Temple", "Guildhall", "Workshop", "Council Hall"],
  ["Warehouse", "Harbor", "Granary", "Temple", "Guildhall", "Workshop", "Council Hall", "Citadel", "Mint", "Shipyard"]];
// Mirrors `colony_pass` (sim/campaign/tick/colonies.rs): a settlement colony
// advances past stage N once population clears the next threshold, it has had
// five unbroken years of supply and holds N buildings; it may rise at 50 years
// once it is a Town. (ColonyDetail.indep_in_years still counts to the older 70.)
const STAGE_POP = [0, 0, 4_000, 15_000, 40_000];
const INDEPENDENCE_AGE = 50;
// Mirrors `maybe_graduate_outpost` (houses.rs / tick mod.rs constants).
const OUTPOST_GRADUATE_YEARS = 30, OUTPOST_MAX_POP = 800, OUTPOST_GRADUATE_WEALTH = 60_000;
const COLONY_VIOLET = "#c08cff";
const SUPPLY_CAT = ["Food", "Stores", "Preservatives"];
const BACKER_KIND = ["city", "house", "bank"];
/** `reserve_food` / `reserve_cap` count DAYS of the colony's own need (the sim
 *  adds or draws one per day), so cover in months is a plain division. */
const DAYS_PER_MONTH = 30.4;
const GOOD_BY_NAME = new Map(GOOD_DEFS.map((g) => [g.name, g]));

export function ColonyWindow({ summary, roster, gates, onBack, onClose, onDragStart }: {
  summary: ColonySummary; roster: ColonySummary[]; gates: ColonyGateStatus | null;
  onBack: () => void; onClose: () => void; onDragStart: (e: React.PointerEvent<HTMLElement>) => void;
}) {
  const tick = useCampaignStore((s) => s.snapshot?.clock?.tick ?? 0);
  const houses = useCampaignStore((s) => s.houses);
  const kits = useCultureKits();
  const [detail, setDetail] = useState<ColonyDetail | null>(null);
  const [hub, setHub] = useState<HubDetail | null>(null);
  const outpost = summary.colony_kind === 2;

  useEffect(() => {
    let alive = true;
    if (!outpost) campaignGetColony(summary.id).then((d) => { if (alive) setDetail(d); }).catch(() => { if (alive) setDetail(null); });
    else setDetail(null);
    campaignGetHub(summary.id).then((h) => { if (alive) setHub(h); }).catch(() => { if (alive) setHub(null); });
    return () => { alive = false; };
  }, [summary.id, outpost, tick]);

  const kit = hub?.culture ? kits?.get(hub.culture) : undefined;
  const base = useMemo(() => hub ? cityScene(hub, {
    kit, river: (hub.vessels?.classes.find((c) => c.kind === "river")?.registered ?? 0) > 0,
  }) : null, [hub, kit]);
  const style: StyleKey = base?.style ?? "med";
  const color = outpost ? toHex(summary.owner_color, "#c9a96a") : COLONY_VIOLET;
  const stage = Math.max(1, Math.min(4, summary.colony_stage || 1));
  const age = detail?.age_years ?? summary.age_years;
  const reserve = detail?.reserve_food ?? summary.reserve_food, cap = detail?.reserve_cap ?? summary.reserve_cap;
  const owner = houses.find((h) => h.name === summary.owner_house_name);

  const stageCfg = (s: number): CityCfg | null => base && {
    ...base.cfg, walled: s >= 3 && base.cfg.walled, popBucket: s - 1, ships: Math.min(base.cfg.ships, s * 2),
    buildings: STAGE_B[s].map((k, i) => ({ kind: k, status: "op" as const, color: i ? "#9a8a78" : color, ref: i })),
  };
  const notes = outpost
    ? ["Trade post · cap 800", "Graduates to a colony", "Grown town", "Full city"]
    : ["Founded", `${fk(STAGE_POP[2])} people`, `${fk(STAGE_POP[3])} people · walls`, `${fk(STAGE_POP[4])} people`];

  // The real gate toward the next stage.
  const checks: [string, boolean | null][] = [];
  let next: string | null = null;
  if (outpost) {
    next = "Colony";
    checks.push([`${OUTPOST_GRADUATE_YEARS} years held`, age >= OUTPOST_GRADUATE_YEARS]);
    checks.push([`${fk(OUTPOST_MAX_POP * .9)} of ${OUTPOST_MAX_POP} people`, summary.population >= OUTPOST_MAX_POP * .9]);
    checks.push([`${summary.owner_house_name || "owner"} worth ${fk(OUTPOST_GRADUATE_WEALTH)}`, owner ? owner.wealth >= OUTPOST_GRADUATE_WEALTH : null]);
    checks.push(["trade passing through", null]);
  } else if (stage < 4) {
    next = STAGES[stage + 1];
    checks.push([`${fk(STAGE_POP[stage + 1])} people`, summary.population >= STAGE_POP[stage + 1]]);
    checks.push(["5 years unbroken supply", (detail?.supply_years ?? summary.supply_years) >= 5]);
    const nb = hub?.structures?.length;
    checks.push([`${stage} ${stage === 1 ? "building" : "buildings"}`, nb === undefined ? null : nb >= stage]);
  }
  const selfRule = outpost ? Math.max(0, OUTPOST_GRADUATE_YEARS - age) : Math.max(0, INDEPENDENCE_AGE - age);

  const mine = roster.filter((c) => outpost ? c.colony_kind === 2 && c.owner_house_name === summary.owner_house_name
    : c.colony_kind === 1 && c.founder_hub === summary.founder_hub);
  const kc = koppenCode(hub?.koppen ?? 0);

  return (
    <WindowFrame onDragStart={onDragStart} onClose={onClose}
      mark={<span style={{ width: 14, height: 14, borderRadius: outpost ? 3 : "50%", background: color, boxShadow: `0 0 0 2px ${K.bd}`, flex: "none" }} />}
      name={summary.name}
      tag={`${outpost ? "HOUSE OUTPOST" : "COLONY"} · STAGE ${stage} OF 4`}
      sub={<>
        <span data-no-drag onClick={onBack} style={{ color: K.ac, cursor: "pointer", marginRight: 8 }}>‹ Roster</span>
        {outpost ? `Trade outpost of ${summary.owner_house_name || "—"}` : `Colony of ${summary.founder_name || "—"}`}
        {base && <span style={{ color: K.fa }}> · {STYLES[style].label}{kc ? ` · ${kc}` : ""}</span>}
      </>}
      stats={[["Pop", fc(summary.population)],
        ...(cap > 0 ? [["Food reserve", `${fk(reserve)} / ${fk(cap)} d`, K.ac] as [string, string, string]] : []),
        ["Coin", (detail?.coin_name || summary.coin_name) || "barter"]]}>

      {/* ── growth strip ── */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 10, padding: "14px 16px", background: K.scene, borderBottom: `1px solid ${K.bd}` }}>
        {[1, 2, 3, 4].map((s) => {
          const now = s === stage, fut = s > stage;
          const cfg = stageCfg(s);
          return (
            <div key={s} style={{ position: "relative", borderRadius: 6, overflow: "hidden", aspectRatio: "272 / 176",
              boxShadow: `0 0 0 ${now ? 2 : 1}px ${now ? K.ac : K.bd}`, background: K.scene }}>
              {cfg && <IsoThumb cfg={cfg} W={272} H={176} N={28} R={2 + s} TW={12}
                style={fut ? { filter: "saturate(.35) brightness(1.04)", opacity: .55 } : undefined} />}
              <div style={{ position: "absolute", left: 8, top: 8, display: "flex", gap: 5, alignItems: "center" }}>
                <span style={{ background: now ? K.ac : K.chip, color: now ? "#0b1420" : K.tx, border: `1px solid ${now ? K.ac : K.bd}`,
                  borderRadius: 4, padding: "2px 9px", font: `600 10.5px/1.3 ${K.bf}` }}>{s} · {STAGES[s]}</span>
                {fut && <span style={{ background: K.chip, color: K.fa, border: `1px dashed ${K.bd}`, borderRadius: 4, padding: "2px 8px", font: `600 10px/1.3 ${K.bf}` }}>projected</span>}
              </div>
              <div style={{ position: "absolute", left: 8, bottom: 8, background: K.chip, color: now ? K.tx : K.mu, border: `1px solid ${K.bd}`,
                borderRadius: 4, padding: "2px 9px", font: `400 10.5px/1.3 ${K.bf}` }}>{now ? `now · ${fc(summary.population)} people` : notes[s - 1]}</div>
            </div>
          );
        })}
      </div>

      {/* ── stage ladder + self-rule ── */}
      <div style={{ display: "flex", alignItems: "center", gap: 14, padding: "10px 18px", borderBottom: `1px solid ${K.bd}`, background: K.head, flexWrap: "wrap" }}>
        <div style={{ display: "flex", gap: 4 }}>
          {[1, 2, 3, 4].map((t) => <span key={t} title={STAGES[t]} style={{ width: 26, height: 6, borderRadius: 4, background: t <= stage ? K.ac : K.bar }} />)}
        </div>
        <span style={{ font: `600 11px ${K.bf}`, color: K.mu, whiteSpace: "nowrap" }}>
          {next ? <>Toward <b style={{ color: K.tx }}>{next}</b></> : <b style={{ color: K.tx }}>Full city</b>}
        </span>
        <div style={{ display: "flex", gap: 12, flexWrap: "wrap", minWidth: 0, flex: 1 }}>
          {checks.map(([l, ok]) => (
            <span key={l} title={ok === null ? "Not reported by the sim" : undefined}
              style={{ font: `400 11px ${K.bf}`, whiteSpace: "nowrap", color: ok ? K.pos : K.fa, opacity: ok === null ? .7 : 1 }}>
              {ok ? "✓" : ok === null ? "?" : "○"} {l}
            </span>
          ))}
        </div>
        <span style={{ font: `600 11px ${K.bf}`, color: K.mu, whiteSpace: "nowrap" }}>
          {summary.autonomous ? <b style={{ color: K.pos }}>Self-governing</b>
            : <>{outpost ? "Colony charter in" : "Independence in"} <b style={{ color: K.tx, font: `700 15px ${K.hf}` }}>{selfRule}</b> yrs
              {!outpost && stage < 3 && <span style={{ color: K.fa, fontWeight: 400 }}> · once a Town</span>}</>}
        </span>
      </div>

      <CardGrid>
        <FounderCard summary={summary} detail={detail} style={style} outpost={outpost} age={age} selfRule={selfRule} />
        {detail && <BackersCard detail={detail} />}
        {cap > 0 && <FoodCard detail={detail} reserve={reserve} cap={cap} style={style} />}
        {detail && <SupplyCard detail={detail} />}
        <RosterCard rows={mine} current={summary.id} outpost={outpost} title={outpost ? `Outposts of ${summary.owner_house_name}` : `Colonies founded by ${summary.founder_name}`}
          cap={outpost ? null : gates?.max_settlement_colonies ?? null} />
      </CardGrid>
    </WindowFrame>
  );
}

function FounderCard({ summary, detail, style, outpost, age, selfRule }:
  { summary: ColonySummary; detail: ColonyDetail | null; style: StyleKey; outpost: boolean; age: number; selfRule: number }) {
  const who = outpost ? summary.owner_house_name : (detail?.founder_name || summary.founder_name);
  const icolor = outpost ? toHex(summary.owner_color, "#c9a96a") : "#9a8a78";
  const total = outpost ? OUTPOST_GRADUATE_YEARS : INDEPENDENCE_AGE;
  return (
    <Card title={outpost ? "Owner & charter" : "Founder & charter"} meta={`${age} yrs old`}>
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <Blit src={landmarkIcon(outpost ? "Warehouse" : "Council Hall", style, "op", icolor, 46)} size={46} />
        <div style={{ minWidth: 0 }}>
          <div style={{ font: `700 16px/1.15 ${K.hf}`, color: K.tx }}>{who || "—"}</div>
          <div style={{ font: `400 11px ${K.bf}`, color: K.mu }}>{outpost ? `seated at ${summary.founder_name || "—"}` : "metropolis"}</div>
        </div>
      </div>
      <KV k="Chartered bank">{(detail?.main_bank_name || summary.main_bank_name) || "—"}</KV>
      <KV k="Settles in">{(detail?.coin_name || summary.coin_name) || "barter"}</KV>
      {!outpost && <KV k="Backers' charter">{(detail?.charter_open ?? summary.charter_open)
        ? <span style={{ color: K.pos }}>in force</span> : <span style={{ color: K.fa }}>lapsed</span>}</KV>}
      {!outpost && <KV k="Unbroken supply">{(detail?.supply_years ?? summary.supply_years).toFixed(1)} yrs</KV>}
      {!summary.autonomous && (
        <>
          <Sub meta={`${age} yrs in · ${selfRule} to go`}>{outpost ? "Road to a colony charter" : "Road to self-rule"}</Sub>
          <div style={{ display: "flex" }}><Bar frac={Math.min(1, age / total)} color={K.ac} h={7} /></div>
        </>
      )}
      <Note>{outpost ? "An outpost stays the house's until it is old, full and trading enough to be chartered a colony."
        : "At independence the colony seats its own council and may strike its own coin."}</Note>
    </Card>
  );
}

function BackersCard({ detail }: { detail: ColonyDetail }) {
  const tot = detail.backers.reduce((s, b) => s + b.share, 0) || 1;
  return (
    <Card title="Backers of the venture" meta={`${detail.backers.length} ${detail.backers.length === 1 ? "backer" : "backers"}`}>
      {detail.backers.length === 0 ? <Empty>No backers on record.</Empty> : (
        <>
          <Stack h={10} parts={detail.backers.map((b) => [b.share, toHex(b.color)] as [number, string])} />
          {detail.backers.map((b, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
              <span style={{ font: `600 11.5px ${K.bf}`, color: K.tx, display: "flex", gap: 6, alignItems: "center", minWidth: 0, flex: 1, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                <Swatch color={toHex(b.color)} round />{b.name}
              </span>
              <Tag>{BACKER_KIND[b.kind] ?? "backer"}</Tag>
              <span style={{ width: 40, textAlign: "right", font: `700 14px ${K.hf}`, color: K.tx }}>{Math.round(b.share / tot * 100)}%</span>
            </div>
          ))}
          <Note>Backers split the colony's trade dividend by share.</Note>
        </>
      )}
    </Card>
  );
}

function FoodCard({ detail, reserve, cap, style }: { detail: ColonyDetail | null; reserve: number; cap: number; style: StyleKey }) {
  const f = cap > 0 ? reserve / cap : 0;
  const carry = detail?.supply_capacity ?? 0, got = detail?.supply_delivered ?? 0;
  return (
    <Card title="Food reserve" meta={`${Math.round(f * 100)}% of granary`}>
      <Big value={fc(reserve)} sub={`of ${fc(cap)} days' stores`} />
      <div style={{ display: "flex" }}><Bar frac={f} color={gcol(f * 1.2)} h={8} /></div>
      <div style={{ font: `400 11px ${K.bf}`, color: K.mu }}>
        Covers <b style={{ color: K.tx }}>{(reserve / DAYS_PER_MONTH).toFixed(1)} months</b> without convoys
      </div>
      {detail && (detail.supply_ships > 0 || detail.supply_source) && (
        <>
          <Sub meta={detail.supply_source ? `from ${detail.supply_source}` : "no source"}>Grain-run fleet</Sub>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <Blit src={vesselIcon(VESSEL_ICONS[style].sea, style, 40)} size={40} style={{ borderRadius: 6 }} />
            <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 4 }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                <b style={{ font: `700 16px ${K.hf}`, color: K.tx }}>{detail.supply_ships}</b>
                <span style={{ font: `400 10.5px ${K.bf}`, color: K.fa }}>dedicated hulls · carry {fk(carry)} / mo</span>
              </div>
              <Stack h={7} parts={[[got, K.sea], [Math.max(0, carry - got), K.bar]]} />
              <div style={{ font: `400 10.5px ${K.bf}`, color: got <= 0 ? K.neg : K.fa }}>
                <b style={{ color: K.tx }}>{fk(got)}</b> delivered last month{carry > 0 ? ` · ${Math.round(Math.min(1, got / carry) * 100)}% of carriage` : ""}
              </div>
            </div>
          </div>
        </>
      )}
    </Card>
  );
}

function SupplyCard({ detail }: { detail: ColonyDetail }) {
  const cols = "92px minmax(0,1.1fr) minmax(0,1fr) minmax(0,1fr) 64px";
  const mx = Math.max(1e-6, ...detail.supply.map((s) => s.qty));
  const tot = detail.supply.reduce((s, x) => s + x.qty, 0);
  return (
    <Card span={2} title="Supply convoys · civic contracts" meta={`${detail.supply.length} ${detail.supply.length === 1 ? "contract" : "contracts"} · ${fk(tot)} / mo`}>
      {detail.supply.length === 0 ? <Empty>No convoy contracts — the colony is feeding itself.</Empty> : (
        <>
          <HdrRow cols={cols} labels={[["Category"], ["Supplier"], ["Good"], ["Monthly share"], ["Qty / mo", true]]} />
          {detail.supply.map((s, i) => {
            const food = s.category === 0;
            return (
              <div key={i} style={{ display: "grid", gridTemplateColumns: cols, gap: 10, alignItems: "center" }}>
                <span><Tag color={food ? K.pos : K.mu}>{SUPPLY_CAT[s.category] ?? "Supply"}</Tag></span>
                <span style={{ font: `600 11.5px ${K.bf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{s.supplier || "—"}</span>
                <div style={{ display: "flex", alignItems: "center", gap: 7, minWidth: 0 }}>
                  <GoodIcon name={s.good} size={22} />
                  <span style={{ font: `400 11.5px ${K.bf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{GOOD_BY_NAME.get(s.good)?.label ?? s.good}</span>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <Bar frac={s.qty / mx} color={food ? K.pos : K.sea} h={7} />
                  <span style={{ font: `400 10px ${K.bf}`, color: K.fa, width: 34, textAlign: "right" }}>{Math.round(s.qty / (tot || 1) * 100)}%</span>
                </div>
                <span style={{ textAlign: "right", font: `700 13px ${K.hf}`, color: K.tx }}>{fc(s.qty)}</span>
              </div>
            );
          })}
          <Note>Convoys sail from {detail.supply_source || "the metropolis"} on the backers' account; a shortfall draws down the reserve first.</Note>
        </>
      )}
    </Card>
  );
}

function RosterCard({ rows, current, outpost, title, cap }:
  { rows: ColonySummary[]; current: number; outpost: boolean; title: string; cap: number | null }) {
  return (
    <Card title={title} meta={cap != null ? `${rows.length} here · world cap ${cap}` : `${rows.length}`}>
      {rows.map((c) => {
        const me = c.id === current;
        const ind = Math.max(0, (outpost ? OUTPOST_GRADUATE_YEARS : INDEPENDENCE_AGE) - c.age_years);
        return (
          <div key={c.id} style={{ display: "flex", flexDirection: "column", gap: 4, padding: "6px 8px", borderRadius: 6,
            ...(me ? { background: K.acBg, border: `1px solid ${K.acBd}` } : { border: "1px solid transparent" }) }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
              <span style={{ font: `700 13px ${K.hf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{c.name}</span>
              <Tag>{c.colony_kind === 2 ? "outpost" : "colony"}</Tag>
              <span style={{ flex: 1 }} />
              <span style={{ font: `600 11px ${K.bf}`, color: K.tx }}>{fc(c.population)}</span>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Pips n={c.colony_stage} of={4} w={16} />
              <span style={{ font: `400 10.5px ${K.bf}`, color: K.fa }}>{STAGES[c.colony_stage] ?? ""}</span>
              <span style={{ flex: 1 }} />
              <span style={{ font: `400 10.5px ${K.bf}`, color: c.autonomous ? K.pos : K.mu }}>
                {c.autonomous ? "independent" : `${outpost ? "charter" : "indep."} in ${ind} yrs`}
              </span>
            </div>
          </div>
        );
      })}
    </Card>
  );
}
