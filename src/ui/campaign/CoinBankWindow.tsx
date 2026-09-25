import { useEffect, useState } from "react";
import type { BankBrief, CoinLedgerSummary, CoinSnapshot, CoinUseCity, HubDetail, MintBrief, MonetaryEvent } from "@types";
import {
  campaignGetMints, campaignCoinUsage, campaignMonetaryChronicle, campaignGetBanks, campaignGetCoinCatalogue,
  campaignCoinHistory, campaignGetHub,
} from "@bridge";
import { useCampaignStore } from "@state/campaignStore";
import { useUIStore } from "@state/uiStore";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { GoodIcon } from "@ui/goods/GoodIcon";
import { GOOD_DEFS } from "@goods";
import { CoinIcon, type CoinMetal } from "@ui/heraldry/CoinIcon";
import { landmarkIcon, toHex } from "@canvas/cityArt";
import { pickFamily } from "@ui/campaign/settlementWindowData";
import {
  K, fk, fc, gcol, Card, CardGrid, Bar, Stack, Tag, Pill, Swatch, Sub, HdrRow, Note, Empty, Blit, MarkChart,
  WindowFrame, WIDE_WINDOW, useCultureKits,
} from "@ui/campaign/windowKit";

// ── The coin & bank window (design handoff "Turn 5", 5c) ──────────────────────
// One coin's biography (fineness / trust / value with event markers), where
// struck coin sits, the cities that settle in it, its monetary chronicle, and
// the bank that banks in it — balance sheet, stakes, branches and loan book.
// Not in data and so not drawn: the sim's run-risk threshold as a tick on the
// reserve-ratio gauge, and a per-coin (rather than world-wide) holder ledger.

const METAL: Record<string, [string, string]> = {
  gold: ["#e8c452", "#8a6a1c"], silver: ["#c4cdd8", "#6a7686"], electrum: ["#dcd083", "#7c7234"], bronze: ["#c07f45", "#6a4020"],
};
const BAD_EVENT = new Set(["debasement", "run", "crash"]);
const PURPOSE: Record<string, string> = {
  trade: "trade venture", guild_factory: "guild factory", guild_civic: "guild civic works",
  treasury: "public works", colony: "colony venture", estate: "estate", structure: "building", mine: "mine",
};
const KIND_COLOR: Record<string, string> = { house: "#c8813a", guild: "#6fc3b0", polis: "#5a8ac8" };
const GOOD_BY_NAME = new Map(GOOD_DEFS.map((g) => [g.name, g]));

/** The floating host: open with `useUIStore.setCoinWindow(coinName)`. */
export function CoinBankWindow() {
  const coinName = useUIStore((s) => s.coinWindow);
  const tick = useCampaignStore((s) => s.snapshot?.clock?.tick ?? 0);
  const active = useCampaignStore((s) => !!s.snapshot?.active);
  const [mints, setMints] = useState<MintBrief[]>([]);
  const [usage, setUsage] = useState<CoinUseCity[]>([]);
  const [chron, setChron] = useState<MonetaryEvent[]>([]);
  const [banks, setBanks] = useState<BankBrief[]>([]);
  const [ledger, setLedger] = useState<CoinLedgerSummary | null>(null);
  const year = Math.floor(tick / 365);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.coin);

  useEffect(() => {
    if (!coinName || !active) return;
    let alive = true;
    campaignGetMints().then((m) => { if (alive) setMints(m); }).catch(() => {});
    campaignCoinUsage().then((u) => { if (alive) setUsage(u); }).catch(() => {});
    campaignMonetaryChronicle().then((c) => { if (alive) setChron(c); }).catch(() => {});
    campaignGetBanks().then((b) => { if (alive) setBanks(b); }).catch(() => {});
    campaignGetCoinCatalogue().then((c) => { if (alive) setLedger(c.ledger); }).catch(() => {});
    return () => { alive = false; };
  }, [coinName, active, year]);

  if (!coinName) return null;
  const close = () => useUIStore.getState().setCoinWindow(null);
  const mint = mints.find((m) => m.coin_name === coinName);
  return (
    <div data-draggable onPointerDown={onPointerDown}
      style={{ position: "absolute", top: 50, right: 12, zIndex: 46, borderRadius: 8, boxShadow: "0 8px 28px rgba(0,0,0,0.55)",
        ...rootStyle, width: WIDE_WINDOW, maxHeight: "90vh", overflowY: "auto", padding: 6 }}>
      {mint ? (
        <CoinWindow mint={mint} usage={usage} chron={chron} banks={banks} ledger={ledger} year={year} onClose={close} onDragStart={onPointerDown} />
      ) : (
        <WindowFrame name={coinName} mark={null} stats={[]} onClose={close} onDragStart={onPointerDown}>
          <div style={{ padding: 18, color: K.fa }}>{mints.length ? "This coin is no longer struck by any mint." : "Reading the mints…"}</div>
        </WindowFrame>
      )}
    </div>
  );
}

function CoinWindow({ mint: m, usage, chron, banks, ledger, year, onClose, onDragStart }: {
  mint: MintBrief; usage: CoinUseCity[]; chron: MonetaryEvent[]; banks: BankBrief[]; ledger: CoinLedgerSummary | null;
  year: number; onClose: () => void; onDragStart: (e: React.PointerEvent<HTMLElement>) => void;
}) {
  const [hist, setHist] = useState<CoinSnapshot[]>([]);
  const [seat, setSeat] = useState<HubDetail | null>(null);
  const kits = useCultureKits();
  useEffect(() => {
    let alive = true;
    campaignCoinHistory(m.hub).then((h) => { if (alive) setHist(h); }).catch(() => { if (alive) setHist([]); });
    campaignGetHub(m.hub).then((d) => { if (alive) setSeat(d); }).catch(() => { if (alive) setSeat(null); });
    return () => { alive = false; };
  }, [m.hub, year]);

  const [metalCol] = METAL[m.metal] ?? METAL.gold;
  const cities = usage.filter((u) => u.coin === m.hub)
    .sort((a, b) => Number(b.mint) - Number(a.mint) || Number(b.primary) - Number(a.primary) || b.share - a.share);
  const main = cities.filter((c) => c.primary || c.mint).length;
  const events = chron.filter((e) => e.city === m.city);
  const issuers = banks.filter((b) => !b.defunct && b.coin_name === m.coin_name)
    .sort((a, b) => Number(b.seat === m.city) - Number(a.seat === m.city) || b.equity - a.equity);
  const bank = issuers[0];
  const kit = seat?.culture ? kits?.get(seat.culture) : undefined;
  const style = pickFamily(seat?.koppen ?? 0, kit, seat?.coastal ?? false);

  const years = hist.map((h) => h.year);
  const marks = hist.flatMap((h, k) => h.event ? [[k, BAD_EVENT.has(h.event)] as [number, boolean]] : []);
  const series: [string, number[], (v: number) => string, string, string][] = [
    ["Fineness", hist.map((h) => h.fineness), (v) => (v * 100).toFixed(1) + "%", "metal content of the struck coin", metalCol],
    ["Trust", hist.map((h) => h.trust), (v) => Math.round(v * 100) + "%", "merchants' willingness to hold", K.pos],
    ["Value", hist.map((h) => h.value), (v) => v.toFixed(2) + "×", "in grain-equivalent", K.ac],
  ];

  return (
    <WindowFrame onClose={onClose} onDragStart={onDragStart}
      mark={<CoinIcon issuer={m.issuer || m.city} value={m.value} metal={(METAL[m.metal] ? m.metal : "gold") as CoinMetal} size={26} />}
      name={m.coin_name} tag={`${m.metal.toUpperCase()} · MINT OF ${m.city.toUpperCase()}`}
      sub={<>Settles {main} {main === 1 ? "city" : "cities"}{cities.length > main ? ` · reserve coin in ${cities.length - main} more` : ""}
        {years.length > 0 && <span style={{ color: K.fa }}> · first struck {years[0]}</span>}</>}
      stats={[["Value", `${m.value.toFixed(2)}× grain`, K.ac], ["Trust", `${Math.round(m.trust * 100)}%`], ["Fineness", `${(m.fineness * 100).toFixed(1)}%`]]}>

      {/* ── biography band ── */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(3, minmax(0, 1fr)) minmax(0, 1.15fr)", gap: 12, padding: "14px 16px", background: K.head, borderBottom: `1px solid ${K.bd}` }}>
        {series.map(([l, vals, f, note, col]) => {
          const d = vals.length ? vals[vals.length - 1] - vals[0] : 0;
          return (
            <div key={l} style={{ background: K.card, border: `1px solid ${K.bd}`, borderRadius: 6, padding: "10px 12px", display: "flex", flexDirection: "column", gap: 6, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
                <span style={{ font: `600 10px ${K.bf}`, letterSpacing: .7, textTransform: "uppercase", color: K.fa }}>{l}</span>
                <span style={{ flex: 1 }} />
                {vals.length > 1 && <span style={{ font: `400 10.5px ${K.bf}`, color: d >= 0 ? K.pos : K.neg }}>{d >= 0 ? "+" : "−"}{f(Math.abs(d))} since {years[0]}</span>}
              </div>
              <div style={{ font: `700 24px/1 ${K.hf}`, color: K.tx }}>{f(l === "Fineness" ? m.fineness : l === "Trust" ? m.trust : m.value)}</div>
              <MarkChart vals={vals} color={col} w={244} h={46} marks={marks} />
              <div style={{ display: "flex", justifyContent: "space-between", font: `400 9.5px ${K.bf}`, color: K.fa, gap: 6 }}>
                <span>{years[0] ?? ""}</span><span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{note}</span><span>{years[years.length - 1] ?? ""}</span>
              </div>
            </div>
          );
        })}
        <WhereItSits ledger={ledger} metalCol={metalCol} />
      </div>

      <CardGrid>
        <Card span={2} title={`Cities settling in the ${m.coin_name}`} meta="share of each city's circulating basket">
          {cities.length === 0 ? <Empty>No city settles in this coin yet.</Empty> : (
            <>
              <HdrRow cols="minmax(0,1fr) 84px minmax(0,1.3fr) 64px" labels={[["City"], ["Role"], ["Basket share"], ["Settled / yr", true]]} />
              {cities.slice(0, 14).map((c) => {
                const role = c.mint ? "mint · main" : c.primary ? "main" : "reserve";
                const rc = c.mint ? K.ac : c.primary ? K.pos : K.fa;
                return (
                  <div key={c.city} style={{ display: "grid", gridTemplateColumns: "minmax(0,1fr) 84px minmax(0,1.3fr) 64px", gap: 10, alignItems: "center" }}>
                    <span style={{ font: `700 13px ${K.hf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{c.name}</span>
                    <span><Tag color={rc}>{role}</Tag></span>
                    <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
                      <div style={{ flex: 1 }}><Stack h={8} parts={[[c.share, c.primary || c.mint ? metalCol : K.fa], [Math.max(0, 1 - c.share), K.bar]]} /></div>
                      <span style={{ width: 34, textAlign: "right", font: `600 11px ${K.bf}`, color: K.tx }}>{Math.round(c.share * 100)}%</span>
                    </div>
                    <span style={{ textAlign: "right", font: `400 11.5px ${K.bf}`, color: K.mu }}>{fk(c.volume)}</span>
                  </div>
                );
              })}
              {cities.length > 14 && <Note>…and {cities.length - 14} more.</Note>}
            </>
          )}
          <div style={{ display: "flex", gap: 14, font: `400 10.5px ${K.bf}`, color: K.fa, flexWrap: "wrap" }}>
            <span>main = the city settles its own trade in this coin</span>
            <span>reserve = held by its merchants, not the settling coin</span>
            <span style={{ marginLeft: "auto" }}>price level ×{m.price_level.toFixed(2)} · {m.exchange.toFixed(1)} ag specie</span>
          </div>
        </Card>

        <Card title="Monetary chronicle" meta={`${events.length} ${events.length === 1 ? "entry" : "entries"}`}>
          {events.length === 0 ? <Empty>Nothing notable yet at {m.city}'s mint.</Empty>
            : events.slice(0, 9).map((e, i) => {
              const bad = BAD_EVENT.has(e.kind) || /debas/i.test(e.text);
              return (
                <div key={i} style={{ display: "grid", gridTemplateColumns: "40px minmax(0,1fr)", gap: 8, alignItems: "start" }}>
                  <span style={{ font: `700 13px/1.2 ${K.hf}`, color: K.tx }}>{e.year}</span>
                  <div style={{ display: "flex", flexDirection: "column", gap: 3, minWidth: 0 }}>
                    <span><Tag color={bad ? K.neg : K.pos}>{e.kind}</Tag></span>
                    <span style={{ font: `400 11px/1.35 ${K.bf}`, color: K.mu }}>{e.text}</span>
                  </div>
                </div>
              );
            })}
        </Card>

        {bank ? (
          <>
            <BankCard bank={bank} others={issuers.length - 1} style={style} metalCol={metalCol} />
            <StakesCard bank={bank} />
            <LoansCard bank={bank} metalCol={metalCol} />
          </>
        ) : (
          <Card span={3} title="Bank of issue"><Empty>No chartered bank banks in the {m.coin_name} yet.</Empty></Card>
        )}
      </CardGrid>
    </WindowFrame>
  );
}

function WhereItSits({ ledger, metalCol }: { ledger: CoinLedgerSummary | null; metalCol: string }) {
  const L = ledger;
  const parts: [string, number, string][] = L ? [
    ["City treasuries", L.in_city_treasuries, "#c8813a"], ["Households", L.in_households, "#6aa05a"], ["Local merchants", L.in_local_merchants, "#5a8ac8"],
    ["Banks & houses", Math.max(0, L.total_struck - L.in_city_treasuries - L.in_households - L.in_local_merchants), metalCol],
  ] : [];
  return (
    <div style={{ background: K.card, border: `1px solid ${K.bd}`, borderRadius: 6, padding: "10px 12px", display: "flex", flexDirection: "column", gap: 7, minWidth: 0 }}>
      <div style={{ display: "flex", alignItems: "baseline" }}>
        <span style={{ font: `600 10px ${K.bf}`, letterSpacing: .7, textTransform: "uppercase", color: K.fa }}>Where struck coin sits</span>
        <span style={{ marginLeft: "auto", font: `400 10px ${K.bf}`, color: K.fa }}>every mint</span>
      </div>
      {!L || L.total_struck <= 0 ? <Empty>No purses recorded yet.</Empty> : (
        <>
          <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
            <span style={{ font: `700 24px/1 ${K.hf}`, color: K.tx }}>{fk(L.total_struck)}</span>
            <span style={{ font: `400 10.5px ${K.bf}`, color: K.fa }}>struck in all</span>
          </div>
          <Stack h={10} parts={parts.map(([, v, c]) => [v, c] as [number, string])} />
          <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: "3px 10px" }}>
            {parts.map(([n, v, c]) => (
              <div key={n} style={{ font: `400 10.5px ${K.bf}`, color: K.mu, display: "flex", alignItems: "center", gap: 5 }}>
                <Swatch color={c} size={7} />{n}<span style={{ marginLeft: "auto", color: K.tx, fontWeight: 600 }}>{fk(v)}</span>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

function BankCard({ bank: b, others, style, metalCol }: { bank: BankBrief; others: number; style: ReturnType<typeof pickFamily>; metalCol: string }) {
  const infinite = b.reserve_ratio >= 99;
  const A: [string, number][] = [["Reserves", b.reserves], ["Loans out", b.loans_out], ["Manufactory stakes", b.stake_book], ["Real estate", b.real_estate]];
  const L: [string, number][] = [["Deposits", b.deposits], ["Notes issued", b.notes_issued], ["Equity", b.equity]];
  const mx = Math.max(1e-6, ...A.map((x) => x[1]), ...L.map((x) => x[1]));
  const eq = b.history.map((h) => h.equity);
  return (
    <Card span={2} title={`Bank of issue · ${b.name}`} meta={`founded ${b.founded_year} · ${b.n_loans} loans${others > 0 ? ` · ${others} other ${others === 1 ? "bank" : "banks"} in this coin` : ""}`}>
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <Blit src={landmarkIcon("Bank", style, "op", toHex(b.color), 46)} size={46} />
        <div style={{ minWidth: 0, flex: 1 }}>
          <div style={{ font: `700 16px/1.15 ${K.hf}`, color: K.tx }}>{b.name}</div>
          <div style={{ font: `400 11px ${K.bf}`, color: K.mu, display: "flex", gap: 5, alignItems: "center" }}>
            <Swatch color={toHex(b.color)} size={7} round /> owned by {b.owner} · seated at {b.seat}
          </div>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 4, width: 210 }}>
          <div style={{ display: "flex", font: `400 10.5px ${K.bf}`, color: K.fa }}>Reserve ratio
            <span style={{ marginLeft: "auto", color: K.tx, fontWeight: 600 }}>{infinite ? "∞" : `${Math.round(b.reserve_ratio * 100)}%`}</span></div>
          <div style={{ display: "flex" }}><Bar frac={infinite ? 1 : b.reserve_ratio} color={gcol(infinite ? 1 : b.reserve_ratio * 1.6)} h={8} /></div>
          <div style={{ font: `400 9.5px ${K.bf}`, color: K.fa }}>specie against deposits + notes</div>
        </div>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 18, borderTop: `1px solid ${K.bd}`, paddingTop: 10 }}>
        {([["Assets", A, metalCol], ["Liabilities & equity", L, K.sea]] as [string, [string, number][], string][]).map(([title, rows, col]) => (
          <div key={title} style={{ display: "flex", flexDirection: "column", gap: 6, minWidth: 0 }}>
            <Sub meta={<b style={{ color: K.tx }}>{fc(rows.reduce((s, x) => s + x[1], 0))}</b>}>{title}</Sub>
            {rows.map(([n, v]) => (
              <div key={n} style={{ display: "grid", gridTemplateColumns: "120px minmax(0,1fr) 56px", gap: 8, alignItems: "center" }}>
                <span style={{ font: `400 11.5px ${K.bf}`, color: K.mu }}>{n}</span>
                <Bar frac={Math.max(0, v) / mx} color={n === "Equity" ? (v >= 0 ? K.pos : K.neg) : col} h={7} />
                <span style={{ textAlign: "right", font: `600 11.5px ${K.bf}`, color: v < 0 ? K.neg : K.tx }}>{fk(v)}</span>
              </div>
            ))}
          </div>
        ))}
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr)) minmax(0, 1.4fr)", gap: 10, borderTop: `1px solid ${K.bd}`, paddingTop: 10, alignItems: "end" }}>
        {([["Interest earned", b.interest_earned, false], ["Dividends", b.dividends_earned, false], ["Bills of exchange", b.bills_income, false], ["Written off", b.losses, true]] as [string, number, boolean][]).map(([l, v, bad]) => (
          <div key={l}>
            <div style={{ font: `600 9.5px ${K.bf}`, letterSpacing: .5, textTransform: "uppercase", color: K.fa }}>{l}</div>
            <div style={{ font: `700 17px/1.2 ${K.hf}`, color: bad ? K.neg : K.pos }}>{bad ? "−" : "+"}{fk(v)}</div>
            <div style={{ font: `400 10px ${K.bf}`, color: K.fa }}>cumulative</div>
          </div>
        ))}
        <div style={{ minWidth: 0 }}>
          <div style={{ display: "flex", font: `600 9.5px ${K.bf}`, letterSpacing: .5, textTransform: "uppercase", color: K.fa, marginBottom: 2 }}>
            Equity since {b.history[0]?.year ?? b.founded_year}
            {eq.length > 0 && <span style={{ marginLeft: "auto", textTransform: "none", letterSpacing: 0, fontWeight: 400 }}>{fk(eq[0])} → {fk(b.equity)}</span>}
          </div>
          <MarkChart vals={eq} color={K.pos} w={170} h={40} />
        </div>
      </div>
    </Card>
  );
}

function StakesCard({ bank: b }: { bank: BankBrief }) {
  return (
    <Card title="Stakes & branches" meta={`stake book ${fk(b.stake_book)}`}>
      {b.stakes.length === 0 ? <Empty>No manufactory stakes held.</Empty> : b.stakes.map((s, i) => (
        <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
          <GoodIcon name={s.good} size={24} />
          <div style={{ minWidth: 0, flex: 1 }}>
            <div style={{ font: `600 11.5px ${K.bf}`, color: K.tx, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{s.works}</div>
            <div style={{ font: `400 10px ${K.bf}`, color: K.fa }}>{GOOD_BY_NAME.get(s.good)?.label ?? s.good} · basis {fk(s.basis)}</div>
          </div>
          <span style={{ font: `700 14px ${K.hf}`, color: K.tx }}>{Math.round(s.share * 100)}%</span>
        </div>
      ))}
      <Sub meta={`${b.branches.length} ${b.branches.length === 1 ? "city" : "cities"}`}>Branch houses</Sub>
      {b.branches.length === 0 ? <Empty>Only the seat at {b.seat}.</Empty>
        : <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>{b.branches.map((br) => <Pill key={br}>{br}</Pill>)}</div>}
    </Card>
  );
}

function LoansCard({ bank: b, metalCol }: { bank: BankBrief; metalCol: string }) {
  const cols = "minmax(0,1.2fr) 70px 120px 64px minmax(0,1fr) 62px 84px";
  const out = b.loans.reduce((s, l) => s + l.outstanding, 0);
  return (
    <Card span={3} title="Loans on the books" meta={`${fk(out)} outstanding · rates monthly`}>
      {b.loans.length === 0 ? <Empty>No loans on the books.</Empty> : (
        <>
          <HdrRow cols={cols} labels={[["Borrower"], ["Kind"], ["Purpose"], ["Principal", true], ["Repaid · outstanding"], ["Rate", true], ["Term", true]]} />
          {b.loans.map((l, i) => (
            <div key={i} style={{ display: "grid", gridTemplateColumns: cols, gap: 10, alignItems: "center" }}>
              <span style={{ font: `600 12px ${K.bf}`, color: K.tx, display: "flex", gap: 6, alignItems: "center", whiteSpace: "nowrap", overflow: "hidden", minWidth: 0 }}>
                <Swatch color={KIND_COLOR[l.borrower_kind] ?? K.mu} round />{l.borrower}
              </span>
              <span><Tag>{l.borrower_kind}</Tag></span>
              <span style={{ font: `400 11px ${K.bf}`, color: K.mu, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{PURPOSE[l.purpose] ?? l.purpose.replace("_", " · ")}</span>
              <span style={{ textAlign: "right", font: `400 11.5px ${K.bf}`, color: K.mu }}>{fk(l.principal)}</span>
              <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
                <div style={{ flex: 1 }}><Stack h={8} parts={[[Math.max(0, l.principal - l.outstanding), K.pos], [l.outstanding, metalCol]]} /></div>
                <span style={{ width: 40, font: `600 11px ${K.bf}`, color: K.tx, textAlign: "right" }}>{fk(l.outstanding)}</span>
              </div>
              <span style={{ textAlign: "right", font: `600 11.5px ${K.bf}`, color: K.tx }}>{(l.rate * 100).toFixed(1)}%</span>
              <span style={{ textAlign: "right", font: `400 11px ${K.bf}`, color: K.fa, whiteSpace: "nowrap" }}>{l.start_year}–{Math.round(l.start_year + l.term_years)}</span>
            </div>
          ))}
        </>
      )}
    </Card>
  );
}
