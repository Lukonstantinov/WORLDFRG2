import { useEffect, useState } from "react";
import { useCampaignStore } from "../state/campaignStore";
import { useUIStore } from "../state/uiStore";
import { CoatOfArms } from "./CoatOfArms";
import { CoinIcon } from "./CoinIcon";
import { YearChronicle } from "./YearChronicle";
import { GOOD_DEFS } from "../goods";
import { campaignGetHouseHistory, campaignMerchantRoutes, campaignHouseLedger, campaignGetBanks } from "../bridge/tauri";
import type { HouseHistory, CampaignDiagnostics, HouseBrief, MerchantRoute, HouseLedger, BankBrief } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

const GOOD_ICON = new Map(GOOD_DEFS.map((g) => [g.name, g.emoji]));
const goodIcon = (name: string) => GOOD_ICON.get(name) ?? "\u{1F4E6}"; // 📦 fallback

/** Desaturate a house colour toward grey — guilds read DULL vs the vivid private
 *  houses, so the civic bodies are visually distinct. */
function dull(hex: string): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex || "");
  if (!m) return "#7a8694";
  const n = parseInt(m[1], 16);
  const mix = (c: number) => Math.round(c * 0.45 + 0x86 * 0.55);
  const r = mix((n >> 16) & 255), g = mix((n >> 8) & 255), b = mix(n & 255);
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, "0")}`;
}

/** "Is trade actually moving?" — a compact health strip above the houses list.
 *  Answers the core merchant-house question: are shipments flowing, how many are
 *  financed by houses vs. local guilds, are voyages being lost, and how much of
 *  the world do houses actually control. */
function TradeDiagnostics({ diag }: { diag: CampaignDiagnostics }) {
  const fleet = diag.fleet_sea + diag.fleet_river + diag.fleet_caravan;
  const moving = diag.shipments_last > 0;
  const housePct = diag.shipments_last > 0
    ? Math.round((diag.by_house / diag.shipments_last) * 100) : 0;
  const stat = (label: string, value: string, color = "#cfe0f4", title?: string) => (
    <div style={diagCell} title={title}>
      <div style={{ color, fontWeight: 700, fontSize: 12 }}>{value}</div>
      <div style={{ color: "#6a86a6", fontSize: 9 }}>{label}</div>
    </div>
  );
  return (
    <div style={diagBar}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
        <span style={{ width: 7, height: 7, borderRadius: "50%", background: moving ? "#5fd08a" : "#d06a5f" }} />
        <span style={{ color: moving ? "#9fe0b8" : "#e0a09a", fontSize: 10, fontWeight: 600 }}>
          {moving ? "Trade is flowing" : "No shipments last advance"}
        </span>
        <span style={{ flex: 1 }} />
        <span style={{ color: "#6a86a6", fontSize: 9 }}>year {diag.year}</span>
      </div>
      <div style={{ display: "flex", gap: 4 }}>
        {stat("shipped", String(diag.shipments_last), "#cfe0f4", "Shipments dispatched last advance")}
        {stat("by houses", `${housePct}%`, housePct > 0 ? "#e0c060" : "#7a90a8", `${diag.by_house} financed by a house, ${diag.by_guild} by local guilds`)}
        {stat("lost", String(diag.lost_last), diag.lost_last > 0 ? "#e0a09a" : "#7a90a8", "Voyages lost to storm/ambush last advance")}
        {stat("in transit", String(diag.in_transit), "#9ab0c8", "Shipments currently in flight")}
      </div>
      <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
        {stat("controls", String(diag.controlled_settlements), diag.controlled_settlements > 0 ? "#9fe0b8" : "#d06a5f", "Settlements a house controls (>=50% of trade throughput)")}
        {stat("ships", String(fleet), "#9ab0c8", `${diag.fleet_sea} sea · ${diag.fleet_river} river · ${diag.fleet_caravan} caravan`)}
        {stat("houses", `${diag.houses_active}`, "#cfe0f4", `${diag.houses_active} active · ${diag.houses_defunct} ruined`)}
        {stat("wealth", diag.total_house_wealth >= 100 ? `${Math.round(diag.total_house_wealth)}` : diag.total_house_wealth.toFixed(1), "#e0c060", "Combined wealth of all active houses")}
      </div>
    </div>
  );
}

/** Merchant Houses browser — every trading family, its coat of arms, head of
 *  family, home city, wealth, the trades it controls (monopolies) and rivals.
 *  Active houses first; ruined ones greyed at the bottom. */
export function HousesPanel() {
  const open = useUIStore((s) => s.showHouses);
  const houses = useCampaignStore((s) => s.houses);
  const diag = useCampaignStore((s) => s.diagnostics);
  const [history, setHistory] = useState<HouseHistory | null>(null);
  const [tab, setTab] = useState<"houses" | "guilds">("houses");
  const [selected, setSelected] = useState<HouseBrief | null>(null);
  const setSelectedHouse = useCampaignStore((s) => s.setSelectedHouse);
  // Focus a house: open its detail AND tell the map to highlight only it.
  const selectHouse = (h: HouseBrief | null) => {
    setSelected(h);
    setSelectedHouse(h?.idx ?? null);
    // Auto-show the House Control map layer so the focused house's sphere is visible.
    if (h) useUIStore.getState().setOverlayVisible("houseControl", true);
  };
  const close = () => useUIStore.getState().setShowHouses(false);
  const openTimeline = (name: string) => {
    campaignGetHouseHistory(name).then((h) => setHistory(h)).catch(() => setHistory(null));
  };
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.houses);
  if (!open) return null;

  const active = houses.filter((h) => !h.defunct);
  const gone = houses.filter((h) => h.defunct);
  const inTab = active.filter((h) => (tab === "guilds") === !!h.is_guild);
  const maxWealth = Math.max(1, ...inTab.map((h) => h.wealth));
  const nHouses = active.filter((h) => !h.is_guild).length;
  const nGuilds = active.filter((h) => h.is_guild).length;

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      {history && <HouseTimeline history={history} onClose={() => setHistory(null)} />}
      {selected && <HouseDetail h={selected} onClose={() => selectHouse(null)} onChronicle={openTimeline} />}
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>⚜️ Trading Families</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>
      {/* Houses vs Guilds tabs */}
      <div style={{ display: "flex", gap: 2, padding: "0 8px", borderBottom: "1px solid #1e2e42" }}>
        {([["houses", `👑 Houses (${nHouses})`], ["guilds", `🏛 Guilds (${nGuilds})`]] as const).map(([id, lbl]) => (
          <div key={id} onClick={() => setTab(id)}
            style={{ padding: "4px 9px", cursor: "pointer", fontSize: 11, fontWeight: tab === id ? 700 : 400,
              color: tab === id ? "#cfe2f6" : "#6a86a6",
              borderBottom: tab === id ? "2px solid #3a80c0" : "2px solid transparent" }}>
            {lbl}
          </div>
        ))}
      </div>
      {diag && <TradeDiagnostics diag={diag} />}
      <div style={{ overflowY: "auto", padding: "4px 8px 10px" }}>
        {houses.length === 0 && (
          <div style={empty}>Begin the campaign (Step 11) — trading families rise as goods start to move.</div>
        )}
        {houses.length > 0 && inTab.length === 0 && (
          <div style={empty}>{tab === "guilds" ? "No civic guilds yet (cities form a guild at 50,000 people)." : "No private houses yet."}</div>
        )}
        {inTab.map((h, i) => (
          <div key={h.name + i} style={{ ...card, cursor: "pointer" }} onClick={() => selectHouse(h)} title="Open this family's detail">
            <CoatOfArms name={h.name} size={30} guild={h.is_guild} />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                {/* Colour chip — vivid for private houses, dull for civic guilds */}
                <span style={{ width: 9, height: 9, borderRadius: 2, background: h.is_guild ? dull(h.color ?? "") : (h.color ?? "#888"), flex: "0 0 auto", alignSelf: "center" }} />
                {h.owns_bank && <span title="Owns a chartered bank" style={{ fontSize: 10 }}>🏦</span>}
                <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12,
                  textDecoration: h.owns_bank ? "underline" : "none", textDecorationColor: "#c9a227" }}>{h.name}</span>
                {h.coin_name && <CoinIcon issuer={h.name} value={h.coin_value} size={14} title={`Mints the ${h.coin_name}`} />}
                {h.is_guild && <span title="A civic Merchant Guild — acts in its home city's interest" style={{ fontSize: 9, color: "#7fd0c0", border: "1px solid #2e5a52", borderRadius: 3, padding: "0 3px" }}>GUILD</span>}
                <span style={{ color: "#6a86a6", fontSize: 9 }}>· {h.home_name}</span>
                <span style={{ flex: 1 }} />
                {h.dominant && <span title="Controls its seat city (>=50% of its trade)" style={{ fontSize: 10 }}>⚖</span>}
                {h.political_power > 0.5 && <span title="A leading political power" style={{ fontSize: 10 }}>👑</span>}
              </div>
              <div style={{ color: "#9ab0c8", fontSize: 10 }}>
                {h.head_name} · gen. {h.generation} · led {h.head_age}y
              </div>
              {/* Trades the house controls — with good icons */}
              {h.specialties.length > 0 && (
                <div style={{ color: "#cdbb88", fontSize: 10, marginTop: 1, display: "flex", flexWrap: "wrap", gap: 4, alignItems: "center" }}>
                  {h.specialties.map((g) => (
                    <span key={g} title={g} style={{ display: "inline-flex", alignItems: "center", gap: 2 }}>
                      <span style={{ fontSize: 11 }}>{goodIcon(g)}</span>{g}
                    </span>
                  ))}
                </div>
              )}
              {h.monopolies.length > 0 && (
                <div style={{ color: "#e0b060", fontSize: 10 }}>
                  {h.monopolies.map(([g, s]) => `${goodIcon(g)} ${g} ${Math.round(s * 100)}%`).join(" · ")} of the trade
                </div>
              )}
              {/* Cities this house trades with (seat first) */}
              {h.cities && h.cities.length > 0 && (
                <div style={{ color: "#88a8c8", fontSize: 9, marginTop: 1 }}>
                  🏙 {h.cities.slice(0, 6).join(", ")}{h.cities.length > 6 ? ` +${h.cities.length - 6}` : ""}
                </div>
              )}
              {/* Fleet — each vessel is one concurrent shipment the house can run */}
              {(() => {
                const sea = h.fleet_sea ?? 0, river = h.fleet_river ?? 0, car = h.fleet_caravan ?? 0;
                if (sea + river + car === 0) return null;
                const parts: string[] = [];
                if (sea) parts.push(`🚢 ${sea} ship${sea > 1 ? "s" : ""}`);
                if (river) parts.push(`🛶 ${river} boat${river > 1 ? "s" : ""}`);
                if (car) parts.push(`🐫 ${car} caravan${car > 1 ? "s" : ""}`);
                return (
                  <div style={{ color: "#a0b8c8", fontSize: 9, marginTop: 1 }}
                    title="Transport capital — each vessel carries one shipment at a time">
                    {parts.join(" · ")}
                  </div>
                );
              })()}
              {/* Foreign offices — footholds in other cities (−5% on goods bought there) */}
              {h.offices && h.offices.length > 0 && (
                <div style={{ color: "#c8a8e0", fontSize: 9, marginTop: 1 }}
                  title="Offices abroad — each gives −5% on goods bought there and a base to trade from">
                  🏢 offices: {h.offices.map(([nm]) => nm).slice(0, 6).join(", ")}
                  {h.offices.length > 6 ? ` +${h.offices.length - 6}` : ""}
                </div>
              )}
              {h.rivals.length > 0 && (
                <div style={{ color: "#c98", fontSize: 9 }}>⚔ rivals: {h.rivals.slice(0, 3).join(", ")}</div>
              )}
              {/* Wealth bar */}
              <div style={{ display: "flex", alignItems: "center", gap: 5, marginTop: 2 }}>
                <div style={{ flex: 1, height: 4, background: "#0a1018", borderRadius: 3, overflow: "hidden" }}>
                  <div style={{ width: `${(h.wealth / maxWealth) * 100}%`, height: "100%", background: "#c9a227" }} />
                </div>
                <span style={{ color: "#c9a227", fontSize: 9, minWidth: 40, textAlign: "right" }}>
                  {h.wealth >= 1000 ? `${(h.wealth / 1000).toFixed(1)}k` : h.wealth.toFixed(0)}
                </span>
              </div>
            </div>
          </div>
        ))}
        {gone.length > 0 && (
          <>
            <div style={{ color: "#5a6a7e", fontSize: 9, margin: "8px 0 2px", textTransform: "uppercase" }}>
              Fallen houses ({gone.length})
            </div>
            {gone.map((h, i) => (
              <div key={"d" + i} style={{ ...card, opacity: 0.55, cursor: "pointer" }} onClick={() => openTimeline(h.name)} title="View this family's timeline">
                <CoatOfArms name={h.name} size={22} guild={h.is_guild} />
                <div style={{ flex: 1 }}>
                  <span style={{ color: "#9aa6b4", fontSize: 11, textDecoration: "line-through" }}>{h.name}</span>
                  <span style={{ color: "#566", fontSize: 9 }}> · once of {h.home_name}</span>
                </div>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}

/** Click-through detail for one house/guild: where it's active, its offices and
 *  estates, its fleet, and its TOP 5 routes (back & forth) with the goods it moves
 *  each way and the volume. */
function HouseDetail({ h, onClose, onChronicle }:
  { h: HouseBrief; onClose: () => void; onChronicle: (name: string) => void }) {
  const [routes, setRoutes] = useState<MerchantRoute[]>([]);
  const [ledger, setLedger] = useState<HouseLedger | null>(null);
  const [bank, setBank] = useState<BankBrief | null>(null);
  const [view, setView] = useState<"summary" | "bank" | "ledger">("summary");
  const tick = useCampaignStore((s) => s.snapshot?.clock.tick ?? 0);
  useEffect(() => {
    let alive = true;
    campaignMerchantRoutes().then((rs) => {
      if (alive) setRoutes(rs.filter((r) => r.holder === h.name).sort((a, b) => b.volume - a.volume).slice(0, 5));
    }).catch(() => {});
    if (h.idx !== undefined) {
      campaignHouseLedger(h.idx).then((l) => { if (alive) setLedger(l); }).catch(() => {});
    }
    // Find this family's bank (if any) so we can show its balance-sheet subtab.
    if (h.owns_bank) {
      campaignGetBanks().then((bs) => {
        if (alive) setBank(bs.find((b) => b.owner_idx === h.idx) ?? null);
      }).catch(() => {});
    } else { setBank(null); }
    return () => { alive = false; };
  }, [h.name, h.idx, h.owns_bank, tick]);
  const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));
  const goodsStr = (gs: [string, number][]) => gs.slice(0, 3).map(([g, v]) => `${goodIcon(g)}${fmt(v)}`).join(" ") || "—";
  const Row = ({ label, children }: { label: string; children: React.ReactNode }) => (
    <div style={{ fontSize: 9, marginTop: 3 }}>
      <span style={{ color: "#6a86a6", textTransform: "uppercase", letterSpacing: 0.4 }}>{label}</span>
      <div style={{ color: "#bcd0e4" }}>{children}</div>
    </div>
  );
  return (
    <div style={detailPanel}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginBottom: 3 }}>
        <span style={{ width: 10, height: 10, borderRadius: 2, background: h.is_guild ? dull(h.color ?? "") : (h.color ?? "#888"), alignSelf: "center" }} />
        {h.owns_bank && <span title="This family owns a chartered bank" style={{ fontSize: 11 }}>🏦</span>}
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13,
          textDecoration: h.owns_bank ? "underline" : "none", textDecorationColor: "#c9a227" }}>{h.name}</span>
        {h.is_guild && <span style={{ fontSize: 8, color: "#7fd0c0" }}>GUILD</span>}
        <span style={{ flex: 1 }} />
        <span onClick={onClose} style={{ color: "#7090b0", cursor: "pointer", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>
      <div style={{ color: "#9ab0c8", fontSize: 10 }}>{h.head_name} · of {h.home_name} · gen {h.generation}</div>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span style={{ color: "#c9a227", fontSize: 11, fontWeight: 700 }}>wealth {fmt(h.wealth)}</span>
        {h.coin_name ? (
          <span style={{ display: "inline-flex", alignItems: "center", gap: 4, fontSize: 9, color: "#d8c878" }}
            title={`Mints the ${h.coin_name} · value ${(h.coin_value ?? 0).toFixed(2)}× · trust ${Math.round((h.coin_trust ?? 0) * 100)}%`}>
            <CoinIcon issuer={h.name} value={h.coin_value} size={16} /> mints {h.coin_name} · {(h.coin_value ?? 0).toFixed(2)}×
          </span>
        ) : null}
      </div>

      {/* Subtabs — Accountant gets its own roomy view so expenses aren't clipped. */}
      <div style={{ display: "flex", gap: 4, margin: "7px 0 5px", borderBottom: "1px solid #1a2a3e" }}>
        {(["summary", ...(bank ? ["bank" as const] : []), "ledger"] as const).map((t) => (
          <div key={t} onClick={() => setView(t)} style={{
            fontSize: 10, padding: "3px 9px", cursor: "pointer",
            color: view === t ? "#e8dcc0" : "#7090b0", fontWeight: view === t ? 700 : 400,
            borderBottom: view === t ? "2px solid #c9a227" : "2px solid transparent",
          }}>{t === "summary" ? "Summary" : t === "bank" ? "🏦 Bank" : `📒 Accountant${ledger && ledger.year > 0 ? ` · yr ${ledger.year}` : ""}`}</div>
        ))}
      </div>

      {view === "bank" && bank ? (
        <BankSheet b={bank} fmt={fmt} />
      ) : view === "summary" ? (
        <>
          <HouseStatGrid h={h} fmt={fmt} />
          {h.active && h.active.length > 0 ? (
            <div style={{ margin: "2px 0 5px" }}>
              <div style={{ color: "#6a86a6", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4 }}>
                Active in ({h.active.length}) — most influential first
              </div>
              {h.active.slice(0, 12).map((c, i) => {
                const mark = c.role === "seat" ? "👑" : c.role === "bailo" ? "🏛️"
                  : c.role === "dominant" ? "◆" : c.role === "office" ? "◇" : "·";
                const roleColor = c.role === "seat" ? "#f4c430" : c.role === "bailo" ? "#e0863a"
                  : c.role === "dominant" ? "#cfe2f6" : "#9fb4cc";
                return (
                  <div key={i} style={{ display: "flex", alignItems: "center", gap: 5, padding: "1px 0" }}>
                    <span style={{ width: 16, textAlign: "center" }}>{mark}</span>
                    <span style={{ flex: 1, minWidth: 0, color: roleColor, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 11 }}>
                      {c.name}{c.role === "bailo" ? " · BAILO" : c.role === "dominant" ? " · dominates" : ""}
                      {c.contested && <span style={{ color: "#e08a8a" }}> ⚔</span>}
                    </span>
                    <div style={{ width: 54, height: 6, background: "#16202c", borderRadius: 3, overflow: "hidden" }}>
                      <div style={{ width: `${Math.round(Math.min(1, c.influence) * 100)}%`, height: "100%", background: roleColor }} />
                    </div>
                    <span style={{ width: 30, textAlign: "right", color: "#7a90a8", fontSize: 9 }}>{c.influence.toFixed(2)}</span>
                  </div>
                );
              })}
            </div>
          ) : (
            h.cities && h.cities.length > 0 && <Row label="Active in">{h.cities.slice(0, 10).join(" · ")}</Row>
          )}
          {h.offices && h.offices.length > 0 && <Row label="Offices">🏢 {h.offices.map(([nm]) => nm).join(" · ")}</Row>}
          {h.estates && h.estates.length > 0 && <Row label="Estates">{h.estates.map(([g, c]) => `${goodIcon(g)} ${g} (${c})`).join(" · ")}</Row>}
          <Row label="Fleet">🚢 {h.fleet_sea ?? 0} · 🛶 {h.fleet_river ?? 0} · 🐫 {h.fleet_caravan ?? 0}</Row>
          {h.barred && h.barred.length > 0 && (
            <div style={{ fontSize: 9, marginTop: 3, color: "#e08a8a" }}>⚔ Barred from (trade war): {h.barred.join(" · ")}</div>
          )}
          <div style={{ color: "#6a86a6", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4, marginTop: 6 }}>Top routes (back &amp; forth)</div>
          {routes.length === 0 && <div style={{ color: "#56708e", fontSize: 9 }}>no active routes right now</div>}
          {routes.map((r, i) => (
            <div key={i} style={{ fontSize: 9, marginBottom: 3, borderBottom: "1px solid #131f2c", paddingBottom: 2 }}>
              <div style={{ color: "#cfe0f4" }}>{r.sea ? "🚢" : "🐫"} {r.a_name} ⇄ {r.b_name} <span style={{ color: "#6a86a6" }}>· vol {fmt(r.volume)}</span></div>
              <div style={{ color: "#9ab0c8" }}>→ {goodsStr(r.out_goods)} · ← {goodsStr(r.ret_goods)}</div>
            </div>
          ))}
          <div onClick={() => onChronicle(h.name)} style={{ color: "#88a8c8", fontSize: 9, cursor: "pointer", marginTop: 5, textDecoration: "underline" }}>
            View family chronicle →
          </div>
        </>
      ) : ledger ? (
        <LedgerView l={ledger} fmt={fmt} />
      ) : (
        <div style={{ color: "#56708e", fontSize: 10, padding: "8px 2px" }}>No completed year yet — the first year's books appear after a full year passes.</div>
      )}
    </div>
  );
}

/** DLC 3.5 · a compact grid of a family's individual stats (top of the Summary). */
function HouseStatGrid({ h, fmt }: { h: HouseBrief; fmt: (v: number) => string }) {
  const year = useCampaignStore((s) => Math.floor((s.snapshot?.clock.tick ?? 0) / 365));
  const age = h.founded_year !== undefined ? Math.max(0, year - h.founded_year) : undefined;
  const Cell = ({ label, value, bar }: { label: string; value: React.ReactNode; bar?: number }) => (
    <div style={{ background: "#0a1119", border: "1px solid #1a2a3c", borderRadius: 4, padding: "3px 5px" }}>
      <div style={{ color: "#6a86a6", fontSize: 8, textTransform: "uppercase", letterSpacing: 0.3 }}>{label}</div>
      <div style={{ color: "#cfe0f4", fontSize: 10, fontWeight: 700 }}>{value}</div>
      {bar !== undefined && (
        <div style={{ height: 3, background: "#0a1018", borderRadius: 2, marginTop: 1, overflow: "hidden" }}>
          <div style={{ width: `${Math.min(100, bar * 100)}%`, height: "100%", background: "#c9a227" }} />
        </div>
      )}
    </div>
  );
  return (
    <>
      {h.archetype_perk ? (
        <div style={{ color: "#9fd0c0", fontSize: 9, marginBottom: 4 }}>{h.archetype_label} · {h.archetype_perk}</div>
      ) : null}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 4 }}>
        <Cell label="Prestige" value={(h.prestige ?? 0).toFixed(2)} bar={h.prestige} />
        <Cell label="Political power" value={(h.political_power ?? 0).toFixed(2)} bar={h.political_power} />
        <Cell label="Trade volume" value={fmt(h.volume ?? 0)} />
        <Cell label="Founded" value={age !== undefined ? `yr ${h.founded_year} · ${age}y` : "—"} />
        <Cell label="Controls" value={String(h.controls?.length ?? 0)} />
        <Cell label="Estates" value={String(h.estates?.length ?? 0)} />
        <Cell label="Offices" value={String(h.offices?.length ?? 0)} />
        <Cell label="Monopolies (all-time)" value={String(h.mono_ever_count ?? 0)} />
        {(h.worst_loss ?? 0) > 0.01 && <Cell label="Worst loss" value={`−${fmt(h.worst_loss ?? 0)}`} />}
        {h.coin_name ? <Cell label="Mints coin" value={`${(h.coin_value ?? 0).toFixed(2)}×`} /> : null}
      </div>
    </>
  );
}

/** DLC 3.5 · a bank's T-account balance sheet (the Bank subtab in a house detail). */
function BankSheet({ b, fmt }: { b: BankBrief; fmt: (v: number) => string }) {
  const assets = b.reserves + b.loans_out + b.real_estate;
  const liab = b.deposits + b.notes_issued;
  const fragile = b.reserve_ratio < 0.22;
  const Side = ({ title, rows, total, color }: { title: string; rows: [string, number][]; total: number; color: string }) => (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ color: "#8aa8c8", fontSize: 9, fontWeight: 700, borderBottom: "1px solid #1c2c40", paddingBottom: 2, marginBottom: 2 }}>{title}</div>
      {rows.map(([k, v]) => (
        <div key={k} style={{ display: "flex", justifyContent: "space-between", fontSize: 9.5, color: "#b8c8da" }}>
          <span>{k}</span><span>{fmt(v)}</span>
        </div>
      ))}
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9.5, fontWeight: 700, color, borderTop: "1px solid #1c2c40", marginTop: 2, paddingTop: 1 }}>
        <span>Σ</span><span>{fmt(total)}</span>
      </div>
    </div>
  );
  return (
    <div style={{ padding: "2px 4px 6px", border: "1px solid #1b2a3c", borderRadius: 4, background: "#0a1119" }}>
      <div style={{ color: "#e8dcc0", fontSize: 11, fontWeight: 700 }}>{b.name}{b.defunct ? " · FAILED" : ""}</div>
      <div style={{ color: "#9ab0c8", fontSize: 9, marginBottom: 4 }}>{b.seat} · est. {b.founded_year}</div>
      <div style={{ display: "flex", gap: 12 }}>
        <Side title="Assets" color="#80c890" total={assets}
          rows={[["Specie reserves", b.reserves], ["Loans out", b.loans_out], ["Real estate", b.real_estate]]} />
        <Side title="Liabilities" color="#e0a880" total={liab}
          rows={[["Deposits", b.deposits], ["Notes issued", b.notes_issued], ["Equity", b.equity]]} />
      </div>
      <div style={{ display: "flex", gap: 10, fontSize: 9, color: "#8aa8c8", marginTop: 4, flexWrap: "wrap" }}>
        <span style={{ color: fragile ? "#e6303a" : "#8aa8c8" }} title="Reserves ÷ liabilities — below 22% = fragile">
          reserve ratio {Number.isFinite(b.reserve_ratio) ? `${Math.round(b.reserve_ratio * 100)}%` : "—"}{fragile ? " ⚠" : ""}
        </span>
        <span>{b.n_loans} loans</span>
        <span style={{ color: "#80c890" }}>+{fmt(b.interest_earned)} earned</span>
        {b.losses > 0.01 && <span style={{ color: "#e08080" }}>−{fmt(b.losses)} lost</span>}
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

/** The yearly ledger (Accountant view): Income then Expenditure stacked FULL-WIDTH
 *  (so nothing is clipped in the narrow panel) + NET + warehouse stock. Per-city
 *  tax/profit lines arrive sorted largest → lowest. */
function LedgerView({ l, fmt }: { l: HouseLedger; fmt: (v: number) => string }) {
  const head: React.CSSProperties = { color: "#6a86a6", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4, marginTop: 7, marginBottom: 2 };
  const Line = ({ label, amt, neg }: { label: string; amt: number; neg?: boolean }) => (
    <div style={{ display: "flex", justifyContent: "space-between", gap: 8, fontSize: 10, padding: "1px 0" }}>
      <span style={{ color: "#9ab0c8", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{label}</span>
      <span style={{ color: neg ? "#e0a0a0" : "#7fcf8f", flexShrink: 0 }}>{neg ? "−" : "+"}{fmt(Math.abs(amt))}</span>
    </div>
  );
  const Sub = ({ label, amt, color }: { label: string; amt: number; color: string }) => (
    <div style={{ display: "flex", justifyContent: "space-between", borderTop: "1px solid #1b2a3c", marginTop: 2, paddingTop: 2, fontSize: 10, fontWeight: 700 }}>
      <span style={{ color: "#8ca4bc" }}>{label}</span>
      <span style={{ color }}>{amt >= 0 ? "+" : "−"}{fmt(Math.abs(amt))}</span>
    </div>
  );
  const anyExp = l.import_tax.length || l.export_tax.length || l.estate_tax || l.upkeep
    || l.fleet_cost || l.lost_cargo || l.events || l.consumption || l.inflation;
  return (
    <div style={{ padding: "2px 4px 6px", border: "1px solid #1b2a3c", borderRadius: 4, background: "#0a1119" }}>
      <div style={head}>Income</div>
      {l.trade_profit.length === 0 && l.office_income === 0 && l.estate_income === 0 &&
        <div style={{ color: "#56708e", fontSize: 9 }}>none recorded yet</div>}
      {l.trade_profit.map((c, i) => <Line key={i} label={`Trade · ${c.label}`} amt={c.amount} />)}
      {l.office_income > 0 && <Line label="Office income" amt={l.office_income} />}
      {l.estate_income > 0 && <Line label="Estate income" amt={l.estate_income} />}
      <Sub label="Total income" amt={l.income_total} color="#7fcf8f" />

      <div style={head}>Expenditure</div>
      {!anyExp && <div style={{ color: "#56708e", fontSize: 9 }}>none recorded yet</div>}
      {l.import_tax.map((c, i) => <Line key={`i${i}`} label={`Import tax · ${c.label}`} amt={c.amount} neg />)}
      {l.export_tax.map((c, i) => <Line key={`e${i}`} label={`Export tax · ${c.label}`} amt={c.amount} neg />)}
      {l.estate_tax > 0 && <Line label="Estate tax" amt={l.estate_tax} neg />}
      {l.upkeep > 0 && <Line label="Upkeep & retainers" amt={l.upkeep} neg />}
      {l.fleet_cost > 0 && <Line label="Fleet upkeep & decay" amt={l.fleet_cost} neg />}
      {l.lost_cargo > 0 && <Line label="Lost cargo" amt={l.lost_cargo} neg />}
      {l.events > 0 && <Line label="Misfortune & fees" amt={l.events} neg />}
      {l.consumption > 0 && <Line label="Feasts & consumption" amt={l.consumption} neg />}
      {l.inflation > 0 && <Line label="Inflation" amt={l.inflation} neg />}
      <Sub label="Total spent" amt={-l.expense_total} color="#e0a0a0" />

      <div style={{ display: "flex", justifyContent: "space-between", borderTop: "1px solid #24364e", marginTop: 5, paddingTop: 4 }}>
        <span style={{ color: "#cfe0f4", fontWeight: 700, fontSize: 11 }}>NET</span>
        <span style={{ color: l.net >= 0 ? "#9fe0a8" : "#e88", fontWeight: 700, fontSize: 11 }}>{l.net >= 0 ? "+" : "−"}{fmt(Math.abs(l.net))}</span>
      </div>
      {l.wealth_graph.length >= 2 && <WealthGraph data={l.wealth_graph} fmt={fmt} />}
      {l.warehouse.length > 0 && (
        <div>
          <div style={head}>Warehouse · {l.warehouse_city}</div>
          <div style={{ fontSize: 11, color: "#bcd0e4", lineHeight: 1.6 }}>{l.warehouse.map((w) => `${goodIcon(w.label)}${fmt(w.amount)}`).join("  ")}</div>
        </div>
      )}
    </div>
  );
}

/** Wealth-through-the-year sparkline (monthly samples) for the Accountant. */
function WealthGraph({ data, fmt }: { data: number[]; fmt: (v: number) => string }) {
  const W = 244, H = 46, pad = 3;
  const max = Math.max(...data), min = Math.min(...data);
  const span = max - min || 1;
  const pts = data.map((v, i) => {
    const x = pad + (i / (data.length - 1)) * (W - 2 * pad);
    const y = pad + (1 - (v - min) / span) * (H - 2 * pad);
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  const up = data[data.length - 1] >= data[0];
  const stroke = up ? "#9fe0a8" : "#e8a0a0";
  return (
    <div style={{ marginTop: 7 }}>
      <div style={{ color: "#6a86a6", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4, marginBottom: 2 }}>Wealth through the year</div>
      <svg width={W} height={H} style={{ display: "block", background: "#0a1119", border: "1px solid #1b2a3c", borderRadius: 3 }}>
        <polyline points={pts} fill="none" stroke={stroke} strokeWidth={1.5} strokeLinejoin="round" />
      </svg>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: "#7a8aa0", marginTop: 1 }}>
        <span>start {fmt(data[0])}</span>
        <span style={{ color: stroke }}>end {fmt(data[data.length - 1])}</span>
      </div>
    </div>
  );
}

const detailPanel: React.CSSProperties = {
  position: "absolute", top: 60, right: 690, width: 290, maxHeight: "78vh", overflowY: "auto",
  background: "#0c141e", border: "1px solid #24364e", borderRadius: 8,
  padding: "9px 11px", boxShadow: "0 8px 28px rgba(0,0,0,0.55)", zIndex: 45,
};

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 320, maxHeight: "78vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 40,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#cfe0f4", fontWeight: 700, fontSize: 12,
};
const card: React.CSSProperties = {
  display: "flex", gap: 8, alignItems: "flex-start", padding: "6px 4px",
  borderBottom: "1px solid #131e2a", cursor: "default",
};
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "10px 4px" };
const diagBar: React.CSSProperties = {
  padding: "6px 10px", borderBottom: "1px solid #1a2a3e", background: "#0a1119",
};
const diagCell: React.CSSProperties = {
  flex: 1, textAlign: "center", padding: "3px 2px", borderRadius: 4,
  background: "#101c28",
};

const EVENT_ICON: Record<string, string> = {
  founded: "🏛", succession: "👤", monopoly: "💰", monopoly_lost: "💸",
  control_gained: "⚖", control_lost: "💔", branch: "🌿", loss: "⚠️", dissolved: "🪦",
};
const EVENT_COLOR: Record<string, string> = {
  founded: "#cfe0f4", succession: "#9ab0c8", monopoly: "#e0b060", monopoly_lost: "#b08a5a",
  control_gained: "#7fd0a0", control_lost: "#d88", loss: "#e08a5a",
  branch: "#9fe07a", dissolved: "#8a93a0",
};

/** A house's chronicle as a vertical timeline: founding, successions, monopolies,
 *  cities controlled (gained/lost + year), the worst loss — plus its most
 *  profitable trade resources. */
function HouseTimeline({ history, onClose }: { history: HouseHistory; onClose: () => void }) {
  const ev = history.events;
  const maxProfit = Math.max(1e-6, ...history.top_goods.map(([, p]) => p));
  return (
    <div style={timelinePanel}>
      <div style={{ ...header, borderBottom: "1px solid #1a2a3e" }}>
        <span style={{ display: "flex", alignItems: "center", gap: 7 }}>
          <span style={{ width: 11, height: 11, borderRadius: 2, background: history.color }} />
          <CoatOfArms name={history.name} size={22} />
          <span>{history.name}</span>
        </span>
        <span style={{ cursor: "pointer", color: "#7a90a8" }} onClick={onClose}>✕</span>
      </div>
      <div style={{ overflowY: "auto", padding: "8px 12px 12px" }}>
        <div style={{ color: "#9ab0c8", fontSize: 10, marginBottom: 8 }}>
          {history.founder || `Founded in year ${history.founded_year}`}
          {history.defunct && <span style={{ color: "#d88" }}> · fallen</span>}
        </div>

        {/* Most profitable resources */}
        {history.top_goods.length > 0 && (
          <>
            <div style={timelineHdr}>Most profitable trade resources</div>
            {history.top_goods.map(([g, p]) => (
              <div key={g} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
                <span style={{ fontSize: 12, width: 16 }}>{goodIcon(g)}</span>
                <span style={{ color: "#cdbb88", fontSize: 10, width: 78, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{g}</span>
                <div style={{ flex: 1, height: 4, background: "#0a1018", borderRadius: 2, overflow: "hidden" }}>
                  <div style={{ width: `${(p / maxProfit) * 100}%`, height: "100%", background: "#c9a227" }} />
                </div>
              </div>
            ))}
          </>
        )}

        {/* Timeline — grouped by year; click a year to expand what happened. */}
        <div style={{ ...timelineHdr, marginTop: 10 }}>Chronicle <span style={{ color: "#56708e", fontWeight: 400 }}>(click a year)</span></div>
        <YearChronicle entries={ev} icons={EVENT_ICON} colors={EVENT_COLOR} />
      </div>
    </div>
  );
}

const timelinePanel: React.CSSProperties = {
  position: "absolute", top: 0, right: 326, width: 300, maxHeight: "78vh",
  display: "flex", flexDirection: "column",
  background: "#0a121c", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.6)", zIndex: 41,
};
const timelineHdr: React.CSSProperties = {
  color: "#7a90a8", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4,
  margin: "4px 0 3px", borderBottom: "1px solid #16222e", paddingBottom: 2,
};
