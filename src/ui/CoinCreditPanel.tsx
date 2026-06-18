import { useEffect, useState } from "react";
import { useCampaignStore } from "../state/campaignStore";
import { useUIStore } from "../state/uiStore";
import {
  campaignGetCurrencies, campaignGetBanks, campaignGetCrashes, campaignGetSchematics,
} from "../bridge/tauri";
import type { CurrencyBrief, BankBrief, CrashRecord, CitySchematic } from "../types";

/** DLC 3.5 · Coin, Credit & Crashes. Four tabs:
 *   • Currencies — the world reserve-currency ranking (the "Venice ducat"): each
 *     polis's named coin, its acceptance/trust, fineness and reserve standing.
 *   • Banks — each chartered bank as a T-account balance sheet (assets vs
 *     liabilities), equity, reserve ratio, branches and chronicle.
 *   • Crashes — the log of regional financial crises (bank failures / popped
 *     bubbles) and the regions they swept.
 *   • Schematics — a per-city blueprint of standing buildings, estates and bank
 *     counting-houses (the "schematics view").
 *  All read straight from the live campaign sim. */
export function CoinCreditPanel() {
  const open = useUIStore((s) => s.showCoinCredit);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;
  const [tab, setTab] = useState<"coins" | "banks" | "crashes" | "schem">("coins");
  const [coins, setCoins] = useState<CurrencyBrief[]>([]);
  const [banks, setBanks] = useState<BankBrief[]>([]);
  const [crashes, setCrashes] = useState<CrashRecord[]>([]);
  const [schem, setSchem] = useState<CitySchematic[]>([]);

  useEffect(() => {
    if (!open || !active) return;
    campaignGetCurrencies().then(setCoins).catch(() => setCoins([]));
    campaignGetBanks().then(setBanks).catch(() => setBanks([]));
    campaignGetCrashes().then(setCrashes).catch(() => setCrashes([]));
    campaignGetSchematics().then(setSchem).catch(() => setSchem([]));
  }, [open, active, tick]);

  if (!open) return null;
  const close = () => useUIStore.getState().setShowCoinCredit(false);

  const tabs = [
    ["coins", "🪙 Currencies"],
    ["banks", "🏦 Banks"],
    ["crashes", "📉 Crashes"],
    ["schem", "🏛 Schematics"],
  ] as const;

  return (
    <div style={panel}>
      <div style={header}>
        <span>🪙 Coin, Credit &amp; Crashes</span>
        <span style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>
      <div style={{ display: "flex", gap: 2, padding: "0 8px", borderBottom: "1px solid #1e2e42" }}>
        {tabs.map(([id, lbl]) => (
          <div key={id} onClick={() => setTab(id)}
            style={{ padding: "4px 8px", cursor: "pointer", fontSize: 10.5, fontWeight: tab === id ? 700 : 400,
              color: tab === id ? "#cfe2f6" : "#6a86a6",
              borderBottom: tab === id ? "2px solid #c8a23a" : "2px solid transparent" }}>
            {lbl}
          </div>
        ))}
      </div>

      {!active && <div style={empty}>Begin the campaign (Step 11) and let it run — coinage and banks form over the years.</div>}

      {active && tab === "coins" && (
        <div style={scroll}>
          {coins.length === 0 && <div style={empty}>No coinage yet — a council seat mints its first coin at New Year.</div>}
          {coins.length > 0 && (
            <div style={hint}>Ranked by reserve strength (trust × trade). Reserve coins are accepted abroad and shave freight.</div>
          )}
          {coins.map((c, i) => <CurrencyCard key={c.hub} c={c} rank={i + 1} />)}
        </div>
      )}

      {active && tab === "banks" && (
        <div style={scroll}>
          {banks.length === 0 && <div style={empty}>No banks chartered yet — a wealthy banking house in a trusted-coin city founds the first.</div>}
          {banks.map((b, i) => <BankCard key={`${b.name}-${i}`} b={b} />)}
        </div>
      )}

      {active && tab === "crashes" && (
        <div style={scroll}>
          {crashes.length === 0 && <div style={empty}>No financial crashes — credit is holding. (A bank failure or a popped bubble triggers a regional crash.)</div>}
          {crashes.map((c, i) => <CrashCard key={i} c={c} />)}
        </div>
      )}

      {active && tab === "schem" && (
        <div style={scroll}>
          {schem.length === 0 && <div style={empty}>No cities yet.</div>}
          {schem.slice(0, 40).map((s) => <SchematicCard key={s.hub} s={s} />)}
        </div>
      )}
    </div>
  );
}

const fmtk = (v: number) => {
  const a = Math.abs(v);
  if (a >= 1000) return `${(v / 1000).toFixed(1)}k`;
  return v.toFixed(a < 10 ? 1 : 0);
};

function TrustBar({ trust, reserve }: { trust: number; reserve: boolean }) {
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 5 }}>
      <span style={{ width: 80, height: 6, background: "#1a2636", borderRadius: 3, overflow: "hidden", flex: "0 0 auto" }}>
        <span style={{ display: "block", height: "100%",
          width: `${Math.max(2, Math.min(100, trust * 100))}%`,
          background: reserve ? "#37a05a" : "#c8a23a" }} />
      </span>
      <span style={{ color: "#9ab0c8", fontSize: 9 }}>{(trust * 100).toFixed(0)}%</span>
    </span>
  );
}

function CurrencyCard({ c, rank }: { c: CurrencyBrief; rank: number }) {
  const debased = c.fineness < 0.999;
  return (
    <div style={card}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
        <span style={{ color: "#6a86a6", fontSize: 9, width: 16, flex: "0 0 auto" }}>#{rank}</span>
        <span style={{ width: 9, height: 9, borderRadius: 2, background: c.color, alignSelf: "center", flex: "0 0 auto" }} />
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12 }}>{c.coin_name}</span>
        <span style={{ flex: 1 }} />
        {c.is_reserve && <span style={{ color: "#37a05a", fontSize: 9, fontWeight: 700 }}>RESERVE</span>}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 3, flexWrap: "wrap" }}>
        <TrustBar trust={c.trust} reserve={c.is_reserve} />
        <span title="Mint fineness" style={{ color: debased ? "#e0a020" : "#8aa8c8", fontSize: 9 }}>
          🪙 {(c.fineness * 100).toFixed(0)}%{debased ? " debased" : ""}
        </span>
        <span style={{ color: "#7fa0c0", fontSize: 9 }} title="Trade throughput at the issuing city">⇄ {fmtk(c.throughput)}</span>
      </div>
    </div>
  );
}

function Side({ title, rows, total, totalColor }: {
  title: string; rows: [string, number][]; total: number; totalColor: string;
}) {
  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ color: "#8aa8c8", fontSize: 9, fontWeight: 700, borderBottom: "1px solid #1c2c40", paddingBottom: 2, marginBottom: 2 }}>{title}</div>
      {rows.map(([k, v]) => (
        <div key={k} style={{ display: "flex", justifyContent: "space-between", fontSize: 9.5, color: "#b8c8da" }}>
          <span>{k}</span><span>{fmtk(v)}</span>
        </div>
      ))}
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9.5, fontWeight: 700, color: totalColor, borderTop: "1px solid #1c2c40", marginTop: 2, paddingTop: 1 }}>
        <span>Σ</span><span>{fmtk(total)}</span>
      </div>
    </div>
  );
}

function BankCard({ b }: { b: BankBrief }) {
  const assets = b.reserves + b.loans_out + b.real_estate;
  const liab = b.deposits + b.notes_issued;
  const fragile = b.reserve_ratio < 0.22;
  return (
    <div style={{ ...card, opacity: b.defunct ? 0.55 : 1 }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
        <span style={{ width: 9, height: 9, borderRadius: 2, background: b.color, alignSelf: "center", flex: "0 0 auto" }} />
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12 }}>{b.name}</span>
        <span style={{ flex: 1 }} />
        {b.defunct
          ? <span style={{ color: "#e6303a", fontSize: 9, fontWeight: 700 }}>FAILED</span>
          : <span style={{ color: "#d8c878", fontSize: 10 }} title="Equity (net worth)">≋ {fmtk(b.equity)}</span>}
      </div>
      <div style={{ color: "#9ab0c8", fontSize: 9, marginTop: 1 }}>
        {b.seat} · {b.owner} · est. {b.founded_year}
      </div>
      {/* T-account balance sheet */}
      <div style={{ display: "flex", gap: 12, marginTop: 5 }}>
        <Side title="Assets" totalColor="#80c890" total={assets}
          rows={[["Specie reserves", b.reserves], ["Loans out", b.loans_out], ["Real estate", b.real_estate]]} />
        <Side title="Liabilities" totalColor="#e0a880" total={liab}
          rows={[["Deposits", b.deposits], ["Notes issued", b.notes_issued], ["Equity", b.equity]]} />
      </div>
      <div style={{ display: "flex", gap: 10, fontSize: 9, color: "#8aa8c8", marginTop: 4, flexWrap: "wrap" }}>
        <span style={{ color: fragile ? "#e6303a" : "#8aa8c8" }}
          title="Reserves ÷ liabilities — below 22% the bank is fragile (run risk)">
          reserve ratio {Number.isFinite(b.reserve_ratio) ? `${(b.reserve_ratio * 100).toFixed(0)}%` : "—"}{fragile ? " ⚠" : ""}
        </span>
        <span>{b.n_loans} loans</span>
        <span style={{ color: "#80c890" }} title="Cumulative interest earned">+{fmtk(b.interest_earned)}</span>
        {b.losses > 0.01 && <span style={{ color: "#e08080" }} title="Losses written off">−{fmtk(b.losses)}</span>}
      </div>
      {b.branches.length > 0 && (
        <div style={{ fontSize: 9, color: "#7fa0c0", marginTop: 2 }}>
          Counting-houses: {b.branches.join(", ")}
        </div>
      )}
      {b.events.length > 0 && (
        <div style={{ fontSize: 9, color: "#6a86a6", marginTop: 2, fontStyle: "italic" }}>
          {b.events[0]}
        </div>
      )}
    </div>
  );
}

function CrashCard({ c }: { c: CrashRecord }) {
  return (
    <div style={card}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
        <span style={{ color: "#e6303a", fontWeight: 700, fontSize: 12 }}>Year {c.year}</span>
        <span style={{ color: "#9ab0c8", fontSize: 10 }}>· {c.origin_name}</span>
        <span style={{ flex: 1 }} />
        <span style={{ color: "#8aa0b8", fontSize: 9, fontStyle: "italic" }}>{c.cause}</span>
      </div>
      <div style={{ color: "#c0d0e0", fontSize: 9.5, marginTop: 2 }}>{c.text}</div>
      <div style={{ display: "flex", gap: 10, fontSize: 9, color: "#8aa8c8", marginTop: 2 }}>
        <span>🏙 {c.cities_hit} cities</span>
        <span style={{ color: c.banks_failed > 0 ? "#e08080" : "#8aa8c8" }}>🏦 {c.banks_failed} banks failed</span>
      </div>
    </div>
  );
}

function Chip({ text, sub, color }: { text: string; sub?: string; color?: string }) {
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 4, padding: "2px 7px", margin: 2,
      background: "#13202e", border: `1px solid ${color ?? "#22364c"}`, borderRadius: 5, fontSize: 9.5, color: "#cbd8e6" }}>
      {text}{sub ? <span style={{ color: "#7a90a8" }}>· {sub}</span> : null}
    </span>
  );
}

function SchematicCard({ s }: { s: CitySchematic }) {
  return (
    <div style={card}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12 }}>{s.name}</span>
        <span style={{ color: "#6a86a6", fontSize: 9 }}>· {s.population.toLocaleString()}</span>
        <span style={{ flex: 1 }} />
        {s.coin_name ? <span style={{ color: "#d8c878", fontSize: 9 }}>🪙 {s.coin_name}</span> : null}
      </div>
      {s.council ? <div style={{ color: "#9ab0c8", fontSize: 9, marginTop: 1 }}>Council: {s.council}</div> : null}

      <div style={{ marginTop: 4 }}>
        <div style={blueprintLabel}>Buildings</div>
        {s.buildings.length === 0 ? <span style={none}>none</span> :
          <div style={{ display: "flex", flexWrap: "wrap" }}>
            {s.buildings.map((b, i) => <Chip key={i} text={b.label} sub={b.effect} />)}
          </div>}
      </div>

      {(s.banks_seated.length > 0 || s.bank_branches.length > 0) && (
        <div style={{ marginTop: 4 }}>
          <div style={blueprintLabel}>Banks</div>
          <div style={{ display: "flex", flexWrap: "wrap" }}>
            {s.banks_seated.map((b, i) => <Chip key={`s${i}`} text={`🏦 ${b}`} sub="seat" color="#3a6a44" />)}
            {s.bank_branches.map((b, i) => <Chip key={`br${i}`} text={`🏦 ${b}`} sub="branch" color="#2a4a64" />)}
          </div>
        </div>
      )}

      <div style={{ marginTop: 4 }}>
        <div style={blueprintLabel}>Estates ({s.estates.length})</div>
        {s.estates.length === 0 ? <span style={none}>none</span> :
          <div style={{ display: "flex", flexWrap: "wrap" }}>
            {s.estates.slice(0, 14).map((e, i) =>
              <Chip key={i} text={`${e.label} ${"★".repeat(Math.min(5, e.tier))}`} sub={e.good || e.owner} />)}
          </div>}
      </div>
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 360, maxHeight: "80vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 40,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#cfe0f4", fontWeight: 700, fontSize: 12,
};
const scroll: React.CSSProperties = { overflowY: "auto", padding: "6px 8px 10px" };
const card: React.CSSProperties = {
  display: "flex", flexDirection: "column", padding: "6px 4px",
  borderBottom: "1px solid #131e2a",
};
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "12px 10px", lineHeight: 1.5 };
const hint: React.CSSProperties = { color: "#6a86a6", fontSize: 9, marginBottom: 6 };
const blueprintLabel: React.CSSProperties = { color: "#7a90a8", fontSize: 8.5, textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 1 };
const none: React.CSSProperties = { color: "#445268", fontSize: 9, fontStyle: "italic" };
