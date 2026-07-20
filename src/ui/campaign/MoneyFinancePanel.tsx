import { useEffect, useState } from "react";
import { useCampaignStore } from "@state/campaignStore";
import { useUIStore } from "@state/uiStore";
import {
  campaignGetMints, campaignGetBanks, campaignGetCrashes, campaignGetSchematics,
  campaignGetWars, campaignCoinUsage, campaignGetSpeculation, campaignMonetaryChronicle,
  campaignReserves,
} from "@bridge";
import type {
  MintBrief, CoinUseCity, BankBrief, CrashRecord, CitySchematic, WarsPayload,
  HouseBrief, SpecCenter, MonetaryEvent, ReservesPayload, ReserveHolder,
} from "@types";
import { CoinIcon, type CoinMetal } from "@ui/heraldry/CoinIcon";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";

/** v2.0 · 💰 Money & Finance — the ONE window for the whole monetary system,
 *  merging the old Speculation·Poleis and Coin·Credit·Crashes panels. Five tabs:
 *   • Coin & Mints — each polis-issuer as a single unified card: one headline coin
 *     STRENGTH (fineness × acceptance) with the two drivers as sub-bars, the civic
 *     treasury/tariffs behind it, reach, the local price level, and reform state.
 *   • Banks — chartered banks as T-account balance sheets.
 *   • Bubbles — the yearly speculation why-chain per polis.
 *   • Shocks — wars, crashes and the monetary chronicle interleaved by year.
 *   • Schematics — the per-city blueprint of buildings, banks and estates. */
export function MoneyFinancePanel() {
  const open = useUIStore((s) => s.showMoneyFinance);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const houses = useCampaignStore((s) => s.houses);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;
  const [tab, setTab] = useState<"mints" | "reserves" | "banks" | "bubbles" | "shocks" | "schem">("mints");
  const [mints, setMints] = useState<MintBrief[]>([]);
  const [banks, setBanks] = useState<BankBrief[]>([]);
  const [crashes, setCrashes] = useState<CrashRecord[]>([]);
  const [schem, setSchem] = useState<CitySchematic[]>([]);
  const [wars, setWars] = useState<WarsPayload>({ active: [], log: [] });
  const [coinUse, setCoinUse] = useState<CoinUseCity[]>([]);
  const [bubbles, setBubbles] = useState<SpecCenter[]>([]);
  const [chronicle, setChronicle] = useState<MonetaryEvent[]>([]);
  const [reserves, setReserves] = useState<ReservesPayload>({ cities: [], banks: [], houses: [] });

  useEffect(() => {
    if (!open || !active) return;
    campaignGetMints().then(setMints).catch(() => setMints([]));
    campaignGetBanks().then(setBanks).catch(() => setBanks([]));
    campaignGetCrashes().then(setCrashes).catch(() => setCrashes([]));
    campaignGetSchematics().then(setSchem).catch(() => setSchem([]));
    campaignGetWars().then(setWars).catch(() => setWars({ active: [], log: [] }));
    campaignCoinUsage().then(setCoinUse).catch(() => setCoinUse([]));
    campaignGetSpeculation().then(setBubbles).catch(() => setBubbles([]));
    campaignMonetaryChronicle().then(setChronicle).catch(() => setChronicle([]));
    campaignReserves().then(setReserves).catch(() => setReserves({ cities: [], banks: [], houses: [] }));
  }, [open, active, tick]);
  const coinOverlay = useUIStore((s) => s.coinOverlayHub);
  const setCoinOverlay = useUIStore((s) => s.setCoinOverlayHub);
  const showRiskOverlay = () => useUIStore.getState().setOverlayVisible("speculation", true);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.coin);
  if (!open) return null;
  const close = () => useUIStore.getState().setShowMoneyFinance(false);

  const coined = mints.filter((m) => m.coin_name);
  const topCoin = coined[0];
  const tabs = [
    ["mints", "🪙 Coin & Mints"],
    ["reserves", "💰 Reserves"],
    ["banks", "🏦 Banks"],
    ["bubbles", "🫧 Bubbles"],
    ["shocks", `⚔ Shocks${wars.active.length ? ` (${wars.active.length})` : ""}`],
    ["schem", "🏛 Schematics"],
  ] as const;

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>💰 Money &amp; Finance</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>
      <div style={{ display: "flex", gap: 2, padding: "0 8px", borderBottom: "1px solid #1e2e42", flexWrap: "wrap" }}>
        {tabs.map(([id, lbl]) => (
          <div key={id} onClick={() => setTab(id)}
            style={{ padding: "4px 8px", cursor: "pointer", fontSize: 10.5, fontWeight: tab === id ? 700 : 400,
              color: tab === id ? "#cfe2f6" : "#6a86a6",
              borderBottom: tab === id ? "2px solid #c8a23a" : "2px solid transparent" }}>
            {lbl}
          </div>
        ))}
      </div>

      {!active && <div style={empty}>Begin the campaign (Step 11) and let it run — coinage, banks and the monetary story form over the years.</div>}

      {active && tab === "mints" && (
        <div style={scroll}>
          {mints.length === 0 && <div style={empty}>No poleis yet — council seats mint their first coin at New Year.</div>}
          {mints.length > 0 && (
            <div style={hint}>
              Every council seat as a unified mint, ranked by coin strength. One headline
              <b style={{ color: "#cbb88a" }}> strength</b> = fineness × acceptance; the civic treasury and
              tariffs behind it, its reach, and the local price level all sit on the same card.
              <div style={{ marginTop: 3, color: "#5a7290" }}>Click a mint for the full explanation + who uses the coin.</div>
            </div>
          )}
          {mints.map((m, i) => (
            <MintCard key={m.hub} m={m} rank={i + 1} topCoin={topCoin}
              usage={coinUse.filter((u) => u.coin === m.hub)}
              onMap={coinOverlay === m.hub}
              toggleMap={() => setCoinOverlay(coinOverlay === m.hub ? null : m.hub)} />
          ))}
        </div>
      )}

      {active && tab === "reserves" && <ReservesTab reserves={reserves} />}

      {active && tab === "banks" && (
        <div style={scroll}>
          {banks.length === 0 && <div style={empty}>No banks chartered yet — a wealthy banking house in a trusted-coin city founds the first (from year 20).</div>}
          {banks.map((b, i) => <BankCard key={`${b.name}-${i}`} b={b} house={houses.find((h) => h.idx === b.owner_idx)} />)}
        </div>
      )}

      {active && tab === "bubbles" && (
        <div style={scroll}>
          {bubbles.length === 0 && <div style={empty}>No reading yet — advance the campaign past its first New Year.</div>}
          {bubbles.length > 0 && (
            <div style={{ ...hint, display: "flex", justifyContent: "space-between" }}>
              <span>Year {bubbles[0]?.year} · top {bubbles.length} at-risk poleis (speculation why-chain)</span>
              <span style={{ cursor: "pointer", color: "#7fb0e0" }} onClick={showRiskOverlay}>show on map</span>
            </div>
          )}
          {bubbles.map((c) => <SpecCard key={c.hub} c={c} />)}
        </div>
      )}

      {active && tab === "shocks" && (
        <ShocksTab wars={wars} crashes={crashes} chronicle={chronicle} />
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

/** Tint for the metal label (matches the struck-coin palettes). */
const METAL_COLOR: Record<string, string> = {
  gold: "#e8c452", silver: "#c4cdd8", electrum: "#dcd083", bronze: "#c07f45",
};
/** Tint for the bullion-supply (coin-supply limit) label. */
const BULLION_COLOR: Record<string, string> = {
  ample: "#5fbf8a", tight: "#e0b060", scarce: "#e08080",
};

/** A labelled driver sub-bar (fineness / trust). */
function DriverBar({ label, frac, color, text }: { label: string; frac: number; color: string; text: string }) {
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
      <span style={{ color: "#7e93ab", fontSize: 8.5 }}>{label}</span>
      <span style={{ width: 42, height: 5, background: "#1a2636", borderRadius: 3, overflow: "hidden", flex: "0 0 auto" }}>
        <span style={{ display: "block", height: "100%", width: `${Math.max(3, Math.min(100, frac * 100))}%`, background: color }} />
      </span>
      <span style={{ color: "#9ab0c8", fontSize: 8.5 }}>{text}</span>
    </span>
  );
}

/** v2.0 · the unified MINT card — polis + coin in one, headline strength + 2 drivers. */
function MintCard({ m, rank, topCoin, usage, onMap, toggleMap }:
  { m: MintBrief; rank: number; topCoin?: MintBrief; usage: CoinUseCity[]; onMap: boolean; toggleMap: () => void }) {
  const [open, setOpen] = useState(false);
  const hasCoin = !!m.coin_name;
  const debased = m.fineness < 0.999;
  const xrate = hasCoin && topCoin && topCoin.hub !== m.hub && topCoin.value > 0
    ? m.value / topCoin.value : null;
  // Strength → color: red (weak) → gold → green (reserve-grade).
  const sc = m.strength;
  const strengthColor = m.is_reserve ? "#37a05a" : sc >= 40 ? "#c8a23a" : "#d08a3a";
  const cpi = m.price_level;
  return (
    <div style={{ ...card, cursor: "pointer", border: onMap ? "1px solid #3a80c0" : card.border }} onClick={() => setOpen((v) => !v)}>
      {/* Title row */}
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ color: "#6a86a6", fontSize: 9, width: 16, flex: "0 0 auto" }}>#{rank}</span>
        {hasCoin
          ? <CoinIcon issuer={m.issuer || m.city} value={m.value} metal={m.metal as CoinMetal} size={22}
              title={`${m.coin_name} · ${m.metal} · strength ${sc.toFixed(0)} · value ${m.value.toFixed(2)}×`} />
          : <span style={{ width: 22, textAlign: "center", flex: "0 0 auto" }}>🏛</span>}
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12 }}>{hasCoin ? m.coin_name : m.city}</span>
        <span style={{ flex: 1 }} />
        {m.under_mandate && <span title="Honest-money mandate: debasement barred after a reform" style={{ color: "#7fd0a0", fontSize: 9 }}>⚖ mandate</span>}
        {m.is_reserve && <span style={{ color: "#37a05a", fontSize: 9, fontWeight: 700 }}>RESERVE</span>}
        {hasCoin && (
          <span onClick={(e) => { e.stopPropagation(); toggleMap(); }} title="Highlight the cities that use this coin on the map"
            style={{ fontSize: 10, padding: "1px 6px", borderRadius: 4, marginLeft: 4,
              border: `1px solid ${onMap ? "#3a80c0" : "#24364e"}`, background: onMap ? "#19324a" : "transparent",
              color: onMap ? "#cfe2f6" : "#7fa0c4" }}>📍 map</span>
        )}
        <span style={{ color: "#5a7290", fontSize: 10, marginLeft: 4 }}>{open ? "▾" : "▸"}</span>
      </div>

      {/* MONEY row — one headline strength bar + the two drivers */}
      {hasCoin ? (
        <div style={{ marginTop: 4 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span style={{ color: "#7a90a8", fontSize: 8.5, width: 40, flex: "0 0 auto", textTransform: "uppercase", letterSpacing: 0.4 }}>Money</span>
            <span style={{ flex: 1, height: 8, background: "#141f2c", borderRadius: 4, overflow: "hidden" }}>
              <span style={{ display: "block", height: "100%", width: `${Math.max(3, Math.min(100, sc))}%`, background: strengthColor }} />
            </span>
            <span style={{ color: strengthColor, fontSize: 11, fontWeight: 700, width: 20, textAlign: "right" }}>{sc.toFixed(0)}</span>
            <span style={{ color: m.value >= 1.05 ? "#e0c060" : "#9ab0c8", fontSize: 9 }} title="Exchange agio vs grain">
              {m.value.toFixed(2)}×
            </span>
          </div>
          <div style={{ display: "flex", gap: 12, marginTop: 3, marginLeft: 46, alignItems: "center" }}>
            <DriverBar label="fineness" frac={m.fineness} color={debased ? "#e0a020" : "#8ab0d0"}
              text={`${(m.fineness * 100).toFixed(0)}%${debased ? " ✂" : ""}`} />
            <DriverBar label="trust" frac={m.trust} color={m.is_reserve ? "#37a05a" : "#c8a23a"}
              text={`${(m.trust * 100).toFixed(0)}%`} />
            <span style={{ color: METAL_COLOR[m.metal] ?? "#9ab0c8", fontSize: 8.5, textTransform: "capitalize" }}
              title="The metal this coin is struck in — set by the bullion the polis's region can reach">
              ◈ {m.metal}
            </span>
          </div>
        </div>
      ) : (
        <div style={{ color: "#6a86a6", fontSize: 9.5, marginTop: 3, marginLeft: 22 }}>
          {m.has_mint
            ? "Holds the right of the mint — its first coin is struck at New Year."
            : "No mint right — not yet a large enough commercial centre to be chartered a mint."}
        </div>
      )}

      {/* CIVIC row — the polis behind the mint */}
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 4, flexWrap: "wrap", fontSize: 9 }}>
        <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
          <span style={{ width: 8, height: 8, borderRadius: 2, background: m.council_color }} />
          <span style={{ color: "#9ab0c8" }}>{m.council}{m.council_archetype ? ` · ${m.council_archetype}` : ""}</span>
        </span>
        <span style={{ color: "#d8c878" }} title="Retained civic treasury">🏛 {fmtk(m.treasury)}</span>
        <span style={{ color: "#8aa8c8" }} title="Export / import tariff">⬆{(m.tariff_export * 100).toFixed(1)}% ⬇{(m.tariff_import * 100).toFixed(1)}%</span>
        <span style={{ color: cpi >= 1.15 ? "#e0a880" : "#8aa8c8" }} title="Local price level (CPI, 1.0 = par at start) — rises with debasement / money growth">
          prices ×{cpi.toFixed(2)}
        </span>
        {m.bullion && (
          <span style={{ color: BULLION_COLOR[m.bullion] ?? "#8aa8c8" }}
            title="Bullion supply — how much gold/silver the region can mint. Scarce bullion caps fineness (forces debasement); it is the hard limit on coin supply.">
            ⛏ {m.bullion}
          </span>
        )}
        {m.war_with && <span style={{ color: "#e88" }}>⚔ {m.war_with}</span>}
      </div>

      {open && (
        <div style={{ marginTop: 6, paddingTop: 6, borderTop: "1px solid #1b2a3c", fontSize: 10, lineHeight: 1.5 }}>
          {hasCoin ? (
            <>
              <div style={{ color: "#8aa8c8", marginBottom: 3 }}>
                Minted by <b style={{ color: "#cfe2f6" }}>{m.city}</b>{m.issuer ? ` · council ${m.issuer}` : ""}.
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 4, margin: "2px 0 5px" }}>
                <Stat label="Strength" value={sc.toFixed(0)} hint="Headline 0–100 = fineness × acceptance. The one number to judge the coin by." />
                <Stat label="Held in" value={`${m.held_in} ${m.held_in === 1 ? "city" : "cities"}`} hint="settlements holding this coin in their basket" />
                <Stat label="Abroad" value={`${m.abroad}`} hint="holders outside the home market (reserve reach)" />
                <Stat label="Money supply" value={fmtk(m.circulating)} hint="Σ over holders of (trade volume × this coin's basket share) — now a live driver of local prices" />
                <Stat label="Prices" value={`×${cpi.toFixed(2)}`} hint="local price level (CPI): 1.0 at start, rises with debasement + money growth" />
                {xrate && <Stat label={`vs ${topCoin!.coin_name.split(" ")[0]}`} value={`${xrate.toFixed(2)}×`} hint={`1 ${m.coin_name} ≈ ${xrate.toFixed(2)} ${topCoin!.coin_name} (by value)`} />}
              </div>
              <div style={{ color: "#8aa8c8", marginBottom: 4 }}>
                {debased ? "⚠ Debased coin — " : m.value >= 1.05 ? "★ Hard premium coin — " : "Sound coin — "}
                {m.is_reserve ? "trusted enough to serve as an international reserve." : "circulates mainly in its home market."}
                {m.reformed && <span style={{ color: "#7fd0a0" }}> Has been reformed (called in &amp; re-struck at full fineness).</span>}
                {m.under_mandate && <span style={{ color: "#7fd0a0" }}> An honest-money mandate currently bars further debasement.</span>}
              </div>
              <Explain label="Strength" value={sc.toFixed(0)} text="The headline. fineness × acceptance, 0–100. The two bars above are its drivers — cut the coin (fineness) or lose merchants' confidence (trust) and it falls." />
              <Explain label="Fineness" value={`${(m.fineness * 100).toFixed(0)}%`} text={debased ? "Precious-metal content. Below 100% = DEBASED — the mint skimmed seigniorage into the treasury, which erodes trust AND raises local prices (the inflation tax)." : "Precious-metal content. 100% = full-bodied, honest coin."} />
              <Explain label="Trust (acceptance)" value={`${(m.trust * 100).toFixed(0)}%`} text="How widely merchants accept the coin. Sticky reputation; hit hard by debasement. ≥55% makes it a reserve currency accepted abroad." />
              <Explain label="Price level" value={`×${cpi.toFixed(2)}`} text="v2.0 closed loop: this city's cost of living. Debasing the coin it settles in, or flooding money supply, drives this up — and erodes resident fortunes faster." />
              <Explain label="Bullion supply" value={m.bullion} text={m.bullion === "ample"
                ? "The region has gold/silver to spare — the mint can strike full-bodied coin. Bullion is not the binding constraint here."
                : `Coin SUPPLY is limited by the region's ${m.metal} bullion. Minting beyond the metal forces debasement — fineness is capped down, so scarce bullion is the hard limit on how sound this coin can be.`} />
              <CoinUsageChart usage={usage} />
            </>
          ) : (
            <div style={{ color: "#7e93ab" }}>
              {m.has_mint
                ? "This council seat has just been granted the right of the mint (a charter) and will strike its own coin at New Year. Until then its trade settles in neighbours' currency."
                : "Minting is a privilege of substantial commercial centres. This seat isn't busy or large enough to be chartered a mint yet, so its trade settles in neighbours' currency or by barter."}
            </div>
          )}
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

function Stat({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div title={hint} style={{ background: "#0e1925", border: "1px solid #1c2c40", borderRadius: 4, padding: "2px 7px", minWidth: 52 }}>
      <div style={{ color: "#6a86a6", fontSize: 8, textTransform: "uppercase", letterSpacing: 0.3 }}>{label}</div>
      <div style={{ color: "#cfe2f6", fontSize: 11, fontWeight: 700 }}>{value}</div>
    </div>
  );
}

const COIN_PALETTE = ["#f0d77a", "#c9a227", "#b88f20", "#9c7a1c", "#6fae8a", "#5f97c0", "#52708e"];

/** Which settlements use this coin most — donut (default) / bar. */
function CoinUsageChart({ usage }: { usage: CoinUseCity[] }) {
  const [mode, setMode] = useState<"donut" | "bar">("donut");
  if (usage.length === 0) return <div style={{ color: "#56708e", fontSize: 9, marginTop: 6 }}>No settlements settle in this coin yet.</div>;
  const sorted = [...usage].sort((a, b) => b.volume - a.volume);
  const top = sorted.slice(0, 6);
  const restVol = sorted.slice(6).reduce((s, u) => s + u.volume, 0);
  const rows = top.map((u) => ({ name: u.name, vol: u.volume, mint: u.mint, reserve: u.reserve_reach }));
  if (restVol > 0) rows.push({ name: "others", vol: restVol, mint: false, reserve: false });
  const total = rows.reduce((s, r) => s + r.vol, 0) || 1;
  const tag = (r: { mint: boolean; reserve: boolean }) => (r.mint ? " ★" : r.reserve ? " ⇄" : "");
  return (
    <div style={{ marginTop: 7, paddingTop: 6, borderTop: "1px solid #1b2a3c" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
        <span style={{ color: "#8aa8c8", fontSize: 10, fontWeight: 700, flex: 1 }}>Used by settlements</span>
        <span onClick={(e) => { e.stopPropagation(); setMode("donut"); }} style={chipS(mode === "donut")}>◍</span>
        <span onClick={(e) => { e.stopPropagation(); setMode("bar"); }} style={chipS(mode === "bar")}>▭</span>
      </div>
      {mode === "donut" ? (
        <svg viewBox="0 0 240 130" width="100%">
          {(() => {
            let a0 = -Math.PI / 2; const cx = 56, cy = 62, r = 48, ri = 27;
            return rows.map((rw, i) => {
              const a1 = a0 + (rw.vol / total) * Math.PI * 2;
              const p = (ang: number, rad: number) => `${(cx + rad * Math.cos(ang)).toFixed(1)},${(cy + rad * Math.sin(ang)).toFixed(1)}`;
              const large = rw.vol / total > 0.5 ? 1 : 0;
              const d = `M${p(a0, r)} A${r},${r} 0 ${large} 1 ${p(a1, r)} L${p(a1, ri)} A${ri},${ri} 0 ${large} 0 ${p(a0, ri)} Z`;
              a0 = a1;
              return <path key={i} d={d} fill={COIN_PALETTE[i % COIN_PALETTE.length]} stroke="#0b1420" strokeWidth={1} />;
            });
          })()}
          {rows.map((rw, i) => (
            <g key={i}>
              <rect x={120} y={8 + i * 17 - 8} width={9} height={9} rx={2} fill={COIN_PALETTE[i % COIN_PALETTE.length]} />
              <text x={133} y={8 + i * 17} fontSize={9} fill="#c7d6e8">{rw.name}{tag(rw)} {Math.round((rw.vol / total) * 100)}%</text>
            </g>
          ))}
        </svg>
      ) : (
        <div>
          {rows.map((rw, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 5, padding: "1px 0", fontSize: 9 }}>
              <span style={{ width: 96, color: "#c7d6e8", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{rw.name}{tag(rw)}</span>
              <div style={{ flex: 1, height: 8, background: "#16202c", borderRadius: 3, overflow: "hidden" }}>
                <div style={{ width: `${(rw.vol / total) * 100}%`, height: "100%", background: COIN_PALETTE[i % COIN_PALETTE.length] }} />
              </div>
              <span style={{ width: 28, textAlign: "right", color: "#9ab0c8" }}>{Math.round((rw.vol / total) * 100)}%</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function chipS(on: boolean): React.CSSProperties {
  return { fontSize: 11, padding: "1px 7px", borderRadius: 4, cursor: "pointer",
    border: `1px solid ${on ? "#3a80c0" : "#24364e"}`, background: on ? "#19324a" : "transparent", color: on ? "#cfe2f6" : "#7fa0c4" };
}

// ── Reserves (currency composition per holder: cities / banks / houses) ───────
const HOLDER_TABS = [
  ["cities", "🏙 Settlements"], ["banks", "🏦 Banks"], ["houses", "⚜ Houses"],
] as const;
type HolderKind = typeof HOLDER_TABS[number][0];
const HOLDER_BLURB: Record<HolderKind, string> = {
  cities: "Each settlement's currency reserves — the coins it holds in its basket, by share. The primary (settlement) coin is ringed; ★ marks its own mint.",
  banks: "Each bank's specie reserves, attributed across the coins of its seat and branch cities — so a bank with counting-houses in gold- and silver-coin cities holds both.",
  houses: "Each house's fortune, spread over the coins of the markets where it is active (home city + trade offices) — its currency exposure.",
};

/** Currency-reserve donuts per holder, with a Settlements / Banks / Houses toggle.
 *  Cities carry an explicit basket; banks & houses have their composition derived
 *  from the markets they operate in (see campaign_reserves). */
function ReservesTab({ reserves }: { reserves: ReservesPayload }) {
  const [kind, setKind] = useState<HolderKind>("cities");
  const list = kind === "cities" ? reserves.cities : kind === "banks" ? reserves.banks : reserves.houses;
  return (
    <div style={scroll}>
      <div style={{ display: "flex", gap: 4, marginBottom: 6 }}>
        {HOLDER_TABS.map(([id, lbl]) => (
          <span key={id} onClick={() => setKind(id)} style={chipS(kind === id)}>{lbl}</span>
        ))}
      </div>
      <div style={hint}>{HOLDER_BLURB[kind]}</div>
      {list.length === 0 && <div style={empty}>
        {kind === "banks" ? "No banks yet — none hold currency reserves."
          : kind === "houses" ? "No houses hold coined wealth yet."
          : "No coin in circulation yet — currency baskets form once councils mint and trade carries the coin."}
      </div>}
      {list.map((h, i) => <ReserveCard key={i} h={h} />)}
    </div>
  );
}

function ReserveCard({ h }: { h: ReserveHolder }) {
  const total = h.slices.reduce((s, x) => s + x.share, 0) || 1;
  const primary = h.slices.find((s) => s.primary);
  const cx = 32, cy = 32, r = 28, ri = 16;
  let a0 = -Math.PI / 2;
  const kindLabel = h.kind === "bank" ? "🏦" : h.kind === "house" ? "⚜" : "🏙";
  return (
    <div style={card}>
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <svg viewBox="0 0 64 64" width={56} height={56} style={{ flex: "0 0 auto" }}>
          {h.slices.map((s, i) => {
            const a1 = a0 + (s.share / total) * Math.PI * 2;
            const p = (ang: number, rad: number) => `${(cx + rad * Math.cos(ang)).toFixed(1)},${(cy + rad * Math.sin(ang)).toFixed(1)}`;
            const large = (s.share / total) > 0.5 ? 1 : 0;
            const d = `M${p(a0, r)} A${r},${r} 0 ${large} 1 ${p(a1, r)} L${p(a1, ri)} A${ri},${ri} 0 ${large} 0 ${p(a0, ri)} Z`;
            a0 = a1;
            return <path key={i} d={d} fill={s.color} stroke={s.primary ? "#f2ead0" : "#0b1420"} strokeWidth={s.primary ? 1.4 : 0.8} />;
          })}
        </svg>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
            <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{kindLabel} {h.name}</span>
            <span style={{ flex: 1 }} />
            <span style={{ color: "#8aa8c8", fontSize: 9 }} title="Total reserves / wealth (grain-equivalent)">{fmtk(h.total)}</span>
          </div>
          <div style={{ color: "#7a90a8", fontSize: 8.5 }}>
            {h.seat && <span>{h.seat} · </span>}
            {primary ? <span>holds mainly <b style={{ color: "#d8c878" }}>{primary.coin_name.split(" ")[0]}</b>{primary.mint ? " ★" : ""}</span> : "barter / no coin"}
          </div>
          <div style={{ marginTop: 3 }}>
            {h.slices.slice(0, 4).map((s, i) => (
              <div key={i} style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 9, padding: "0.5px 0" }}>
                <span style={{ width: 8, height: 8, borderRadius: 2, background: s.color, flex: "0 0 auto" }} />
                <span style={{ flex: 1, color: s.primary ? "#cfe2f6" : "#9ab0c8", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {s.coin_name.split(" ")[0]}{s.mint ? " ★" : ""} <span style={{ color: METAL_COLOR[s.metal] ?? "#7a90a8" }}>◈</span>{s.primary ? " · primary" : s.share >= 0.2 ? " · reserve" : ""}
                </span>
                <span style={{ color: "#8aa8c8" }}>{Math.round((s.share / total) * 100)}%</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Bubbles (speculation) ─────────────────────────────────────────────────────
const TIER_COLOR: Record<string, string> = { HIGH: "#e6303a", MED: "#e0a020", LOW: "#37a05a" };
function SpecCard({ c }: { c: SpecCenter }) {
  const col = TIER_COLOR[c.tier] ?? "#37a05a";
  return (
    <div style={card}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
        <span style={{ width: 9, height: 9, borderRadius: 2, background: col, alignSelf: "center", flex: "0 0 auto" }} />
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12 }}>{c.name}</span>
        <span style={{ color: col, fontSize: 10, fontWeight: 700 }}>{c.tier} {c.risk.toFixed(2)}</span>
        <span style={{ color: "#d8c060", fontSize: 10 }}>{"★".repeat(c.stars)}</span>
        <span style={{ flex: 1 }} />
        <span style={{ color: "#8aa0b8", fontSize: 9, fontStyle: "italic" }}>{c.pattern_tag}</span>
      </div>
      <div style={{ marginTop: 4 }}>
        {c.drivers.map((d) => (
          <div key={d.key} style={{ display: "flex", alignItems: "center", gap: 5, marginBottom: 2 }}>
            <span style={{ color: "#9ab0c8", fontSize: 9, width: 78, flex: "0 0 auto" }}>{d.label}</span>
            <span style={{ height: 5, background: col, opacity: 0.8, borderRadius: 2,
              width: `${Math.max(3, Math.min(100, d.weight * 320))}px`, flex: "0 0 auto" }} />
            <span style={{ color: "#c0d0e0", fontSize: 9 }}>{d.detail}</span>
          </div>
        ))}
      </div>
      {c.watch_goods.length > 0 && (
        <div style={{ color: "#e0b060", fontSize: 9, marginTop: 3 }}>Watch: {c.watch_goods.join(", ")}</div>
      )}
    </div>
  );
}

// ── Shocks (wars + crashes + monetary chronicle, interleaved by year) ──────────
const CHRON_ICON: Record<string, string> = { coinage: "🪙", charter: "⚒", reform: "⚖", run: "🏦", bank: "🏦", crash: "📉" };
const CHRON_COLOR: Record<string, string> = { coinage: "#d8c878", charter: "#c0b088", reform: "#7fd0a0", run: "#e0a880", bank: "#8ab0d0", crash: "#e6303a" };
function ShocksTab({ wars, crashes, chronicle }:
  { wars: WarsPayload; crashes: CrashRecord[]; chronicle: MonetaryEvent[] }) {
  const nothing = wars.active.length === 0 && wars.log.length === 0 && crashes.length === 0 && chronicle.length === 0;
  return (
    <div style={scroll}>
      {nothing && <div style={empty}>No shocks yet — the poleis are at peace, credit is holding, and the money is sound.</div>}

      {wars.active.length > 0 && <div style={sectLabel}>⚔ Active wars</div>}
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

      {crashes.length > 0 && <div style={sectLabel}>📉 Financial crashes</div>}
      {crashes.map((c, i) => (
        <div key={`c${i}`} style={card}>
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
      ))}

      {wars.log.length > 0 && <div style={sectLabel}>⚔ Concluded wars</div>}
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

      {chronicle.length > 0 && <div style={sectLabel}>🪙 Monetary chronicle</div>}
      {chronicle.slice(0, 60).map((e, i) => (
        <div key={`m${i}`} style={{ display: "flex", gap: 7, alignItems: "baseline", padding: "3px 2px", borderBottom: "1px solid #101a26" }}>
          <span style={{ color: "#5a7290", fontSize: 9, width: 30, flex: "0 0 auto" }}>y{e.year}</span>
          <span style={{ flex: "0 0 auto" }}>{CHRON_ICON[e.kind] ?? "•"}</span>
          <span style={{ color: CHRON_COLOR[e.kind] ?? "#c0d0e0", fontSize: 9.5, lineHeight: 1.4 }}>{e.text}</span>
        </div>
      ))}
    </div>
  );
}

// ── Banks (T-account balance sheet) ───────────────────────────────────────────
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
  const inCoin = b.coin_name && b.coin_value > 0
    ? (v: number) => `${fmtk(v / b.coin_value)} ${b.coin_name}` : null;
  const runEvent = b.events.find((e) => e.toLowerCase().includes("run"));
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
        <span onClick={() => setShowWhy((s) => !s)} title="Why this bank opened" style={{ color: "#7fb0d8", cursor: "pointer", marginLeft: 6 }}>
          {showWhy ? "▾ why it opened" : "▸ why it opened"}
        </span>
        <span onClick={() => setShowInfo((s) => !s)} title="What the balance sheet means" style={{ color: "#7fb0d8", cursor: "pointer", marginLeft: 6 }}>
          {showInfo ? "▾ explain" : "▸ explain"}
        </span>
      </div>
      {fragile && !b.defunct && (
        <div style={{ color: "#e08080", fontSize: 9, marginTop: 2 }} title="Reserve ratio below 22% — depositors may run">
          ⚠ Fragile — thin reserves; a run could topple it{runEvent ? " (run underway)" : ""}.
        </div>
      )}
      {showInfo && (
        <div style={{ margin: "4px 0", padding: "5px 7px", background: "#0e1a27", border: "1px solid #1e2e42", borderRadius: 6, fontSize: 10, lineHeight: 1.5 }}>
          {inCoin && <div style={{ color: "#d8c878", marginBottom: 3 }}>Amounts below are grain-equivalent; in {b.coin_name} the reserves ≈ {inCoin(b.reserves)}.</div>}
          <Explain label="Specie reserves" value={inCoin ? inCoin(b.reserves) : fmtk(b.reserves)} text="Hard money (coin/bullion) in the vault — what actually backs the bank. The founding 40k starts here." />
          <Explain label="Loans out" value={fmtk(b.loans_out)} text="Credit lent to houses (trade) and to the city treasury (public works), earning interest — an asset until repaid or defaulted." />
          <Explain label="Real estate" value={fmtk(b.real_estate)} text="Counting-houses and foreclosed property booked as assets." />
          <Explain label="Deposits" value={fmtk(b.deposits)} text="Idle capital wealthy families park here for interest — a liability the bank owes back. In a run, depositors pull these out fast." />
          <Explain label="Notes issued" value={fmtk(b.notes_issued)} text="Paper credit the bank put into circulation (its IOUs) — a liability; this is bank-created money." />
          <Explain label="Equity" value={fmtk(b.equity)} text="Net worth = assets − liabilities. If it turns negative the bank fails (the owner first injects capital to save it)." />
          <Explain label="Reserve ratio" value={Number.isFinite(b.reserve_ratio) ? `${(b.reserve_ratio * 100).toFixed(0)}%` : "—"} text="Reserves ÷ liabilities. Below 22% the bank is fragile — depositors run, draining reserves and possibly toppling it." />
        </div>
      )}
      {showWhy && (
        <div style={{ margin: "4px 0", padding: "5px 7px", background: "#0e1a27", border: "1px solid #1e2e42", borderRadius: 6 }}>
          <div style={{ fontSize: 9, color: "#9ab0c8", marginBottom: 3 }}>Chartered in {b.founded_year} because {b.owner} met every founding condition:</div>
          {BANK_GATES.map((g) => (
            <div key={g.label} style={{ display: "flex", gap: 7, alignItems: "baseline", fontSize: 10, padding: "1px 0" }}>
              <span style={{ color: "#5fbf6f", fontWeight: 700, width: 12 }}>✓</span>
              <span style={{ flex: 1, color: "#cbd8e6" }}>{g.label} <span style={{ color: "#6a86a6" }}>{g.need}</span></span>
              <span style={{ color: "#9ab0c8", fontFamily: "ui-monospace,monospace" }}>{house ? g.get(house) : "✓"}</span>
            </div>
          ))}
        </div>
      )}
      <div style={{ display: "flex", gap: 12, marginTop: 5 }}>
        <Side title="Assets" totalColor="#80c890" total={assets}
          rows={[["Specie reserves", b.reserves], ["Loans out", b.loans_out], ["Real estate", b.real_estate]]} />
        <Side title="Liabilities" totalColor="#e0a880" total={liab}
          rows={[["Deposits", b.deposits], ["Notes issued", b.notes_issued], ["Equity", b.equity]]} />
      </div>
      <div style={{ display: "flex", gap: 10, fontSize: 9, color: "#8aa8c8", marginTop: 4, flexWrap: "wrap" }}>
        <span style={{ color: fragile ? "#e6303a" : "#8aa8c8" }} title="Reserves ÷ liabilities — below 22% the bank is fragile (run risk)">
          reserve ratio {Number.isFinite(b.reserve_ratio) ? `${(b.reserve_ratio * 100).toFixed(0)}%` : "—"}{fragile ? " ⚠" : ""}
        </span>
        <span>{b.n_loans} loans</span>
        <span style={{ color: "#80c890" }} title="Cumulative interest earned">+{fmtk(b.interest_earned)}</span>
        {b.losses > 0.01 && <span style={{ color: "#e08080" }} title="Losses written off">−{fmtk(b.losses)}</span>}
      </div>
      {b.branches.length > 0 && (
        <div style={{ fontSize: 9, color: "#7fa0c0", marginTop: 2 }}>Counting-houses: {b.branches.join(", ")}</div>
      )}
      {b.events.length > 0 && (
        <div style={{ fontSize: 9, color: "#6a86a6", marginTop: 2, fontStyle: "italic" }}>{b.events[0]}</div>
      )}
    </div>
  );
}

// ── Schematics ────────────────────────────────────────────────────────────────
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
            <CoinIcon issuer={s.council || s.name} metal={s.coin_metal as CoinMetal} size={16} /> {s.coin_name}
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
  position: "absolute", top: 60, right: 360, width: 372, maxHeight: "82vh",
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
const card: React.CSSProperties = { display: "flex", flexDirection: "column", padding: "6px 4px", borderBottom: "1px solid #131e2a" };
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "12px 10px", lineHeight: 1.5 };
const hint: React.CSSProperties = { color: "#6a86a6", fontSize: 9, marginBottom: 6, lineHeight: 1.5 };
const sectLabel: React.CSSProperties = { color: "#5a6a7e", fontSize: 9, margin: "8px 0 2px", textTransform: "uppercase", letterSpacing: 0.5 };
const blueprintLabel: React.CSSProperties = { color: "#7a90a8", fontSize: 8.5, textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 1 };
const none: React.CSSProperties = { color: "#445268", fontSize: 9, fontStyle: "italic" };
