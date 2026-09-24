import { useEffect, useMemo, useState } from "react";
import type { HouseBrief, HouseHistory, HouseLedger, HouseStability, HubDetail } from "@types";
import { campaignGetHub, campaignHouseStability } from "@bridge";
import { useCampaignStore } from "@state/campaignStore";
import { useWorldStore } from "@state/worldStore";
import { GOOD_DEFS } from "@goods";
import { GoodIcon } from "@ui/goods/GoodIcon";
import { CoatOfArms } from "@ui/heraldry/CoatOfArms";
import { STYLES, vesselIcon, toHex, type StyleKey } from "@canvas/cityArt";
import { HULL_NAMES, VESSEL_ICONS } from "@ui/campaign/settlementWindowData";
import { dull } from "@ui/campaign/houseShared";
import {
  K, fk, fc, gcol, Card, CardGrid, Bar, Tag, Pill, Sub, Note, Empty, Blit, Spark, HdrRow,
  WindowFrame, IsoThumb, useCultureKits, cityScene,
} from "@ui/campaign/windowKit";

// ── The house / guild plate (design handoff "Turn 5", 5d house · 5e guild) ────
// One screen for one family: header, a band with the seat city, the head and
// the five stability gauges, then the card grid. It sits beside the dossier's
// existing tabs (Chronicle, Kin, Standing, Accountant …) as their overview,
// reading the same HouseBrief / HouseHistory / HouseLedger / HouseStability.
// Not in data and so not drawn: a guild's elected wardens (the head stands in),
// hulls away vs idle per house, and an estate's kind and tier.

const TIER_WORD = ["", "GREAT", "MAJOR", "LESSER", "MARGINAL"];
const ROLE_COLOR: Record<string, string> = { seat: K.ac, dominant: K.pos, bailo: K.sea, owner: K.land, office: K.mu, trade: K.fa };
const GOOD_BY_NAME = new Map(GOOD_DEFS.map((g) => [g.name, g]));
const gl = (g: string) => GOOD_BY_NAME.get(g)?.label ?? g;

export function HouseWindow({ h, chron, ledger, onClose, onDragStart }: {
  h: HouseBrief; chron: HouseHistory | null; ledger: HouseLedger | null;
  onClose: () => void; onDragStart: (e: React.PointerEvent<HTMLElement>) => void;
}) {
  const tick = useCampaignStore((s) => s.snapshot?.clock?.tick ?? 0);
  const economy = useWorldStore((s) => s.economy);
  const settlements = useWorldStore((s) => s.settlements);
  const kits = useCultureKits();
  const [st, setSt] = useState<HouseStability | null>(null);
  const [seat, setSeat] = useState<HubDetail | null>(null);
  const guild = !!h.is_guild;
  const color = guild ? dull(h.color ?? "") : toHex(h.color, "#888888");

  useEffect(() => {
    let alive = true;
    if (h.idx !== undefined) campaignHouseStability(h.idx).then((s) => { if (alive) setSt(s); }).catch(() => { if (alive) setSt(null); });
    campaignGetHub(h.home_hub).then((d) => { if (alive) setSeat(d); }).catch(() => { if (alive) setSeat(null); });
    return () => { alive = false; };
  }, [h.idx, h.home_hub, tick]);

  const econHub = economy?.hubs.find((x) => x.id === h.home_hub);
  const settlement = settlements.find((s) => s.name === h.home_name);
  const kit = seat?.culture ? kits?.get(seat.culture) : undefined;
  const scene = useMemo(() => seat ? cityScene(seat, {
    kit, seaAccess: econHub?.sea_access, elevation: econHub?.elevation,
    river: settlement?.site === "river" || (seat.vessels?.classes.find((c) => c.kind === "river")?.registered ?? 0) > 0,
  }) : null, [seat, kit, econHub?.sea_access, econHub?.elevation, settlement?.site]);
  const style: StyleKey = scene?.style ?? "med";

  const initials = h.head_name.split(" ").filter((w) => /^\p{Lu}/u.test(w)).slice(-2).map((w) => w[0]).join("") || h.name[0];
  const tag = guild ? "MERCHANT GUILD · CIVIC" : h.tier ? `${TIER_WORD[h.tier]} HOUSE · TIER ${h.tier}` : "HOUSE · UNRANKED";

  return (
    <WindowFrame onClose={onClose} onDragStart={onDragStart}
      mark={<CoatOfArms name={h.name} size={26} guild={guild} />}
      name={h.name} tag={tag}
      sub={<>{guild ? "Acts for" : "Seat"} {h.home_name}{h.archetype_label ? ` · ${h.archetype_label}` : ""}
        {h.founded_year != null && <span style={{ color: K.fa }}> · founded {h.founded_year}</span>}
        {h.defunct && <span style={{ color: K.neg }}> · defunct</span>}</>}
      stats={[["Wealth", fc(h.wealth), K.ac], ["Prestige", fk(h.prestige)], ["Power", fk(h.political_power)]]}>

      {/* ── band: seat · head · gauges · liabilities ── */}
      <div style={{ display: "grid", gridTemplateColumns: "372px minmax(0, 1fr)", gap: 16, padding: "14px 16px", background: K.scene, borderBottom: `1px solid ${K.bd}` }}>
        <div style={{ position: "relative", height: 200, borderRadius: 6, overflow: "hidden", boxShadow: `0 0 0 1px ${K.bd}` }}>
          {scene && <IsoThumb cfg={scene.cfg} W={372} H={200} N={32} R={6} TW={14} />}
          <span style={{ position: "absolute", left: 8, top: 8, background: K.chip, color: K.tx, border: `1px solid ${K.bd}`, borderRadius: 4, padding: "2px 9px", font: `600 10.5px/1.3 ${K.bf}` }}>
            {guild ? "Guild hall" : "Seat"} · {h.home_name}
          </span>
          {scene && <span style={{ position: "absolute", left: 8, bottom: 8, background: K.chip, color: K.mu, border: `1px solid ${K.bd}`, borderRadius: 4, padding: "2px 9px", font: `400 10.5px/1.3 ${K.bf}` }}>{STYLES[style].label}</span>}
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 12, minWidth: 0 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            <span style={{ width: 44, height: 44, borderRadius: "50%", flex: "none", display: "flex", alignItems: "center", justifyContent: "center",
              background: color, color: "#fff8f0", font: `700 17px ${K.hf}`, boxShadow: `0 0 0 3px ${K.bd}` }}>{initials}</span>
            <div style={{ minWidth: 0 }}>
              <div style={{ font: `700 18px/1.15 ${K.hf}`, color: K.tx }}>{h.head_name}</div>
              <div style={{ font: `400 11.5px ${K.bf}`, color: K.mu }}>
                {guild ? `sitting head · acts for ${h.home_name}` : `${h.head_female ? "♀" : "♂"} · generation ${h.generation} · ${h.head_age > 0 ? `has ruled ${h.head_age} yrs` : "newly acceded"}`}
              </div>
            </div>
            <span style={{ flex: 1 }} />
            {h.archetype_perk && <Pill>{h.archetype_perk}</Pill>}
          </div>
          {st ? (
            <>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(5, minmax(0, 1fr))", gap: 8 }}>
                {st.gauges.map((g) => (
                  <div key={g.key} title={g.phrase} style={{ background: K.card, border: `1px solid ${g.warn ? K.neg : K.bd}`, borderRadius: 6, padding: "8px 10px", display: "flex", flexDirection: "column", gap: 5, minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                      <span style={{ font: `600 9.5px ${K.bf}`, letterSpacing: .5, textTransform: "uppercase", color: K.fa, whiteSpace: "nowrap" }}>{g.label}</span>
                      <span style={{ marginLeft: "auto", font: `700 16px ${K.hf}`, color: K.tx }}>{Math.round(g.score * 100)}</span>
                    </div>
                    <div style={{ display: "flex" }}><Bar frac={g.score} color={gcol(g.score)} h={6} /></div>
                    <div style={{ font: `400 10px ${K.bf}`, color: K.fa, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{g.phrase}</div>
                  </div>
                ))}
              </div>
              <div style={{ display: "flex", gap: 18, flexWrap: "wrap", font: `400 11px ${K.bf}`, color: K.fa }}>
                <span>Liquid <b style={{ color: K.tx }}>{fk(st.liquid)}</b></span>
                <span>Burn <b style={{ color: K.tx }}>{fk(st.monthly_burn)}</b> / mo</span>
                {st.liabilities.map((l) => <span key={l.label} title={l.note}>{l.label} <b style={{ color: K.neg }}>{fk(l.amount)}</b></span>)}
                <span>Liabilities <b style={{ color: st.liabilities_total > 0 ? K.neg : K.tx }}>{fk(st.liabilities_total)}</b></span>
                {st.debt_months > 0 && <span>In the red <b style={{ color: K.neg }}>{st.debt_months} of {st.debt_limit} mo</b></span>}
                {st.feuds_live > 0 && <span>Feuds <b style={{ color: st.feuds_hot > 0 ? K.neg : K.tx }}>{st.feuds_live}{st.feuds_hot > 0 ? ` · ${st.feuds_hot} hot` : ""}</b></span>}
              </div>
            </>
          ) : <Empty>Reading the house's standing…</Empty>}
        </div>
      </div>

      <CardGrid>
        <HeadsCard h={h} chron={chron} guild={guild} />
        <AccountsCard h={h} ledger={ledger} />
        <FleetCard h={h} ledger={ledger} style={style} />
        <Card title="Estates & manufactories" meta={`${h.estates?.length ?? 0} holdings`}>
          {(h.estates ?? []).length === 0 ? <Empty>No estates or manufactories owned.</Empty> : (h.estates ?? []).slice(0, 10).map(([good, city], i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
              <GoodIcon name={good} size={28} />
              <div style={{ minWidth: 0, flex: 1 }}>
                <div style={{ font: `700 13px/1.15 ${K.hf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{gl(good)}</div>
                <div style={{ font: `400 10.5px ${K.bf}`, color: K.fa }}>{city}</div>
              </div>
            </div>
          ))}
          {(h.estates?.length ?? 0) > 10 && <Note>…and {(h.estates?.length ?? 0) - 10} more.</Note>}
        </Card>
        <Card title="Monopolies & charters" meta={h.mono_ever_count ? `${h.mono_ever_count} held over the house's life` : undefined}>
          {h.monopolies.length === 0 ? <Empty>No monopoly held today.</Empty> : h.monopolies.map(([good, sh]) => (
            <div key={good} style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
              <GoodIcon name={good} size={24} />
              <span style={{ width: 96, flex: "none", font: `600 11.5px ${K.bf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{gl(good)}</span>
              <Bar frac={sh} color={sh > .5 ? K.ac : K.fa} h={7} />
              <span style={{ width: 36, textAlign: "right", font: `700 14px ${K.hf}`, color: K.tx }}>{Math.round(sh * 100)}%</span>
            </div>
          ))}
          {(h.charters ?? []).length > 0 && (
            <>
              <Sub>Charters</Sub>
              {(h.charters ?? []).map((c) => (
                <div key={c} style={{ font: `400 11.5px ${K.bf}`, color: K.tx, display: "flex", gap: 6, alignItems: "center" }}><span style={{ color: K.ac }}>◆</span>{c}</div>
              ))}
            </>
          )}
          <Sub>Rivals</Sub>
          {h.rivals.length === 0 ? <Empty>No standing rivalries.</Empty>
            : <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>{h.rivals.map((r) => <Pill key={r}>{r}</Pill>)}</div>}
        </Card>
        <ReachCard h={h} />
        <KnownForCard h={h} />
      </CardGrid>
    </WindowFrame>
  );
}

function HeadsCard({ h, chron, guild }: { h: HouseBrief; chron: HouseHistory | null; guild: boolean }) {
  const line = (chron?.line ?? []).slice(-6);
  const events = [...(chron?.events ?? [])].sort((a, b) => b.year - a.year).slice(0, 5);
  return (
    <Card title={guild ? "Head of the guild" : "Heads of the house"} meta={guild ? undefined : `${chron?.line?.length ?? 0} generations`}>
      {guild || line.length === 0 ? (
        <div style={{ display: "grid", gridTemplateColumns: "16px minmax(0,1fr)", gap: 8, alignItems: "center" }}>
          <span style={{ width: 10, height: 10, borderRadius: "50%", background: K.ac, boxShadow: `0 0 0 2px ${K.acBd}`, justifySelf: "center" }} />
          <div style={{ font: `700 13px/1.2 ${K.hf}`, color: K.tx }}>{h.head_name}
            <div style={{ font: `400 10.5px ${K.bf}`, color: K.fa }}>{guild ? "sitting head — the guild's elected wardens are not recorded" : "current head"}</div>
          </div>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column" }}>
          {line.map((p, k) => {
            const now = !p.until_year;
            const role = [p.accession, p.epithet].filter(Boolean).join(" · ");
            return (
              <div key={`${p.name}-${p.since_year}`} style={{ display: "grid", gridTemplateColumns: "16px minmax(0,1fr) auto", gap: 8, alignItems: "start", padding: "5px 0" }}>
                <div style={{ position: "relative", height: "100%", display: "flex", justifyContent: "center" }}>
                  {k < line.length - 1 && <span style={{ position: "absolute", top: 10, bottom: -12, width: 2, background: K.bd }} />}
                  <span style={{ position: "relative", marginTop: 3, width: 10, height: 10, borderRadius: "50%", background: now ? K.ac : K.bar, boxShadow: `0 0 0 2px ${now ? K.acBd : K.bd}` }} />
                </div>
                <div style={{ minWidth: 0 }}>
                  <div style={{ font: `700 13px/1.2 ${K.hf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{p.name}</div>
                  <div style={{ font: `400 10.5px ${K.bf}`, color: K.fa }}>{role || `generation ${p.generation}`}</div>
                </div>
                <span style={{ font: `400 10.5px ${K.bf}`, color: now ? K.ac : K.mu, whiteSpace: "nowrap", paddingTop: 2 }}>
                  {now ? `since ${p.since_year}` : `${p.since_year}–${p.until_year}`}
                </span>
              </div>
            );
          })}
        </div>
      )}
      {events.length > 0 && (
        <>
          <Sub>Chronicle</Sub>
          {events.map((e, i) => (
            <div key={i} style={{ display: "grid", gridTemplateColumns: "36px minmax(0,1fr)", gap: 6, font: `400 10.5px/1.35 ${K.bf}`, color: K.mu }}>
              <b style={{ color: K.tx }}>{e.year}</b><span>{e.text}</span>
            </div>
          ))}
        </>
      )}
    </Card>
  );
}

function AccountsCard({ h, ledger: L }: { h: HouseBrief; ledger: HouseLedger | null }) {
  if (!L || L.year <= 0) {
    return <Card span={2} title="Yearly accounts"><Empty>No completed year yet — the first year's books appear after a full year passes.</Empty></Card>;
  }
  const trade = [...L.trade_profit].sort((a, b) => Math.abs(b.amount) - Math.abs(a.amount));
  const rest = trade.slice(4).reduce((s, x) => s + x.amount, 0);
  const sum = (ls: { amount: number }[]) => ls.reduce((s, x) => s + x.amount, 0);
  const inc: [string, number][] = [...trade.slice(0, 4).map((x) => [`Trade · ${x.label}`, x.amount] as [string, number]),
    ...(trade.length > 4 ? [["Trade · elsewhere", rest] as [string, number]] : []),
    ["Office income", L.office_income], ["Estate income", L.estate_income]];
  const exp: [string, number][] = [["Import tax", sum(L.import_tax)], ["Export tax", sum(L.export_tax)], ["Estate tax", L.estate_tax],
    ["Upkeep", L.upkeep], ["Fleet cost", L.fleet_cost], ["Lost cargo", L.lost_cargo], ["Household & events", L.consumption + L.events],
    ["Inflation", L.inflation], ["War levy", L.war_levy], ["War damage", L.war_damage]];
  const mx = Math.max(1e-6, ...inc.map((x) => Math.abs(x[1])), ...exp.map((x) => Math.abs(x[1])));
  const wy = L.wealth_years;
  const peakYear = h.peak_wealth_tick != null ? Math.floor(h.peak_wealth_tick / 365) : null;
  return (
    <Card span={2} title={`Yearly accounts · ${L.year}`} meta="last completed year · grain-eq">
      <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 18 }}>
        {([["Income", inc, L.income_total, K.pos], ["Expenditure", exp, L.expense_total, K.neg]] as [string, [string, number][], number, string][]).map(([title, rows, tot, col]) => (
          <div key={title} style={{ display: "flex", flexDirection: "column", gap: 5, minWidth: 0 }}>
            <Sub>{title}</Sub>
            {rows.map(([n, v]) => (
              <div key={n} style={{ display: "grid", gridTemplateColumns: "minmax(0,1fr) 90px 50px", gap: 8, alignItems: "center", opacity: Math.abs(v) > 0.05 ? 1 : .45 }}>
                <span style={{ font: `400 11.5px ${K.bf}`, color: K.mu, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{n}</span>
                <Bar frac={Math.abs(v) / mx} color={v < 0 ? (col === K.pos ? K.neg : K.pos) : col} h={6} />
                <span style={{ textAlign: "right", font: `600 11.5px ${K.bf}`, color: K.tx }}>{Math.abs(v) > 0.05 ? fc(v) : "–"}</span>
              </div>
            ))}
            <div style={{ display: "flex", borderTop: `1px solid ${K.bd}`, paddingTop: 5, marginTop: "auto", font: `600 11.5px ${K.bf}`, color: K.mu }}>
              Total<span style={{ marginLeft: "auto", font: `700 15px ${K.hf}`, color: col }}>{fc(tot)}</span>
            </div>
          </div>
        ))}
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "200px minmax(0, 1fr)", gap: 16, alignItems: "center", borderTop: `1px solid ${K.bd}`, paddingTop: 10 }}>
        <div>
          <div style={{ font: `600 9.5px ${K.bf}`, letterSpacing: .5, textTransform: "uppercase", color: K.fa }}>Net for the year</div>
          <div style={{ font: `700 28px/1.1 ${K.hf}`, color: L.net >= 0 ? K.pos : K.neg }}>{L.net >= 0 ? "+" : "−"}{fc(Math.abs(L.net))}</div>
          {h.wealth > 0 && <div style={{ font: `400 10.5px ${K.bf}`, color: K.fa }}>{(L.net / h.wealth * 100).toFixed(1)}% on wealth</div>}
        </div>
        <div style={{ minWidth: 0 }}>
          <div style={{ display: "flex", font: `600 9.5px ${K.bf}`, letterSpacing: .5, textTransform: "uppercase", color: K.fa, marginBottom: 4 }}>
            Wealth{wy.length > 1 ? ` · ${L.wealth_start_year}–${L.wealth_start_year + wy.length - 1}` : ""}
            {h.peak_wealth != null && h.peak_wealth > 0 && <span style={{ marginLeft: "auto", textTransform: "none", letterSpacing: 0, fontWeight: 400 }}>
              peak {fc(h.peak_wealth)}{peakYear != null ? ` in ${peakYear}` : ""}</span>}
          </div>
          {wy.length > 1 ? <Spark vals={wy} color={K.pos} w={520} h={44} /> : <Empty>One year recorded so far.</Empty>}
        </div>
      </div>
    </Card>
  );
}

function FleetCard({ h, ledger, style }: { h: HouseBrief; ledger: HouseLedger | null; style: StyleKey }) {
  const names = HULL_NAMES[style], icons = VESSEL_ICONS[style];
  const rows: ["sea" | "river" | "caravan", number, string][] = [
    ["sea", h.fleet_sea ?? 0, K.sea], ["river", h.fleet_river ?? 0, K.river], ["caravan", h.fleet_caravan ?? 0, K.land]];
  const tot = rows.reduce((s, r) => s + r[1], 0);
  const mx = Math.max(1, ...rows.map((r) => r[1]));
  return (
    <Card title="Fleet · concurrent cargo slots" meta={`${tot} ${tot === 1 ? "hull" : "hulls"}`}>
      {rows.map(([k, n, col]) => (
        <div key={k} style={{ display: "flex", alignItems: "center", gap: 10, opacity: n ? 1 : .45 }}>
          <Blit src={vesselIcon(icons[k], style, 40)} size={40} style={{ borderRadius: 6 }} />
          <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 4 }}>
            <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
              <span style={{ font: `700 13px ${K.hf}`, color: K.tx }}>{names[k]}</span><span style={{ flex: 1 }} />
              <b style={{ font: `700 16px ${K.hf}`, color: K.tx }}>{n}</b>
            </div>
            <div style={{ display: "flex" }}><Bar frac={n / mx} color={col} h={7} /></div>
          </div>
        </div>
      ))}
      {ledger && ledger.year > 0 && (
        <div style={{ font: `400 10.5px ${K.bf}`, color: K.fa, borderTop: `1px solid ${K.bd}`, paddingTop: 8 }}>
          Fleet upkeep <b style={{ color: K.tx }}>{fc(ledger.fleet_cost)}</b> / yr · lost cargo <b style={{ color: K.neg }}>{fc(ledger.lost_cargo)}</b>
        </div>
      )}
      <Note>Hull names are the seat city's building tradition; a hull is a counted slot, not an entity.</Note>
    </Card>
  );
}

function ReachCard({ h }: { h: HouseBrief }) {
  const cols = "minmax(0,1fr) 84px minmax(0,1.4fr) 44px 80px";
  const active = (h.active ?? []).slice(0, 12);
  return (
    <Card span={2} title="Seats, offices & reach" meta="ranked by influence">
      {active.length === 0 ? <Empty>{h.cities?.length ? h.cities.slice(0, 10).join(" · ") : "Not active in any city yet."}</Empty> : (
        <>
          <HdrRow cols={cols} labels={[["City"], ["Role"], ["Influence"], ["", true], [""]]} />
          {active.map((c) => (
            <div key={c.name} style={{ display: "grid", gridTemplateColumns: cols, gap: 10, alignItems: "center" }}>
              <span style={{ font: `700 13px ${K.hf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{c.name}</span>
              <span><Tag color={ROLE_COLOR[c.role] ?? K.mu}>{c.role}</Tag></span>
              <Bar frac={c.influence} color={c.role === "seat" || c.role === "dominant" ? K.ac : K.sea} h={7} />
              <span style={{ textAlign: "right", font: `600 11.5px ${K.bf}`, color: K.tx }}>{Math.round(c.influence * 100)}%</span>
              <span style={{ font: `400 10.5px ${K.bf}`, color: K.neg }}>{c.contested ? "⚠ contested" : ""}</span>
            </div>
          ))}
          {(h.active?.length ?? 0) > 12 && <Note>…and {(h.active?.length ?? 0) - 12} more cities.</Note>}
        </>
      )}
      {h.barred && h.barred.length > 0 && (
        <Note>Barred from <b style={{ color: K.neg }}>{h.barred.join(", ")}</b> while the trade war lasts.</Note>
      )}
    </Card>
  );
}

function KnownForCard({ h }: { h: HouseBrief }) {
  const rows = [...(h.goods_ledger ?? [])].sort((a, b) => b.volume - a.volume).slice(0, 6);
  const mx = Math.max(1e-6, ...rows.map((r) => Math.abs(r.profit)));
  return (
    <Card title="What the house is known for" meta={rows.length ? "cumulative volume · profit" : undefined}>
      {rows.length === 0 ? (
        h.top_goods.length === 0 ? <Empty>No trade on record yet.</Empty>
          : <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>{h.top_goods.slice(0, 6).map((g) => (
            <span key={g} style={{ display: "inline-flex", alignItems: "center", gap: 5, font: `600 11.5px ${K.bf}`, color: K.tx }}><GoodIcon name={g} size={22} />{gl(g)}</span>
          ))}</div>
      ) : rows.map((r) => (
        <div key={r.good} style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
          <GoodIcon name={r.good} size={24} />
          <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 3 }}>
            <div style={{ display: "flex", font: `600 11.5px ${K.bf}`, color: K.tx }}>
              {gl(r.good)}<span style={{ marginLeft: "auto", font: `400 10.5px ${K.bf}`, color: K.fa }}>{fk(r.volume)} moved</span>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <Bar frac={Math.abs(r.profit) / mx} color={r.profit >= 0 ? K.pos : K.neg} h={6} />
              <span style={{ width: 44, textAlign: "right", font: `600 11px ${K.bf}`, color: r.profit >= 0 ? K.pos : K.neg }}>{r.profit >= 0 ? "+" : "−"}{fk(Math.abs(r.profit))}</span>
            </div>
          </div>
        </div>
      ))}
    </Card>
  );
}
