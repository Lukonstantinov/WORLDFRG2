import { useEffect, useState } from "react";
import { useCampaignStore } from "../state/campaignStore";
import { useUIStore } from "../state/uiStore";
import {
  campaignGetCurrencies, campaignGetBanks, campaignGetCrashes, campaignGetSchematics, campaignGetWars,
} from "../bridge/tauri";
import type { CurrencyBrief, BankBrief, CrashRecord, CitySchematic, WarsPayload, HouseBrief } from "../types";
import { CoinIcon } from "./CoinIcon";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

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
  const houses = useCampaignStore((s) => s.houses);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;
  const [tab, setTab] = useState<"coins" | "banks" | "wars" | "crashes" | "schem">("coins");
  const [coins, setCoins] = useState<CurrencyBrief[]>([]);
  const [banks, setBanks] = useState<BankBrief[]>([]);
  const [crashes, setCrashes] = useState<CrashRecord[]>([]);
  const [schem, setSchem] = useState<CitySchematic[]>([]);
  const [wars, setWars] = useState<WarsPayload>({ active: [], log: [] });

  useEffect(() => {
    if (!open || !active) return;
    campaignGetCurrencies().then(setCoins).catch(() => setCoins([]));
    campaignGetBanks().then(setBanks).catch(() => setBanks([]));
    campaignGetCrashes().then(setCrashes).catch(() => setCrashes([]));
    campaignGetSchematics().then(setSchem).catch(() => setSchem([]));
    campaignGetWars().then(setWars).catch(() => setWars({ active: [], log: [] }));
  }, [open, active, tick]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.coin);
  if (!open) return null;
  const close = () => useUIStore.getState().setShowCoinCredit(false);

  const tabs = [
    ["coins", "🪙 Currencies"],
    ["banks", "🏦 Banks"],
    ["wars", `⚔ Wars${wars.active.length ? ` (${wars.active.length})` : ""}`],
    ["crashes", "📉 Crashes"],
    ["schem", "🏛 Schematics"],
  ] as const;

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>🪙 Coin, Credit &amp; Crashes</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
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
            <div style={hint}>
              Ranked by reserve strength (trust × trade). Reserve coins are accepted abroad and shave freight.
              <div style={{ marginTop: 4, display: "flex", flexWrap: "wrap", gap: "2px 10px", color: "#7a90a8" }}>
                <span>▰ trust = acceptance</span>
                <span>value = exchange agio (×grain)</span>
                <span>🪙 = mint fineness</span>
                <span>⇄ = trade throughput</span>
                <span style={{ color: "#37a05a" }}>RESERVE = accepted abroad</span>
              </div>
              <div style={{ marginTop: 2, color: "#5a7290" }}>Click a coin for the full explanation.</div>
            </div>
          )}
          {coins.map((c, i) => <CurrencyCard key={c.hub} c={c} rank={i + 1} />)}
        </div>
      )}

      {active && tab === "banks" && (
        <div style={scroll}>
          {banks.length === 0 && <div style={empty}>No banks chartered yet — a wealthy banking house in a trusted-coin city founds the first.</div>}
          {banks.map((b, i) => <BankCard key={`${b.name}-${i}`} b={b} house={houses.find((h) => h.idx === b.owner_idx)} />)}
        </div>
      )}

      {active && tab === "wars" && (
        <div style={scroll}>
          {wars.active.length === 0 && wars.log.length === 0 &&
            <div style={empty}>No wars — the poleis are at peace. (Rival councils spark economic wars: forced levies, blockades, reparations.)</div>}
          {wars.active.length > 0 && <div style={hint}>Active wars — levies bleed resident houses into the war chest.</div>}
          {wars.active.map((w, i) => (
            <div key={`a${i}`} style={card}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                <span style={{ color: "#e88", fontWeight: 700, fontSize: 12 }}>⚔ {w.a} vs {w.b}</span>
                <span style={{ flex: 1 }} />
                <span style={{ color: "#8aa0b8", fontSize: 9 }}>{w.years}y · {w.cause}</span>
              </div>
              <div style={{ display: "flex", gap: 10, fontSize: 9, color: "#8aa8c8", marginTop: 2 }}>
                <span title="War chest spent by each side">chest {fmtk(w.chest_a)} / {fmtk(w.chest_b)}</span>
                <span style={{ color: "#e0b080" }} title="Total levied from resident houses">levied {fmtk(w.levies)}</span>
              </div>
            </div>
          ))}
          {wars.log.length > 0 && (
            <div style={{ color: "#5a6a7e", fontSize: 9, margin: "8px 0 2px", textTransform: "uppercase" }}>Concluded</div>
          )}
          {wars.log.map((w, i) => (
            <div key={`l${i}`} style={card}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                <span style={{ color: "#cfe0f4", fontWeight: 700, fontSize: 11 }}>{w.winner}</span>
                <span style={{ color: "#7a90a8", fontSize: 9 }}>beat {w.loser} · yr {w.end_year}</span>
              </div>
              <div style={{ color: "#c0d0e0", fontSize: 9.5, marginTop: 1 }}>{w.text}</div>
              <div style={{ display: "flex", gap: 10, fontSize: 9, color: "#8aa8c8", marginTop: 1 }}>
                <span style={{ color: "#c9a227" }}>reparations {fmtk(w.reparations)}</span>
                <span>levied {fmtk(w.levies_total)}</span>
              </div>
            </div>
          ))}
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
  const [open, setOpen] = useState(false);
  const debased = c.fineness < 0.999;
  const strength = c.trust * c.throughput;
  return (
    <div style={{ ...card, cursor: "pointer" }} onClick={() => setOpen((v) => !v)}>
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ color: "#6a86a6", fontSize: 9, width: 16, flex: "0 0 auto" }}>#{rank}</span>
        <CoinIcon issuer={c.issuer || c.city} value={c.value} size={22}
          title={`${c.coin_name} · value ${c.value.toFixed(2)}× · trust ${(c.trust * 100).toFixed(0)}%`} />
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12 }}>{c.coin_name}</span>
        <span style={{ flex: 1 }} />
        {c.is_reserve && <span style={{ color: "#37a05a", fontSize: 9, fontWeight: 700 }}>RESERVE</span>}
        <span style={{ color: "#5a7290", fontSize: 10, marginLeft: 4 }}>{open ? "▾" : "▸"}</span>
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 3, flexWrap: "wrap" }}>
        <TrustBar trust={c.trust} reserve={c.is_reserve} />
        <span style={{ color: c.value >= 1.05 ? "#e0c060" : "#9ab0c8", fontSize: 9, fontWeight: 700 }}
          title="Coin value index — agio above 1.0 = a premium 'hard' currency">
          value {c.value.toFixed(2)}×
        </span>
        <span title="Mint fineness" style={{ color: debased ? "#e0a020" : "#8aa8c8", fontSize: 9 }}>
          🪙 {(c.fineness * 100).toFixed(0)}%{debased ? " debased" : ""}
        </span>
        <span style={{ color: "#7fa0c0", fontSize: 9 }} title="Trade throughput at the issuing city">⇄ {fmtk(c.throughput)}</span>
      </div>
      {open && (
        <div style={{ marginTop: 6, paddingTop: 6, borderTop: "1px solid #1b2a3c", fontSize: 10, lineHeight: 1.5 }}>
          <div style={{ color: "#8aa8c8", marginBottom: 3 }}>
            Minted by <b style={{ color: "#cfe2f6" }}>{c.issuer || c.city}</b>{c.city && c.issuer ? ` · ${c.city}` : ""}.
          </div>
          <Explain label="Trust (acceptance)" value={`${(c.trust * 100).toFixed(0)}%`}
            text="How widely merchants accept the coin. Sticky — it eases toward a target each year and is hit hard by debasement. ≥55% makes it a reserve currency." />
          <Explain label="Value (agio)" value={`${c.value.toFixed(2)}×`}
            text="Exchange value against the grain-equivalent numeraire. Above 1.0 is a 'hard' premium currency; below 1.0 is weak/debased money." />
          <Explain label="Fineness" value={`${(c.fineness * 100).toFixed(0)}%`}
            text={debased ? "Precious-metal content. Below 100% = DEBASED — the mint skimmed seigniorage into the treasury, which erodes trust and can feed bubbles." : "Precious-metal content. 100% = full-bodied, honest coin."} />
          <Explain label="Throughput" value={fmtk(c.throughput)}
            text="Trade volume moving through the issuing city — the economic weight standing behind the coin." />
          <Explain label="Reserve strength" value={fmtk(strength)}
            text="trust × throughput — the ranking score. The strongest coins become international reserves, accepted abroad and granting their merchants a small import-freight discount." />
          <div style={{ color: c.is_reserve ? "#37a05a" : "#6a86a6", marginTop: 2 }}>
            {c.is_reserve ? "★ A RESERVE currency — held and accepted across borders." : "Not yet a reserve currency (needs ≥55% trust)."}
          </div>
        </div>
      )}
    </div>
  );
}

function Explain({ label, value, text }: { label: string; value: string; text: string }) {
  return (
    <div style={{ marginBottom: 3 }}>
      <span style={{ color: "#cbb88a", fontWeight: 700 }}>{label}: </span>
      <span style={{ color: "#cfe2f6" }}>{value}</span>
      <div style={{ color: "#7e93ab" }}>{text}</div>
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

// Founding gates from sim/tick.rs `update_banks` (BANK_FOUND_* constants). A
// chartered bank, by definition, met all of these the year it opened.
const BANK_GATES: { label: string; need: string; get: (h: HouseBrief) => string }[] = [
  { label: "Wealth", need: "≥ 100k", get: (h) => h.wealth.toFixed(0) },
  { label: "Prestige", need: "≥ 0.15", get: (h) => h.prestige.toFixed(2) },
  { label: "Seat coin-trust", need: "≥ 0.40", get: (h) => (h.coin_trust ?? 0).toFixed(2) },
  { label: "Age of banking", need: "year ≥ 20", get: () => "✓" },
];

function BankCard({ b, house }: { b: BankBrief; house?: HouseBrief }) {
  const assets = b.reserves + b.loans_out + b.real_estate;
  const liab = b.deposits + b.notes_issued;
  const fragile = b.reserve_ratio < 0.22;
  const [showWhy, setShowWhy] = useState(false);
  const [showInfo, setShowInfo] = useState(false);
  // Denominate the (grain-equivalent) sheet in the bank's seat coin, if it has one.
  const inCoin = b.coin_name && b.coin_value > 0
    ? (v: number) => `${fmtk(v / b.coin_value)} ${b.coin_name}` : null;
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
        {b.coin_name && <span title="The coin this bank banks in (its seat city's coin)" style={{ color: "#d8c878" }}> · banks in {b.coin_name}</span>}
        <span onClick={() => setShowWhy((s) => !s)} title="Why this bank opened"
          style={{ color: "#7fb0d8", cursor: "pointer", marginLeft: 6 }}>
          {showWhy ? "▾ why it opened" : "▸ why it opened"}
        </span>
        <span onClick={() => setShowInfo((s) => !s)} title="What the balance sheet means"
          style={{ color: "#7fb0d8", cursor: "pointer", marginLeft: 6 }}>
          {showInfo ? "▾ explain" : "▸ explain"}
        </span>
      </div>
      {showInfo && (
        <div style={{ margin: "4px 0", padding: "5px 7px", background: "#0e1a27", border: "1px solid #1e2e42", borderRadius: 6, fontSize: 10, lineHeight: 1.5 }}>
          {inCoin && <div style={{ color: "#d8c878", marginBottom: 3 }}>Amounts below are grain-equivalent; in {b.coin_name} the reserves ≈ {inCoin(b.reserves)}.</div>}
          <Explain label="Specie reserves" value={inCoin ? inCoin(b.reserves) : fmtk(b.reserves)} text="Hard money (coin/bullion) in the vault — what actually backs the bank. The founding 40k starts here." />
          <Explain label="Loans out" value={fmtk(b.loans_out)} text="Credit lent to houses (trade) and to the city treasury (public works), earning interest — an asset until repaid or defaulted." />
          <Explain label="Real estate" value={fmtk(b.real_estate)} text="Counting-houses and foreclosed property booked as assets." />
          <Explain label="Deposits" value={fmtk(b.deposits)} text="Idle capital wealthy families park here for interest — a liability the bank owes back." />
          <Explain label="Notes issued" value={fmtk(b.notes_issued)} text="Paper credit the bank put into circulation (its IOUs) — a liability; this is bank-created money." />
          <Explain label="Equity" value={fmtk(b.equity)} text="Net worth = assets − liabilities. If it turns negative the bank fails (the owner first injects capital to save it)." />
          <Explain label="Reserve ratio" value={Number.isFinite(b.reserve_ratio) ? `${(b.reserve_ratio * 100).toFixed(0)}%` : "—"} text="Reserves ÷ liabilities. Below 22% the bank is fragile and vulnerable to a run/contagion." />
        </div>
      )}
      {showWhy && (
        <div style={{ margin: "4px 0", padding: "5px 7px", background: "#0e1a27", border: "1px solid #1e2e42", borderRadius: 6 }}>
          <div style={{ fontSize: 9, color: "#9ab0c8", marginBottom: 3 }}>
            Chartered in {b.founded_year} because {b.owner} met every founding condition:
          </div>
          {BANK_GATES.map((g) => {
            // A chartered bank met all gates at founding; show ✓ and the house's
            // current value as context (a guaranteed pass for the no-bank/one-per
            // gate too). Fall back to a plain ✓ when the house isn't loaded.
            return (
              <div key={g.label} style={{ display: "flex", gap: 7, alignItems: "baseline", fontSize: 10, padding: "1px 0" }}>
                <span style={{ color: "#5fbf6f", fontWeight: 700, width: 12 }}>✓</span>
                <span style={{ flex: 1, color: "#cbd8e6" }}>{g.label} <span style={{ color: "#6a86a6" }}>{g.need}</span></span>
                <span style={{ color: "#9ab0c8", fontFamily: "ui-monospace,monospace" }}>{house ? g.get(house) : "✓"}</span>
              </div>
            );
          })}
          <div style={{ display: "flex", gap: 7, alignItems: "baseline", fontSize: 10, padding: "1px 0" }}>
            <span style={{ color: "#5fbf6f", fontWeight: 700, width: 12 }}>✓</span>
            <span style={{ flex: 1, color: "#cbd8e6" }}>No prior bank <span style={{ color: "#6a86a6" }}>one per house</span></span>
            <span style={{ color: "#9ab0c8" }}>first</span>
          </div>
          <div style={{ display: "flex", gap: 7, alignItems: "baseline", fontSize: 10, padding: "1px 0" }}>
            <span style={{ color: "#5fbf6f", fontWeight: 700, width: 12 }}>✓</span>
            <span style={{ flex: 1, color: "#cbd8e6" }}>Founding price <span style={{ color: "#6a86a6" }}>50k → 40k reserves, 10k charter fee</span></span>
            <span style={{ color: "#9ab0c8" }}>{fmtk(b.reserves)}</span>
          </div>
        </div>
      )}
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
        {s.coin_name ? (
          <span style={{ display: "inline-flex", alignItems: "center", gap: 4, color: "#d8c878", fontSize: 9 }}>
            <CoinIcon issuer={s.council || s.name} size={16} /> {s.coin_name}
          </span>
        ) : null}
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
