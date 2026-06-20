import { useEffect, useState } from "react";
import { useCampaignStore } from "../state/campaignStore";
import { useUIStore } from "../state/uiStore";
import { campaignGetBanks } from "../bridge/tauri";
import type { BankBrief, BankSnapshot } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** Dedicated Bank panel. Pick a bank from the list (or arrive here by clicking its
 *  icon on the map) and inspect:
 *   • Overview — a balance-sheet schematic (assets vs liabilities + equity), the key
 *     ratios, and the coin it banks in.
 *   • Charts — yearly line graphs: net worth (equity), reserves/loans/stakes,
 *     deposits/notes, and income vs write-offs — so the REASONS behind a rise or
 *     fall are visible.
 *   • Deals — every live loan with its agreement terms, plus equity stakes held.
 *   • How it works — where a bank's money comes from and how to grow its net worth.
 *  All read straight from the live campaign sim. */
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

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.bank);
  if (!open) return null;
  const close = () => useUIStore.getState().setShowBank(false);
  const pick = (i: number) => useUIStore.getState().setSelectedBankIdx(i);

  // The focused bank: the explicit selection if valid, else the soundest (first).
  const idx = selIdx != null && selIdx < banks.length ? selIdx : 0;
  const bank = banks[idx];

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>🏦 Banks</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>

      {!active && <div style={empty}>Banks appear once a campaign is running.</div>}
      {active && banks.length === 0 && (
        <div style={empty}>No banks chartered yet — the age of banking opens around year 20,
          once a wealthy house holds a trusted coin.</div>
      )}

      {active && banks.length > 0 && (
        <div style={{ display: "flex", minHeight: 0, flex: 1 }}>
          {/* ── Bank list ── */}
          <div style={list}>
            {banks.map((b, i) => (
              <div key={i} onClick={() => pick(i)}
                style={{ ...listRow, background: i === idx ? "#15243a" : "transparent",
                  opacity: b.defunct ? 0.5 : 1 }}>
                <span style={{ width: 8, height: 8, borderRadius: 8, background: b.color, flex: "0 0 auto" }} />
                <div style={{ minWidth: 0, flex: 1 }}>
                  <div style={{ color: i === idx ? "#cfe2f6" : "#aebfd4", fontSize: 10.5,
                    overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {b.name}{b.defunct ? " ✝" : ""}
                  </div>
                  <div style={{ color: "#5f7a96", fontSize: 8.5 }}>
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
                  <div style={{ color: "#dfeafc", fontSize: 12.5, fontWeight: 700 }}>
                    {bank.name}
                  </div>
                  <div style={{ color: "#7f96b2", fontSize: 9.5 }}>
                    {bank.owner} · seat {bank.seat} · founded yr {bank.founded_year}
                    {bank.coin_name ? ` · banks in ${bank.coin_name}` : ""}
                  </div>
                </div>
                <div style={tabBar}>
                  {([["overview", "Overview"], ["charts", "Charts"], ["deals", `Deals (${bank.loans.length + bank.stakes.length})`], ["info", "How it works"]] as const).map(([id, lbl]) => (
                    <div key={id} onClick={() => setTab(id)}
                      style={{ ...tabS, fontWeight: tab === id ? 700 : 400,
                        color: tab === id ? "#cfe2f6" : "#6a86a6",
                        borderBottom: tab === id ? "2px solid #5f97c0" : "2px solid transparent" }}>
                      {lbl}
                    </div>
                  ))}
                </div>
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
    </div>
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
        <Stat label="Net worth" value={fmt(bank.equity)} accent="#7fcf9f" />
        <Stat label="Reserve ratio" value={rrPct} accent={fragile ? "#e06a5a" : "#7fb0e0"} />
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
            <div key={i} style={{ color: "#9fb4cc", fontSize: 9.5, padding: "1px 0" }}>• {e}</div>
          ))}
        </>
      )}
    </div>
  );
}

function BalanceColumn({ title, rows, total }: { title: string; rows: { label: string; v: number; c: string }[]; total: number }) {
  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ color: "#8fa6c2", fontSize: 9, marginBottom: 3 }}>{title}</div>
      {rows.length === 0 && <div style={{ color: "#445268", fontSize: 9 }}>—</div>}
      {rows.map((r, i) => (
        <div key={i} style={{ marginBottom: 4 }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: "#aebfd4" }}>
            <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.label}</span>
            <span style={{ color: "#cfe2f6" }}>{fmt(r.v)}</span>
          </div>
          <div style={{ height: 6, background: "#101a26", borderRadius: 3, overflow: "hidden" }}>
            <div style={{ width: `${(r.v / total) * 100}%`, height: "100%", background: r.c }} />
          </div>
        </div>
      ))}
    </div>
  );
}

/* ── Charts: yearly line graphs ── */
function Charts({ hist }: { hist: BankSnapshot[] }) {
  if (hist.length < 2) return <div style={empty}>Charts appear after a couple of years of trading.</div>;
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
        <div style={{ color: "#8fa6c2", fontSize: 9.5 }}>{title}</div>
        <div style={{ display: "flex", gap: 7 }}>
          {series.map((s) => (
            <span key={s.label} style={{ color: s.color, fontSize: 8.5 }}>● {s.label}</span>
          ))}
        </div>
      </div>
      <svg width="100%" viewBox={`0 0 ${W} ${H}`} style={{ display: "block", background: "#0b121c", borderRadius: 4, marginTop: 2 }}>
        <text x={PADL} y={11} fill="#5a7290" fontSize={7}>{fmt(max)}</text>
        {series.map((s) => (
          <polyline key={s.label} fill="none" stroke={s.color} strokeWidth={1.4}
            points={s.vals.map((v, i) => `${x(i)},${y(v)}`).join(" ")} />
        ))}
        <text x={PADL} y={H - 2} fill="#5a7290" fontSize={7}>yr {years[0]}</text>
        <text x={W - PADL} y={H - 2} fill="#5a7290" fontSize={7} textAnchor="end">yr {years[n - 1]}</text>
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
      {bank.loans.length === 0 && <div style={{ color: "#445268", fontSize: 9.5 }}>No loans on the books.</div>}
      {bank.loans.map((l, i) => (
        <div key={i} style={dealRow}>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "#cfe2f6", fontSize: 10 }}>{kindIcon[l.borrower_kind] ?? "•"} {l.borrower}</span>
            <span style={{ color: "#e0c452", fontSize: 10 }}>{fmt(l.outstanding)}</span>
          </div>
          <div style={{ color: "#7f96b2", fontSize: 8.5 }}>
            {purposeLabel[l.purpose] ?? l.purpose} · {(l.rate * 100).toFixed(1)}%/mo ·
            {" "}{l.term_years.toFixed(0)}-yr term · from yr {l.start_year} · principal {fmt(l.principal)}
          </div>
        </div>
      ))}

      <SectionLabel>Equity stakes ({bank.stakes.length})</SectionLabel>
      {bank.stakes.length === 0 && <div style={{ color: "#445268", fontSize: 9.5 }}>No equity stakes held.</div>}
      {bank.stakes.map((s, i) => (
        <div key={i} style={dealRow}>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "#cfe2f6", fontSize: 10 }}>🏭 {s.works}</span>
            <span style={{ color: "#6fae8a", fontSize: 10 }}>{(s.share * 100).toFixed(0)}%</span>
          </div>
          <div style={{ color: "#7f96b2", fontSize: 8.5 }}>{s.good} · book value {fmt(s.basis)}</div>
        </div>
      ))}
    </div>
  );
}

/* ── Info: how a bank earns + grows ── */
function Info({ bank }: { bank: BankBrief }) {
  return (
    <div style={{ fontSize: 10, color: "#aebfd4", lineHeight: 1.5 }}>
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
        Net worth <b style={{ color: "#7fcf9f" }}>{fmt(bank.equity)}</b>. Lifetime interest {fmt(bank.interest_earned)},
        dividends {fmt(bank.dividends_earned)}, write-offs {fmt(bank.losses)}.
        {bank.reserve_ratio < 0.22 ? " ⚠ Reserve ratio is thin — vulnerable to a run." : " Reserves are sound."}
      </p>
    </div>
  );
}

/* ── Bits ── */
function Stat({ label, value, accent }: { label: string; value: string; accent?: string }) {
  return (
    <div style={{ background: "#101a26", borderRadius: 4, padding: "4px 6px" }}>
      <div style={{ color: "#6a86a6", fontSize: 8 }}>{label}</div>
      <div style={{ color: accent ?? "#cfe2f6", fontSize: 12, fontWeight: 700 }}>{value}</div>
    </div>
  );
}
function SectionLabel({ children }: { children: React.ReactNode }) {
  return <div style={{ color: "#7fa0c4", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.5, margin: "9px 0 4px", fontWeight: 600 }}>{children}</div>;
}

function fmt(n: number): string {
  const a = Math.abs(n);
  if (a >= 1000) return `${(n / 1000).toFixed(a >= 10000 ? 0 : 1)}k`;
  return n.toFixed(a >= 100 || a === 0 ? 0 : 1);
}

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 430, maxHeight: "82vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 40,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#cfe0f4", fontWeight: 700, fontSize: 12,
};
const list: React.CSSProperties = {
  width: 130, flex: "0 0 auto", borderRight: "1px solid #16243a",
  overflowY: "auto", padding: "4px 2px",
};
const listRow: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 5, padding: "4px 5px",
  cursor: "pointer", borderRadius: 4,
};
const tabBar: React.CSSProperties = {
  display: "flex", gap: 2, padding: "2px 8px 0", borderBottom: "1px solid #1e2e42",
};
const tabS: React.CSSProperties = { padding: "4px 7px", cursor: "pointer", fontSize: 10 };
const scroll: React.CSSProperties = { overflowY: "auto", padding: "6px 9px 12px" };
const statGrid: React.CSSProperties = {
  display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 5, marginBottom: 4,
};
const dealRow: React.CSSProperties = { padding: "4px 0", borderBottom: "1px solid #131e2a" };
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "14px 12px", lineHeight: 1.5 };
const p: React.CSSProperties = { margin: "0 0 6px" };
