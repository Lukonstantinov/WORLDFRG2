import { useEffect, useState } from "react";
import { useCampaignStore } from "@state/campaignStore";
import { useUIStore } from "@state/uiStore";
import { campaignGetBanks } from "@bridge";
import type { BankBrief, BankSnapshot } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, RADIUS } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, Tabs, Meter, EmptyNote } from "@ui/kit";

/** Dedicated Bank panel. Pick a bank from the list (or arrive here by clicking its
 *  icon on the map) and inspect:
 *   • Overview — a balance-sheet schematic (assets vs liabilities + equity), the key
 *     ratios, and the coin it banks in.
 *   • Charts — yearly line graphs: net worth (equity), reserves/loans/stakes,
 *     deposits/notes, and income vs write-offs — so the REASONS behind a rise or
 *     fall are visible.
 *   • Deals — every live loan with its agreement terms, plus equity stakes held.
 *   • How it works — where a bank's money comes from and how to grow its net worth.
 *  All read straight from the live campaign sim. Built on the shared UI kit (kit.tsx). */
export function BankPanel() {
  const open = useUIStore((s) => s.showBank);
  const selIdx = useUIStore((s) => s.selectedBankIdx);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;
  const [banks, setBanks] = useState<BankBrief[]>([]);
  const [tab, setTab] = useState<"overview" | "charts" | "deals" | "info">("overview");

  useEffect(() => {
    if (!open || !active) return;
    campaignGetBanks().then(setBanks).catch(() => setBanks([]));
  }, [open, active, tick]);

  const { rootStyle, onPointerDown, dragRoot } = useFloatingWindow(PANEL_TINTS.bank);
  if (!open) return null;
  const close = () => useUIStore.getState().setShowBank(false);
  const pick = (i: number) => useUIStore.getState().setSelectedBankIdx(i);

  // The focused bank: the explicit selection if valid, else the soundest (first).
  const idx = selIdx != null && selIdx < banks.length ? selIdx : 0;
  const bank = banks[idx];

  return (
    <Panel {...dragRoot} width={430} maxHeight="82vh" style={{ top: 60, right: 360, zIndex: 40, ...rootStyle }}>
      <PanelHeader icon="🏦" title="Banks" onDragStart={onPointerDown} onClose={close} />

      {!active && <EmptyNote>Banks appear once a campaign is running.</EmptyNote>}
      {active && banks.length === 0 && (
        <EmptyNote>No banks chartered yet — the age of banking opens around year 20,
          once a wealthy house holds a trusted coin.</EmptyNote>
      )}

      {active && banks.length > 0 && (
        <div style={{ display: "flex", minHeight: 0, flex: 1 }}>
          {/* ── Bank list ── */}
          <div style={list}>
            {banks.map((b, i) => (
              <div key={i} onClick={() => pick(i)}
                style={{ ...listRow, background: i === idx ? T.accentSoft : "transparent",
                  opacity: b.defunct ? 0.5 : 1 }}>
                <span style={{ width: 8, height: 8, borderRadius: 8, background: b.color, flex: "0 0 auto" }} />
                <div style={{ minWidth: 0, flex: 1 }}>
                  <div style={{ color: i === idx ? T.ink : T.inkMid, fontSize: FZ.small,
                    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {b.name}{b.defunct ? " ✝" : ""}
                  </div>
                  <div style={{ color: T.inkDim, fontSize: FZ.micro }}>
                    {b.seat} · eq {fmt(b.equity)}
                  </div>
                </div>
              </div>
            ))}
          </div>

          {/* ── Detail ── */}
          <div style={{ display: "flex", flexDirection: "column", minWidth: 0, flex: 1 }}>
            {bank && (
              <>
                <div style={{ padding: "6px 8px 2px" }}>
                  <div style={{ color: T.ink, fontSize: FZ.base, fontWeight: 700 }}>
                    {bank.name}
                  </div>
                  <div style={{ color: T.inkMid, fontSize: FZ.tiny }}>
                    {bank.owner} · seat {bank.seat} · founded yr {bank.founded_year}
                    {bank.coin_name ? ` · banks in ${bank.coin_name}` : ""}
                  </div>
                </div>
                <Tabs<"overview" | "charts" | "deals" | "info">
                  tabs={[["overview", "Overview"], ["charts", "Charts"], ["deals", `Deals (${bank.loans.length + bank.stakes.length})`], ["info", "How it works"]]}
                  active={tab} onSelect={setTab} style={{ padding: "2px 8px 0" }}
                />
                <div style={scroll}>
                  {tab === "overview" && <Overview bank={bank} />}
                  {tab === "charts" && <Charts hist={bank.history} />}
                  {tab === "deals" && <Deals bank={bank} />}
                  {tab === "info" && <Info bank={bank} />}
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </Panel>
  );
}

/* ── Overview: balance-sheet schematic + ratios ── */
function Overview({ bank }: { bank: BankBrief }) {
  const assets = [
    { label: "Reserves (specie)", v: bank.reserves, c: "#e0c452" },
    { label: "Loans outstanding", v: bank.loans_out, c: "#5f97c0" },
    { label: "Equity stakes", v: bank.stake_book, c: "#6fae8a" },
    { label: "Property / branches", v: bank.real_estate, c: "#9a8aa0" },
  ].filter((r) => r.v > 0.01);
  const liabs = [
    { label: "Deposits", v: bank.deposits, c: "#c98f5f" },
    { label: "Notes in circulation", v: bank.notes_issued, c: "#b8728f" },
    { label: "Equity (net worth)", v: Math.max(bank.equity, 0), c: "#7fcf9f" },
  ].filter((r) => r.v > 0.01);
  const total = Math.max(assets.reduce((s, r) => s + r.v, 0), liabs.reduce((s, r) => s + r.v, 0), 1e-6);
  const rrPct = bank.reserve_ratio >= 99 ? "∞" : `${(bank.reserve_ratio * 100).toFixed(0)}%`;
  const fragile = bank.reserve_ratio < 0.22;

  return (
    <div>
      <div style={statGrid}>
        <Stat label="Net worth" value={fmt(bank.equity)} accent={T.goodInk} />
        <Stat label="Reserve ratio" value={rrPct} accent={fragile ? T.badInk : "#7fb0e0"} />
        <Stat label="Interest earned" value={fmt(bank.interest_earned)} />
        <Stat label="Dividends" value={fmt(bank.dividends_earned)} />
        <Stat label="Write-offs" value={fmt(bank.losses)} accent="#e0906a" />
        <Stat label="Branches" value={`${bank.branches.length}`} />
      </div>

      <SectionLabel>Balance sheet</SectionLabel>
      <div style={{ display: "flex", gap: 10 }}>
        <BalanceColumn title="Assets" rows={assets} total={total} />
        <BalanceColumn title="Liabilities + equity" rows={liabs} total={total} />
      </div>

      {bank.events.length > 0 && (
        <>
          <SectionLabel>Recent</SectionLabel>
          {bank.events.slice(0, 8).map((e, i) => (
            <div key={i} style={{ color: T.inkMid, fontSize: FZ.tiny, padding: "1px 0" }}>• {e}</div>
          ))}
        </>
      )}
    </div>
  );
}

function BalanceColumn({ title, rows, total }: { title: string; rows: { label: string; v: number; c: string }[]; total: number }) {
  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ color: T.inkMid, fontSize: FZ.tiny, marginBottom: 3 }}>{title}</div>
      {rows.length === 0 && <div style={{ color: T.inkFaint, fontSize: FZ.tiny }}>—</div>}
      {rows.map((r, i) => (
        <div key={i} style={{ marginBottom: 4 }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: FZ.tiny, color: T.inkMid }}>
            <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.label}</span>
            <span style={{ color: T.ink }}>{fmt(r.v)}</span>
          </div>
          <Meter value={r.v} max={total} color={r.c} height={6} track={T.raised} />
        </div>
      ))}
    </div>
  );
}

/* ── Charts: yearly line graphs ── */
function Charts({ hist }: { hist: BankSnapshot[] }) {
  if (hist.length < 2) return <EmptyNote>Charts appear after a couple of years of trading.</EmptyNote>;
  const years = hist.map((h) => h.year);
  // Per-year income (difference successive cumulative values).
  const yearly = (pick: (h: BankSnapshot) => number) =>
    hist.map((h, i) => (i === 0 ? 0 : Math.max(pick(h) - pick(hist[i - 1]), 0)));
  return (
    <div>
      <Chart title="Net worth (equity)" years={years}
        series={[{ label: "equity", color: "#7fcf9f", vals: hist.map((h) => h.equity) }]} />
      <Chart title="Assets" years={years} series={[
        { label: "reserves", color: "#e0c452", vals: hist.map((h) => h.reserves) },
        { label: "loans", color: "#5f97c0", vals: hist.map((h) => h.loans) },
        { label: "stakes", color: "#6fae8a", vals: hist.map((h) => h.stakes) },
      ]} />
      <Chart title="Liabilities" years={years} series={[
        { label: "deposits", color: "#c98f5f", vals: hist.map((h) => h.deposits) },
        { label: "notes", color: "#b8728f", vals: hist.map((h) => h.notes) },
      ]} />
      <Chart title="Income vs write-offs (per year)" years={years} series={[
        { label: "interest", color: "#7fb0e0", vals: yearly((h) => h.interest_cum) },
        { label: "dividends", color: "#6fae8a", vals: yearly((h) => h.dividends_cum) },
        { label: "write-offs", color: "#e06a5a", vals: yearly((h) => h.losses_cum) },
      ]} />
    </div>
  );
}

function Chart({ title, years, series }: { title: string; years: number[]; series: { label: string; color: string; vals: number[] }[] }) {
  const W = 300, H = 80, PADL = 4, PADB = 12;
  const max = Math.max(...series.flatMap((s) => s.vals), 1e-6);
  const n = years.length;
  const x = (i: number) => PADL + (i / Math.max(n - 1, 1)) * (W - PADL * 2);
  const y = (v: number) => H - PADB - (v / max) * (H - PADB - 4);
  return (
    <div style={{ marginBottom: 10 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
        <div style={{ color: T.inkMid, fontSize: FZ.tiny }}>{title}</div>
        <div style={{ display: "flex", gap: 7 }}>
          {series.map((s) => (
            <span key={s.label} style={{ color: s.color, fontSize: FZ.micro }}>● {s.label}</span>
          ))}
        </div>
      </div>
      <svg width="100%" viewBox={`0 0 ${W} ${H}`} style={{ display: "block", background: T.card, borderRadius: 4, marginTop: 2 }}>
        <text x={PADL} y={11} fill={T.inkDim} fontSize={7}>{fmt(max)}</text>
        {series.map((s) => (
          <polyline key={s.label} fill="none" stroke={s.color} strokeWidth={1.4}
            points={s.vals.map((v, i) => `${x(i)},${y(v)}`).join(" ")} />
        ))}
        <text x={PADL} y={H - 2} fill={T.inkDim} fontSize={7}>yr {years[0]}</text>
        <text x={W - PADL} y={H - 2} fill={T.inkDim} fontSize={7} textAnchor="end">yr {years[n - 1]}</text>
      </svg>
    </div>
  );
}

/* ── Deals: loans + stakes ── */
function Deals({ bank }: { bank: BankBrief }) {
  const purposeLabel: Record<string, string> = {
    trade: "Trade venture", guild_factory: "Guild factory", guild_civic: "Guild civic works",
    treasury: "City public works", colony: "Colony venture", estate: "Estate", structure: "Building",
  };
  const kindIcon: Record<string, string> = { house: "⚜️", guild: "🛠", polis: "🏛" };
  return (
    <div>
      <SectionLabel>Loans &amp; deals ({bank.loans.length})</SectionLabel>
      {bank.loans.length === 0 && <div style={{ color: T.inkFaint, fontSize: FZ.tiny }}>No loans on the books.</div>}
      {bank.loans.map((l, i) => (
        <div key={i} style={dealRow}>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: T.ink, fontSize: FZ.small }}>{kindIcon[l.borrower_kind] ?? "•"} {l.borrower}</span>
            <span style={{ color: "#e0c452", fontSize: FZ.small }}>{fmt(l.outstanding)}</span>
          </div>
          <div style={{ color: T.inkMid, fontSize: FZ.micro }}>
            {purposeLabel[l.purpose] ?? l.purpose} · {(l.rate * 100).toFixed(1)}%/mo ·
            {" "}{l.term_years.toFixed(0)}-yr term · from yr {l.start_year} · principal {fmt(l.principal)}
          </div>
        </div>
      ))}

      <SectionLabel>Equity stakes ({bank.stakes.length})</SectionLabel>
      {bank.stakes.length === 0 && <div style={{ color: T.inkFaint, fontSize: FZ.tiny }}>No equity stakes held.</div>}
      {bank.stakes.map((s, i) => (
        <div key={i} style={dealRow}>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: T.ink, fontSize: FZ.small }}>🏭 {s.works}</span>
            <span style={{ color: "#6fae8a", fontSize: FZ.small }}>{(s.share * 100).toFixed(0)}%</span>
          </div>
          <div style={{ color: T.inkMid, fontSize: FZ.micro }}>{s.good} · book value {fmt(s.basis)}</div>
        </div>
      ))}
    </div>
  );
}

/* ── Info: how a bank earns + grows ── */
function Info({ bank }: { bank: BankBrief }) {
  return (
    <div style={{ fontSize: FZ.small, color: T.inkMid, lineHeight: 1.5 }}>
      <SectionLabel>Where the money comes from</SectionLabel>
      <p style={p}><b style={{ color: "#7fb0e0" }}>Loan interest.</b> The bank lends to merchant
        houses (trade), guilds (factories &amp; civic works), and city treasuries (public works) at
        {" "}<b>{(0.012 * 100).toFixed(1)}%/mo</b>. Borrowers repay interest + principal; the interest
        is profit, principal repayment retires the notes that funded the loan.</p>
      <p style={p}><b style={{ color: "#6fae8a" }}>Equity stakes.</b> The bank buys a share of a
        manufactory and draws that share of its output income as a dividend — so it grows with real
        production, not just lending.</p>
      <p style={p}><b style={{ color: "#c98f5f" }}>Deposits &amp; notes.</b> Wealthy houses place
        capital on deposit; the bank issues notes (credit) up to <b>3×</b> its specie reserves. This
        leverage funds more lending — but a thin reserve ratio invites a run.</p>
      <SectionLabel>How to grow net worth</SectionLabel>
      <ul style={{ margin: "2px 0 0 14px", padding: 0 }}>
        <li>Lend more in calm years (interest compounds into reserves).</li>
        <li>Take stakes in high-tier manufactories for steady dividends.</li>
        <li>Open branches (offices) to reach new markets and depositors.</li>
        <li>Keep the reserve ratio above <b>22%</b> so a crash can't trigger a fatal run.</li>
        <li>Avoid over-lending to fragile borrowers — defaults are written off as losses.</li>
      </ul>
      <SectionLabel>This bank</SectionLabel>
      <p style={p}>
        Net worth <b style={{ color: T.goodInk }}>{fmt(bank.equity)}</b>. Lifetime interest {fmt(bank.interest_earned)},
        dividends {fmt(bank.dividends_earned)}, write-offs {fmt(bank.losses)}.
        {bank.reserve_ratio < 0.22 ? " ⚠ Reserve ratio is thin — vulnerable to a run." : " Reserves are sound."}
      </p>
    </div>
  );
}

/* ── Bits ── */
function Stat({ label, value, accent }: { label: string; value: string; accent?: string }) {
  return (
    <div style={{ background: T.raised, borderRadius: RADIUS.sm, padding: "4px 6px" }}>
      <div style={{ color: T.inkDim, fontSize: FZ.micro }}>{label}</div>
      <div style={{ color: accent ?? T.ink, fontSize: FZ.base, fontWeight: 700 }}>{value}</div>
    </div>
  );
}
function SectionLabel({ children }: { children: React.ReactNode }) {
  return <div style={{ color: T.inkMid, fontSize: FZ.tiny, textTransform: "uppercase", letterSpacing: 0.5, margin: "9px 0 4px", fontWeight: 600 }}>{children}</div>;
}

function fmt(n: number): string {
  const a = Math.abs(n);
  if (a >= 1000) return `${(n / 1000).toFixed(a >= 10000 ? 0 : 1)}k`;
  return n.toFixed(a >= 100 || a === 0 ? 0 : 1);
}

const list: React.CSSProperties = {
  width: 130, flex: "0 0 auto", borderRight: `1px solid ${T.line}`,
  overflowY: "auto", padding: "4px 2px",
};
const listRow: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 5, padding: "4px 5px",
  cursor: "pointer", borderRadius: RADIUS.sm,
};
const scroll: React.CSSProperties = { overflowY: "auto", padding: "6px 9px 12px" };
const statGrid: React.CSSProperties = {
  display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 5, marginBottom: 4,
};
const dealRow: React.CSSProperties = { padding: "4px 0", borderBottom: `1px solid ${T.lineSoft}` };
const p: React.CSSProperties = { margin: "0 0 6px" };
