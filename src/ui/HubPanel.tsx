import { useEffect, useRef, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useGoodsStore } from "../state/goodsStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetHub, campaignWarehouses, campaignFuturesLanes } from "../bridge/tauri";
import type { EconHub, HubCurrency, HubDetail, WarehouseInfo, FuturesLane } from "../types";
import { climatePhrase } from "./climate";
import { CoatOfArms, houseColor } from "./CoatOfArms";
import { CoinIcon } from "./CoinIcon";
import type { HouseBrief } from "../types";
import { SettlementScene } from "./SettlementScene";
import { FlowsView } from "./FlowsView";

type Tab = "summary" | "city" | "govt" | "trade" | "estates" | "depots" | "people";

const LOCAL_COLOR = "#5d6675";  // unaffiliated local merchants (grey)
const GUILD_COLOR = "#4a6a8a";  // organised merchant guilds (slate blue)
const ESTATE_EMOJI: Record<number, string> = { 1: "🌾", 2: "⛏️", 3: "🌿", 4: "🎣", 5: "🍇", 6: "🏭" };
const ESTATE_LABEL: Record<number, string> = { 1: "Farm", 2: "Mine", 3: "Plantation", 4: "Fishery", 5: "Vineyard", 6: "Manufactory" };
const STRUCT_EMOJI: Record<string, string> = {
  Granary: "🌾", Warehouse: "📦", Shipyard: "⚓", Guildhall: "🏛️", Workshop: "🔨",
};

/** Donut chart dividing a settlement's TRADE AMOUNT between the merchant houses
 *  and — always present — the unaffiliated local merchants and the merchant
 *  guilds. EVERY settlement has one: with no houses it's just locals + guilds.
 *  Each house slice is its unique colour. */
function HouseSharePie({ houses, localVolume, guildVolume, merchants }:
  { houses: HouseBrief[]; localVolume: number; guildVolume: number; merchants: number }) {
  const R = 30, r = 17, cx = 34, cy = 34; // outer/inner radius, center
  // Trade amount = recent trade volume; fall back to wealth before any trade has
  // flowed so the chart isn't empty on a fresh campaign.
  const volTotal = houses.reduce((s, h) => s + Math.max(0, h.volume ?? 0), 0);
  const useVol = volTotal > 1e-4;
  const houseVal = (h: HouseBrief) => useVol ? Math.max(0, h.volume ?? 0) : Math.max(0, h.wealth);
  const raw: { name: string; value: number; color: string }[] = houses
    .map((h) => ({ name: h.name, value: houseVal(h), color: h.color ?? houseColor(h.name) }));
  raw.push({ name: "Merchant guilds", value: Math.max(0, guildVolume), color: GUILD_COLOR });
  raw.push({ name: "Local merchants", value: Math.max(0, localVolume), color: LOCAL_COLOR });
  const total = Math.max(1e-6, raw.reduce((s, x) => s + x.value, 0));
  const slices = raw
    .map((x) => ({ ...x, frac: x.value / total }))
    .filter((s) => s.frac > 0.004)
    .sort((a, b) => b.frac - a.frac);
  let a0 = -Math.PI / 2; // start at 12 o'clock
  const arc = (frac: number) => {
    const a1 = a0 + Math.min(frac, 0.9999) * Math.PI * 2;
    const large = frac > 0.5 ? 1 : 0;
    const p = (rad: number, ang: number) => `${cx + rad * Math.cos(ang)},${cy + rad * Math.sin(ang)}`;
    const d = `M ${p(R, a0)} A ${R} ${R} 0 ${large} 1 ${p(R, a1)} L ${p(r, a1)} A ${r} ${r} 0 ${large} 0 ${p(r, a0)} Z`;
    a0 = a1;
    return d;
  };
  const top = slices[0];
  const nHouses = houses.length;
  return (
    <div style={{ display: "flex", gap: 10, alignItems: "center", margin: "2px 0 6px" }}>
      <svg width={68} height={68} viewBox="0 0 68 68" style={{ flex: "0 0 auto" }}>
        {slices.map((s) => (
          <path key={s.name} d={arc(s.frac)} fill={s.color} stroke="#0c1118" strokeWidth={0.6}>
            <title>{`${s.name}: ${Math.round(s.frac * 100)}%`}</title>
          </path>
        ))}
        {slices.length === 0 && <circle cx={cx} cy={cy} r={R} fill="#16222e" />}
      </svg>
      <div style={{ fontSize: 10, color: "#9ab0c8", lineHeight: 1.5 }}>
        <div style={{ color: "#cfe0f4", fontWeight: 600 }}>
          {nHouses} {nHouses === 1 ? "house" : "houses"} · {Math.round(merchants)} merchants
        </div>
        {top && (
          <div>Leads trade: <span style={{ color: top.color, fontWeight: 600 }}>{top.name}</span> ({Math.round(top.frac * 100)}%)</div>
        )}
        <div style={{ color: "#7f8a99" }}>by trade volume moved</div>
      </div>
    </div>
  );
}

/** Hub inspector: click a trade hub → a tabbed settlement window. Overview holds
 *  the identity, population mood and character; Market the prices (× world value,
 *  supply/demand, currency, cheapest/dearest, exports & imports); Society the
 *  classes, luxuries and shortages; History the chronicle + charts (live once a
 *  campaign is running). Falls back to the static economy snapshot pre-campaign. */
export function HubPanel() {
  const selectedHub = useUIStore((s) => s.selectedHub);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const selectedChain = useUIStore((s) => s.selectedChain);
  const setSelectedChain = useUIStore((s) => s.setSelectedChain);
  const selectedExport = useUIStore((s) => s.selectedExport);
  const setSelectedExport = useUIStore((s) => s.setSelectedExport);
  const economy = useWorldStore((s) => s.economy);
  const goodMeta = useGoodsStore((s) => s.meta);
  const campActive = useCampaignStore((s) => s.snapshot?.active ?? false);
  // The campaign tick — re-fetch the hub detail whenever it advances so the open
  // settlement's prices/wealth/population update live alongside the campaign.
  const campTick = useCampaignStore((s) => s.snapshot?.clock.tick ?? 0);

  const [tab, setTab] = useState<Tab>("summary");
  const [tradeView, setTradeView] = useState<"market" | "flows">("market");
  const [detail, setDetail] = useState<HubDetail | null>(null);
  const [depots, setDepots] = useState<WarehouseInfo[]>([]);
  const [lanes, setLanes] = useState<FuturesLane[]>([]);
  const setFuturesFocus = useUIStore((s) => s.setFuturesFocus);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const setFlowHighlight = useUIStore((s) => s.setFlowHighlight);

  // Reset to the Overview tab whenever a different hub is opened.
  useEffect(() => { setTab("summary"); setTradeView("market"); }, [selectedHub]);
  // Clear any map flow-highlight when leaving the Flows view (or the panel).
  useEffect(() => {
    if (!(tab === "trade" && tradeView === "flows")) setFlowHighlight([]);
    return () => setFlowHighlight([]);
  }, [tab, tradeView, selectedHub, setFlowHighlight]);

  // Warehouses sited in this city + the futures lanes touching it (for the Depots tab).
  useEffect(() => {
    if (tab !== "depots" || !campActive) return;
    let alive = true;
    campaignWarehouses().then((w) => { if (alive) setDepots(w); }).catch(() => {});
    campaignFuturesLanes().then((l) => { if (alive) setLanes(l); }).catch(() => {});
    return () => { alive = false; };
  }, [tab, campActive, campTick]);

  // Pull live per-hub detail (sentiment/market/history) while a campaign runs,
  // refreshed every time the campaign tick changes.
  useEffect(() => {
    let alive = true;
    if (selectedHub === null || !campActive) { setDetail(null); return; }
    campaignGetHub(selectedHub).then((d) => { if (alive) setDetail(d); }).catch(() => { if (alive) setDetail(null); });
    return () => { alive = false; };
  }, [selectedHub, campActive, campTick]);

  // Accumulate per-good local price (and the world average) over ticks for the
  // Market price graphs — reset whenever a different city is opened.
  const priceHist = useRef<{ hub: number | null; data: Record<string, { local: number[]; world: number[] }> }>({ hub: null, data: {} });
  useEffect(() => {
    if (!detail) return;
    const ph = priceHist.current;
    if (ph.hub !== detail.id) { ph.hub = detail.id; ph.data = {}; }
    for (const g of detail.goods) {
      const xw = g.price / Math.max(1e-6, g.base_value);
      const e = ph.data[g.name] ?? (ph.data[g.name] = { local: [], world: [] });
      e.local.push(xw); e.world.push(g.world_avg ?? 1);
      if (e.local.length > 80) { e.local.shift(); e.world.shift(); }
    }
  }, [detail]);

  if (selectedHub === null || !economy) return null;
  const hub = economy.hubs.find((h) => h.id === selectedHub);
  if (!hub) return null;

  const iconFor = (id: string) => goodMeta(id).icon;
  const labelFor = (id: string) => goodMeta(id).name;
  const hubName = (id: number) => economy.hubs.find((h) => h.id === id)?.name ?? `Hub ${id}`;
  const chain = selectedChain !== null ? economy.chains.find((c) => c.id === selectedChain) ?? null : null;
  const fmt = (v: number) => v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0);
  const stars = Math.max(1, Math.min(5, hub.stars));

  const topHub = economy.hubs.reduce((a, b) => (b.throughput ?? 0) > (a.throughput ?? 0) ? b : a, economy.hubs[0]);
  const isTop = !!topHub && topHub.id === hub.id;
  const cmp = isTop
    ? `≈${Math.round(hub.ref_pct ?? 100)}% of ${hub.nearest_ref ?? "Venice"}`
    : `≈${Math.round(((hub.throughput ?? 0) / ((topHub?.throughput) || 1)) * 100)}% of ${topHub?.name ?? "the capital"}`;
  const wealthSorted = [...economy.hubs].sort((a, b) => b.wealth - a.wealth);
  const wealthRank = wealthSorted.findIndex((h) => h.id === hub.id) + 1;

  const desireClass = (good: string): string | null =>
    economy.good_stats?.find((g) => g.good_name === good)?.biggest_desire_class ?? null;

  const shortageReason = (r: string): string => ({
    no_supplier: "produced nowhere reachable",
    unreachable: "no trade route reaches a producer",
    no_port: "landlocked — cannot reach a sea producer",
    deficit: "local demand outstrips supply",
  } as Record<string, string>)[r] ?? "scarce";

  // ── Cargo both ways: aggregate goods over every corridor touching this hub ──
  const outMap = new Map<string, number>();
  const inMap = new Map<string, number>();
  for (const c of economy.corridors ?? []) {
    if (c.a !== hub.id && c.b !== hub.id) continue;
    const out = c.a === hub.id ? c.fwd_goods : c.bwd_goods;
    const inc = c.a === hub.id ? c.bwd_goods : c.fwd_goods;
    for (const g of out) outMap.set(g.good_name, (outMap.get(g.good_name) ?? 0) + g.value);
    for (const g of inc) inMap.set(g.good_name, (inMap.get(g.good_name) ?? 0) + g.value);
  }
  const toSorted = (m: Map<string, number>) =>
    [...m.entries()].map(([good_name, value]) => ({ good_name, value })).sort((a, b) => b.value - a.value);
  const outCargo = toSorted(outMap);
  const inCargo = toSorted(inMap);
  const cargoMax = Math.max(1e-6, ...outCargo.map((g) => g.value), ...inCargo.map((g) => g.value));

  const TABS: { id: Tab; label: string }[] = [
    { id: "summary", label: "Summary" },
    ...(detail ? [{ id: "city" as Tab, label: detail.is_estate ? "Estate" : "City" }] : []),
    ...(detail && !detail.is_estate ? [{ id: "govt" as Tab, label: "Government" }] : []),
    { id: "trade", label: "Trade" },
    { id: "estates", label: "Estates" },
    ...(campActive ? [{ id: "depots" as Tab, label: "Depots" }] : []),
    { id: "people", label: "People" },
  ];

  return (
    <div style={{ ...panel, width: tab === "trade" ? 600 : 360 }}>
      {/* ── Title + stats header (always visible) ── */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 4 }}>
        <div>
          <div style={{ color: isTop ? "#f4c430" : hub.emporium ? "#ff8a6a" : "#e8d8b0", fontSize: 15, fontWeight: 700 }}>
            {isTop ? "🟨 " : hub.emporium ? "🔺 " : ""}{hub.name}
            {isTop && <span style={{ color: "#f4c430", fontSize: 9, marginLeft: 5 }}>GREATEST HUB</span>}
            {!isTop && hub.emporium && <span style={{ color: "#ff6a4a", fontSize: 9, marginLeft: 5 }}>EMPORIUM</span>}
          </div>
          <div style={{ color: "#8aa0c0", fontSize: 10 }}>
            <span style={{ color: "#ffd24a" }}>{"★".repeat(stars)}</span>
            {`  ${cmp}`}
            <span style={{ color: "#6a86a6" }}>{`  ·  wealth rank #${wealthRank}/${economy.hubs.length}`}</span>
            {hub.sea_access === false && <span style={{ color: "#6a86a6" }}>{"  ·  lake/inland"}</span>}
          </div>
        </div>
        <span onClick={() => setSelectedHub(null)}
          style={{ color: "#7090b0", cursor: "pointer", fontSize: 18, lineHeight: 1 }} title="Close">×</span>
      </div>

      {/* ── Tab bar ── */}
      <div style={{ display: "flex", gap: 2, margin: "2px 0 6px", borderBottom: "1px solid #1e2e42" }}>
        {TABS.map((t) => (
          <div key={t.id} onClick={() => setTab(t.id)}
            style={{
              padding: "4px 9px", cursor: "pointer", fontSize: 11, fontWeight: tab === t.id ? 700 : 400,
              color: tab === t.id ? "#cfe2f6" : "#6a86a6",
              borderBottom: tab === t.id ? "2px solid #3a80c0" : "2px solid transparent",
            }}>
            {t.label}
          </div>
        ))}
      </div>

      {/* ════════════ CITY / ESTATE SCHEMATIC ════════════ */}
      {tab === "city" && detail && (
        <SettlementScene detail={detail} />
      )}
      {tab === "city" && !detail && (
        <div style={{ color: "#7a8aa0", fontSize: 11, padding: "8px 2px" }}>
          The building schematic appears once a campaign is running.
        </div>
      )}

      {/* ════════════ GOVERNMENT (DLC 3 polis) ════════════ */}
      {tab === "govt" && (() => {
        const g = detail?.government;
        if (!g) return (
          <div style={{ color: "#7a8aa0", fontSize: 11, padding: "8px 2px" }}>
            The city government appears once a campaign is running.
          </div>
        );
        const pct = (x: number) => `${(x * 100).toFixed(1)}%`;
        const tierColor = g.spec_tier === "HIGH" ? "#ff6a4a" : g.spec_tier === "MED" ? "#f4c430" : "#6fae6f";
        const govRow = (label: string, val: string, warn = false) => (
          <div style={{ display: "flex", justifyContent: "space-between", padding: "2px 0", fontSize: 11 }}>
            <span style={{ color: "#8aa0c0" }}>{label}</span>
            <span style={{ color: warn ? "#ff8a6a" : "#e8dcc0", fontWeight: 600 }}>{val}</span>
          </div>
        );
        return (
          <div style={{ fontSize: 11, color: "#c7d6e8" }}>
            <div style={sectionHdr}>Council</div>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
              {g.council !== "—"
                ? <CoatOfArms name={g.council} size={32} guild={g.council_is_guild} />
                : <span style={{ fontSize: 24 }}>🏛️</span>}
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ color: "#e8dcc0", fontWeight: 600 }}>{g.council}</div>
                <div style={{ color: "#8aa0c0", fontSize: 10 }}>
                  {g.council_archetype || (g.council === "—" ? "no governing house" : "")}
                  {g.council_is_guild ? " · civic guild" : ""}
                </div>
              </div>
              {g.council !== "—" && (
                <div style={{ textAlign: "right", minWidth: 60 }}>
                  <div style={{ fontSize: 9, color: "#6a86a6" }}>power</div>
                  <div style={{ height: 6, background: "#1e2e42", borderRadius: 3, overflow: "hidden" }}>
                    <div style={{ width: `${Math.round(Math.min(1, g.council_power) * 100)}%`, height: "100%", background: "#c9a227" }} />
                  </div>
                </div>
              )}
            </div>

            <div style={sectionHdr}>
              Fiscal policy{" "}
              {g.tariff_default && <span style={{ color: "#6a86a6", fontWeight: 400, fontSize: 9 }}>(default — no council yet)</span>}
            </div>
            {govRow("Export tariff", pct(g.tariff_export))}
            {govRow("Import tariff", pct(g.tariff_import))}
            {govRow("Mint fineness", g.mint_fineness.toFixed(2), g.mint_fineness < 0.97)}
            {g.mint_fineness < 0.97 && (
              <div style={{ color: "#ff8a6a", fontSize: 9, marginTop: 1 }}>⚠ debased coin — "cheap money"</div>
            )}

            <div style={sectionHdr}>Treasury</div>
            {govRow("City treasury", fmt(g.treasury))}
            {govRow("Circulating civic pool", fmt(g.civic_pool))}

            <div style={sectionHdr}>🫧 Speculation</div>
            {g.spec_tier ? (
              <>
                <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginBottom: 3, flexWrap: "wrap" }}>
                  <span style={{ color: tierColor, fontWeight: 700 }}>{g.spec_tier}</span>
                  <span style={{ color: "#ffd24a" }}>
                    {"●".repeat(g.spec_stars)}<span style={{ color: "#33404f" }}>{"○".repeat(Math.max(0, 5 - g.spec_stars))}</span>
                  </span>
                  <span style={{ color: "#8aa0c0" }}>({g.spec_risk.toFixed(2)})</span>
                  <span style={{ color: "#7a8aa0", fontStyle: "italic" }}>{g.spec_pattern}</span>
                </div>
                {g.spec_drivers.slice(0, 4).map((d, i) => (
                  <div key={i} style={{ color: "#9fb4cc", fontSize: 10, marginLeft: 4 }}>• {d}</div>
                ))}
                {g.spec_watch.length > 0 && (
                  <div style={{ color: "#c9a227", fontSize: 10, marginTop: 3 }}>Watch goods: {g.spec_watch.join(", ")}</div>
                )}
              </>
            ) : (
              <div style={{ color: "#6fae6f", fontSize: 10 }}>Calm — no speculative pressure detected this year.</div>
            )}
          </div>
        );
      })()}

      {/* ════════════ OVERVIEW ════════════ */}
      {tab === "summary" && (
        <>
          <div style={statGrid}>
            <Stat label="Throughput" value={fmt(hub.throughput ?? 0)} />
            <Stat label="Exports →" value={fmt(hub.exports ?? 0)} />
            <Stat label="← Imports" value={fmt(hub.imports ?? 0)} />
            <Stat label="Partners" value={String(hub.partners ?? 0)} />
            <Stat label="Wealth" value={`${Math.round(hub.wealth * 100)}%`} />
            <Stat label="Population" value={(detail?.population ?? hub.population).toLocaleString()} />
          </div>
          {detail && (detail.estate_kind ?? 0) > 0 && (
            <div style={estateBox}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                <span style={{ fontSize: 13 }}>{ESTATE_EMOJI[detail.estate_kind ?? 0] ?? "🏡"}</span>
                <span style={{ color: "#cdbb88", fontWeight: 700, fontSize: 12 }}>
                  {ESTATE_LABEL[detail.estate_kind ?? 0] ?? "Estate"}
                </span>
                <span style={{ flex: 1 }} />
                <span style={{ color: "#7fd0a0", fontSize: 9 }}>income → owner</span>
              </div>
              <div style={{ color: "#9ab0c8", fontSize: 10, marginTop: 2 }}>
                Owned by <span style={{ color: "#e8dcc0" }}>{detail.estate_owner || "—"}</span>
                {detail.estate_good && <> · works {iconFor(detail.estate_good)} {labelFor(detail.estate_good)}</>}
              </div>
            </div>
          )}
          {hub.top_export && (
            <div style={{ color: "#e0c060", fontSize: 11, margin: "5px 0 2px", display: "flex", gap: 6, alignItems: "baseline" }}>
              <span style={{ color: "#6a86a6", fontSize: 10 }}>Richest trade:</span>
              <span style={{ fontWeight: 700 }}>{iconFor(hub.top_export)} {labelFor(hub.top_export)}</span>
              {hub.top_export_share !== undefined && hub.top_export_share > 0 && (
                <span style={{ color: "#9ab0c8", fontSize: 10 }}>
                  {Math.round(hub.top_export_share * 100)}% of export value
                </span>
              )}
            </div>
          )}
          {hub.monopolies && hub.monopolies.length > 0 && (
            <div style={{ color: "#9ab0c8", fontSize: 10, margin: "4px 0 2px" }}>
              <span style={{ color: "#6a86a6" }}>Monopolies: </span>
              {hub.monopolies.map((m) => `${iconFor(m)} ${labelFor(m)}`).join(", ")}
            </div>
          )}

          {/* Buildings (structures) the city has erected */}
          {detail && (detail.structures?.length ?? 0) > 0 && (
            <>
              <div style={{ ...sectionHdr, marginTop: 6 }}>Buildings</div>
              {detail.structures!.map(([nm, eff], i) => (
                <div key={i} style={{ display: "flex", gap: 6, alignItems: "baseline", fontSize: 10, padding: "1px 2px" }}>
                  <span style={{ fontSize: 12 }}>{STRUCT_EMOJI[nm] ?? "🏗️"}</span>
                  <span style={{ color: "#cdbb88", fontWeight: 700 }}>{nm}</span>
                  <span style={{ flex: 1 }} />
                  <span style={{ color: "#7fbf9a" }}>{eff}</span>
                </div>
              ))}
            </>
          )}

          {/* DLC 3.5 · City finances — the treasury books (taxes in, spending out) */}
          {detail && (detail.treasury !== undefined) && (
            <CityFinances detail={detail} />
          )}

          {/* Population mood + drivers */}
          <div style={{ ...sectionHdr, marginTop: 6 }}>The people</div>
          {detail ? (
            <MoodCard detail={detail} />
          ) : (
            <div style={emptyTxt}>Begin the campaign (Step 11) to see how the people feel.</div>
          )}

          {/* Character summary */}
          <div style={blurbBox}>{peopleSummary(hub, labelFor, topHub?.name, isTop)}</div>
        </>
      )}

      {/* ════════════ TRADE (market flow) ════════════ */}
      {tab === "trade" && (
        <>
          {/* Sub-toggle: live Market view vs realized-trade Flows view */}
          <div style={{ display: "flex", gap: 4, marginBottom: 5 }}>
            {(["market", "flows"] as const).map((v) => (
              <div key={v} onClick={() => setTradeView(v)} style={{
                padding: "2px 10px", cursor: "pointer", fontSize: 10, borderRadius: 4,
                background: tradeView === v ? "#21344a" : "#16202c",
                color: tradeView === v ? "#cfe2f6" : "#6a86a6",
                border: tradeView === v ? "1px solid #3a80c0" : "1px solid #1e2e42",
              }}>{v === "market" ? "Market" : "Flows"}</div>
            ))}
          </div>
          {tradeView === "flows" && (
            <FlowsView hubId={hub.id} active={campActive} tick={campTick} setFlowHighlight={setFlowHighlight} />
          )}
          {tradeView === "market" && (<>
          {/* Live market FLOW: arrivals ⇢ market ⇢ departures (campaign only) */}
          {detail ? (
            <>
              <div style={{ display: "flex", gap: 6 }}>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={sectionHdr}>⇢ Arrivals</div>
                  {(detail.arrivals ?? []).length === 0 && (detail.recent_arrivals ?? []).length === 0 && <div style={emptyTxt}>none inbound</div>}
                  {(detail.arrivals ?? []).slice(0, 14).map((s, i) => (
                    <ShipRow key={"a" + i} s={s} side="in" icon={iconFor} label={labelFor} />
                  ))}
                  {(detail.recent_arrivals ?? []).length > 0 && (
                    <div style={{ color: "#56708e", fontSize: 8, margin: "3px 0 1px", borderTop: "1px solid #131f2c", paddingTop: 2 }}>recent</div>
                  )}
                  {(detail.recent_arrivals ?? []).slice(0, 10).map((s, i) => (
                    <ShipRow key={"ra" + i} s={s} side="in" icon={iconFor} label={labelFor} faded />
                  ))}
                </div>
                <div style={{ flex: 1.25, minWidth: 0, borderLeft: "1px solid #1e2e42", borderRight: "1px solid #1e2e42", padding: "0 6px" }}>
                  <div style={{ textAlign: "center", color: "#e8d8b0", fontSize: 11, fontWeight: 700 }}>Market</div>
                  <div style={{ textAlign: "center", color: "#9ab0c8", fontSize: 9, marginBottom: 3 }}>
                    💰 bought {fmt(detail.bought ?? 0)} · sold {fmt(detail.sold ?? 0)}
                  </div>
                  {(() => {
                    // Per-good import / export amounts reaching THIS market, summed
                    // from the in-flight + recent shipments (whoever carried them).
                    const sumBy = (rows?: typeof detail.arrivals) => {
                      const m: Record<string, number> = {};
                      for (const s of rows ?? []) m[s.good] = (m[s.good] ?? 0) + s.amount;
                      return m;
                    };
                    const imp = sumBy([...(detail.arrivals ?? []), ...(detail.recent_arrivals ?? [])]);
                    const exp = sumBy([...(detail.departures ?? []), ...(detail.recent_departures ?? [])]);
                    const traded = new Set([...Object.keys(imp), ...Object.keys(exp)]);
                    const rows = [...detail.goods]
                      .filter((g) => g.production > 0.01 || g.stock > 0.01 || traded.has(g.name))
                      .sort((a, b) => (b.production + (imp[b.name] ?? 0)) - (a.production + (imp[a.name] ?? 0)))
                      .slice(0, 16);
                    return (<>
                      <div style={{ display: "flex", gap: 3, fontSize: 8, color: "#56708e" }}>
                        <span style={{ flex: 1 }}>good</span>
                        <span style={{ minWidth: 30, textAlign: "right" }}>made</span>
                        <span style={{ minWidth: 26, textAlign: "right", color: "#7fd0a0" }}>in</span>
                        <span style={{ minWidth: 26, textAlign: "right", color: "#e0a080" }}>out</span>
                        <span style={{ minWidth: 26, textAlign: "right" }}>×</span>
                      </div>
                      {rows.map((g) => {
                        const xw = g.price / Math.max(1e-6, g.base_value);
                        const gi = imp[g.name] ?? 0, go = exp[g.name] ?? 0;
                        return (
                          <div key={g.good} style={{ display: "flex", gap: 3, fontSize: 9, alignItems: "baseline" }}>
                            <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", color: "#c0d0e0" }}>{iconFor(g.name)} {labelFor(g.name)}</span>
                            <span style={{ minWidth: 30, textAlign: "right", color: "#9ab0c8" }}>{g.production > 0.01 ? fmt(g.production) : "—"}</span>
                            <span style={{ minWidth: 26, textAlign: "right", color: "#7fd0a0" }}>{gi > 0.01 ? fmt(gi) : "·"}</span>
                            <span style={{ minWidth: 26, textAlign: "right", color: "#e0a080" }}>{go > 0.01 ? fmt(go) : "·"}</span>
                            <span style={{ minWidth: 26, textAlign: "right", fontWeight: 600, color: xw > 1.3 ? "#e08080" : xw < 0.77 ? "#7fd0a0" : "#c0d0e0" }}>{xw.toFixed(1)}×</span>
                          </div>
                        );
                      })}
                    </>);
                  })()}
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ ...sectionHdr, textAlign: "right" }}>Departures ⇢</div>
                  {(detail.departures ?? []).length === 0 && (detail.recent_departures ?? []).length === 0 && <div style={emptyTxt}>none outbound</div>}
                  {(detail.departures ?? []).slice(0, 14).map((s, i) => (
                    <ShipRow key={"d" + i} s={s} side="out" icon={iconFor} label={labelFor} />
                  ))}
                  {(detail.recent_departures ?? []).length > 0 && (
                    <div style={{ color: "#56708e", fontSize: 8, margin: "3px 0 1px", borderTop: "1px solid #131f2c", paddingTop: 2, textAlign: "right" }}>recent</div>
                  )}
                  {(detail.recent_departures ?? []).slice(0, 10).map((s, i) => (
                    <ShipRow key={"rd" + i} s={s} side="out" icon={iconFor} label={labelFor} faded />
                  ))}
                </div>
              </div>
              {/* DLC 3.5 · Transit — the carrying trade: this city's merchants hauling
                  goods between OTHER cities (the entrepôt handling-trade). */}
              <div style={{ ...sectionHdr, marginTop: 8 }}>
                Transit — carrying trade <span style={{ color: "#56708e", fontWeight: 400 }}>(our merchants moving goods between other cities)</span>
              </div>
              {(detail.transit ?? []).length === 0 && <div style={emptyTxt}>no goods passing through our hands right now</div>}
              {(detail.transit ?? []).map((t, i) => (
                <div key={"t" + i} style={{ borderBottom: "1px solid #131f2c", padding: "2px 0" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 9.5 }}>
                    <span style={{ width: 8, height: 8, borderRadius: 2, background: t.color, flex: "0 0 auto" }} />
                    <span style={{ color: "#cbd8e6", fontWeight: 600 }}>{t.merchant}{t.is_guild ? " (guild)" : ""}</span>
                    <span style={{ color: "#cdbb88" }}>{iconFor(t.good)} {labelFor(t.good)} ×{fmt(t.amount)}</span>
                    <span style={{ flex: 1 }} />
                    <span style={{ fontSize: 10 }}>{t.sea ? "🚢" : "🐫"}</span>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 9, color: "#8aa8c8" }}>
                    <span style={{ color: "#9ab0c8" }}>{t.from_name}</span>
                    <span style={{ flex: 1, height: 1, background: "#24364e" }} />
                    <span>▶</span>
                    <span style={{ color: "#cfe0f4" }}>{t.to_name}</span>
                  </div>
                  <div style={{ fontSize: 9, color: "#7a90a8" }}>
                    {t.coin
                      ? <>deal {fmt(t.value)} <span style={{ color: "#d8c878" }}>{t.coin}</span></>
                      : <>barter <span style={{ color: "#9ab0c8" }}>{t.barter}</span></>}
                  </div>
                </div>
              ))}

              <div style={{ ...sectionHdr, marginTop: 8 }}>Prices — local vs world average (live)</div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "4px 10px" }}>
                {[...detail.goods].filter((g) => g.production > 0.01 || g.stock > 0.01)
                  .sort((a, b) => b.production - a.production).slice(0, 6).map((g) => {
                  const e = priceHist.current.data[g.name];
                  return (
                    <div key={g.good}>
                      <div style={{ fontSize: 9, color: "#9ab0c8", overflow: "hidden", whiteSpace: "nowrap", textOverflow: "ellipsis" }}>
                        {iconFor(g.name)} {labelFor(g.name)}
                        <span style={{ color: "#e0c060" }}> {(g.price / Math.max(1e-6, g.base_value)).toFixed(2)}×</span>
                        <span style={{ color: "#8aa0c0" }}> · w {(g.world_avg ?? 1).toFixed(2)}×</span>
                      </div>
                      {e && e.local.length > 1 ? <DualSpark local={e.local} world={e.world} /> : <div style={{ fontSize: 8, color: "#56708e" }}>gathering…</div>}
                    </div>
                  );
                })}
              </div>
              <div style={{ fontSize: 8, color: "#56708e", marginTop: 2 }}>
                <span style={{ color: "#e0c060" }}>━ local price</span> · <span style={{ color: "#8aa0c0" }}>┈ world average</span>
              </div>
            </>
          ) : hub.market ? (
            hub.market.prices.slice(0, 16).map((p) => (
              <div key={p.good} style={{ display: "flex", alignItems: "baseline", gap: 6, fontSize: 10, padding: "1px 0" }}>
                <span style={{ minWidth: 110, color: "#9ab0c8", overflow: "hidden", whiteSpace: "nowrap", textOverflow: "ellipsis" }}>
                  {iconFor(p.good_name)} {labelFor(p.good_name)}
                </span>
                <span style={{ minWidth: 44, textAlign: "right", fontWeight: 600,
                  color: p.price > p.base_value * 1.3 ? "#e08080" : p.price < p.base_value * 0.77 ? "#7fd0a0" : "#c0d0e0" }}>
                  {(p.price / Math.max(1e-6, p.base_value)).toFixed(2)}×
                </span>
              </div>
            ))
          ) : <div style={emptyTxt}>no market here</div>}

          {/* Exports / Imports two-column */}
          <div style={{ display: "flex", gap: 8, marginTop: 6 }}>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={sectionHdr}>→ Exports ({hub.produces.length})</div>
              {hub.produces.length === 0 && <div style={emptyTxt}>nothing of note</div>}
              {hub.produces.slice(0, 14).map((p) => {
                const dests = (hub.exports_to ?? []).filter((e) => e.good_name === p.good_name);
                const active = selectedExport === p.good_name;
                return (
                  <div key={`p${p.good}`}>
                    <div onClick={() => setSelectedExport(active ? null : p.good_name)}
                      style={{ ...row, cursor: dests.length ? "pointer" : "default", background: active ? "#1a2c40" : "transparent", borderRadius: 3 }}>
                      <span style={{ minWidth: 14 }}>{iconFor(p.good_name)}</span>
                      <span style={{ flex: 1, color: active ? "#e8d8b0" : "#c0d0e0", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {labelFor(p.good_name)}
                      </span>
                      <span style={{ color: "#9ab0c8", fontSize: 9 }}>{p.grade}</span>
                      <span style={{ color: "#e0c060", fontSize: 10, minWidth: 30, textAlign: "right" }}>{p.price.toFixed(1)}×</span>
                    </div>
                    {active && (
                      <div style={{ padding: "1px 4px 3px 16px" }}>
                        {dests.length === 0 && <div style={emptyTxt}>consumed locally — not shipped</div>}
                        {dests.slice(0, 8).map((e) => (
                          <div key={e.chain} style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: "#9ab0c8" }}>
                            <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>→ {hubName(e.to_hub)}</span>
                            <span style={{ color: "#7fd0a0" }}>{Math.round(e.pct)}%</span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={sectionHdr}>← Imports ({hub.receives.length})</div>
              {hub.receives.length === 0 && <div style={emptyTxt}>self-sufficient</div>}
              {hub.receives.slice(0, 16).map((r) => {
                const active = selectedChain === r.chain;
                return (
                  <div key={`r${r.chain}-${r.good}`} onClick={() => setSelectedChain(active ? null : r.chain)}
                    style={{ ...row, cursor: "pointer", background: active ? "#1a2c40" : "transparent", borderRadius: 3 }}>
                    <span style={{ minWidth: 14 }}>{iconFor(r.good_name)}</span>
                    <span style={{ flex: 1, color: active ? "#e8d8b0" : "#c0d0e0", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {labelFor(r.good_name)}
                    </span>
                    <span style={{ color: "#7a90a8", fontSize: 8 }}>{hubName(r.from_hub).slice(0, 6)}</span>
                    <span style={{ color: "#ff9a6a", fontSize: 10, minWidth: 30, textAlign: "right" }}>{r.price.toFixed(1)}×</span>
                  </div>
                );
              })}
            </div>
          </div>
          {chain && (
            <div style={{ marginTop: 6, padding: "6px 8px", background: "#0b1622", borderRadius: 5, border: "1px solid #1e3550" }}>
              <div style={{ color: "#9ab0c8", fontSize: 10, marginBottom: 4 }}>
                {iconFor(chain.good_name)} {labelFor(chain.good_name)} — price along the road
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 4, fontSize: 11 }}>
                {chain.stops.map((s, i) => (
                  <span key={i} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                    {i > 0 && <span style={{ color: "#5a7090" }}>→</span>}
                    <span style={{ color: i === 0 ? "#80dc8c" : i === chain.stops.length - 1 ? "#ff9a6a" : "#e0c060" }}>
                      {hubName(s.hub)} <b>{s.price.toFixed(1)}×</b>
                    </span>
                  </span>
                ))}
              </div>
            </div>
          )}
          </>)}
        </>
      )}

      {/* ════════════ ESTATES & BUILDINGS ════════════ */}
      {tab === "estates" && (
        <>
          <div style={sectionHdr}>Estates &amp; manufactories</div>
          {(detail?.estates_here?.length ?? 0) === 0 && (
            <div style={emptyTxt}>
              {detail ? "No estates yet — wealthy houses & guilds build them over time." : "Begin the campaign (Step 11) to see this city's estates."}
            </div>
          )}
          {[...(detail?.estates_here ?? [])]
            .sort((a, b) => b.output - a.output)
            .map((e, i) => (
              <div key={i} style={{ display: "flex", alignItems: "baseline", gap: 6, fontSize: 10, padding: "2px 2px", borderBottom: "1px solid #131f2c" }}>
                <span style={{ fontSize: 13 }}>{ESTATE_EMOJI[e.kind] ?? "🏡"}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: "#cdbb88", fontWeight: 600 }}>
                    {ESTATE_LABEL[e.kind] ?? "Estate"} · {iconFor(e.good)} {labelFor(e.good)}
                    <span style={{ color: "#e0c060", fontSize: 9, marginLeft: 4 }} title="upgrade tier (owners invest to raise output)">
                      {"★".repeat(e.tier ?? 1)}<span style={{ color: "#3a4a5e" }}>{"★".repeat(Math.max(0, 5 - (e.tier ?? 1)))}</span>
                    </span>
                  </div>
                  <div style={{ color: "#7a90a8", fontSize: 9 }}>
                    owner: <span style={{ color: e.owner_is_guild ? "#7fd0c0" : "#e8dcc0" }}>{e.owner}</span> · tier {e.tier ?? 1}/5
                  </div>
                </div>
                <span style={{ color: "#7fd0a0", fontSize: 10 }}>▲ {fmt(e.output)}/day</span>
              </div>
            ))}

          {/* Buildings in the city itself (granary, warehouse, …) with effects */}
          <div style={{ ...sectionHdr, marginTop: 8 }}>Buildings</div>
          {(detail?.structures?.length ?? 0) === 0 ? (
            <div style={emptyTxt}>{detail ? "No civic buildings yet." : "—"}</div>
          ) : (
            detail!.structures!.map(([nm, eff], i) => (
              <div key={i} style={{ display: "flex", gap: 6, alignItems: "baseline", fontSize: 10, padding: "1px 2px" }}>
                <span style={{ fontSize: 12 }}>{STRUCT_EMOJI[nm] ?? "🏗️"}</span>
                <span style={{ color: "#cdbb88", fontWeight: 700, minWidth: 72 }}>{nm}</span>
                <span style={{ flex: 1 }} />
                <span style={{ color: "#7fbf9a" }}>{eff}</span>
              </div>
            ))
          )}
        </>
      )}

      {/* ════════════ DEPOTS (warehouses sited here + futures links) ════════════ */}
      {tab === "depots" && (() => {
        const here = depots.filter((w) => w.city === hub.name || w.city.includes(`(by ${hub.name})`));
        const inbound = lanes.filter((l) => l.b_name === hub.name);
        const outbound = lanes.filter((l) => l.a_name === hub.name);
        const cityList = (ls: FuturesLane[], pick: (l: FuturesLane) => string) =>
          Array.from(new Set(ls.map(pick))).slice(0, 8).join(", ") || "—";
        const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));
        const focusCity = () => { setFuturesFocus({ city: hub.name }); setOverlayVisible("futures", true); };
        return (
          <>
            <div style={sectionHdr}>Warehouses & estates here ({here.length})</div>
            {here.length === 0 && <div style={{ color: "#506080", fontSize: 11, padding: "4px 2px" }}>No house depots in this city yet.</div>}
            {here.map((w, i) => {
              const fill = w.capacity > 0 ? Math.min(1, w.used / w.capacity) : 0;
              return (
                <div key={i} style={{ padding: "3px 2px", borderBottom: "1px solid #131e2a" }}>
                  <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
                    <span style={{ width: 8, height: 8, borderRadius: 2, background: w.color, alignSelf: "center" }} />
                    <span style={{ color: "#e8d8b0", fontSize: 11, fontWeight: 600 }}>{w.owner}</span>
                    {w.is_guild && <span style={{ fontSize: 8, color: "#7fd0c0" }}>GUILD</span>}
                    <span style={{ flex: 1 }} />
                    {w.contracts > 0 && <span style={{ color: "#ffcf3f", fontSize: 9 }}>📜 {w.contracts}</span>}
                    <span style={{ color: "#6a86a6", fontSize: 9 }}>{w.kind === "warehouse" ? `T${w.tier}` : w.kind}</span>
                  </div>
                  {w.capacity > 0 && (
                    <div style={{ display: "flex", alignItems: "center", gap: 4, marginTop: 2 }}>
                      <div style={{ flex: 1, height: 4, background: "#0a1018", borderRadius: 3, overflow: "hidden" }}>
                        <div style={{ width: `${fill * 100}%`, height: "100%", background: fill > 0.85 ? "#e0a020" : "#5a9bd0" }} />
                      </div>
                      <span style={{ color: "#7a90a8", fontSize: 9 }}>{fmt(w.used)}/{fmt(w.capacity)}</span>
                    </div>
                  )}
                </div>
              );
            })}
            <div style={{ ...sectionHdr, marginTop: 8 }}>Futures supply links</div>
            <div style={{ fontSize: 10, color: "#9ab0c8", padding: "2px 2px" }}>
              <span style={{ color: "#7fd0a0" }}>Imports ←</span> {cityList(inbound, (l) => l.a_name)}
            </div>
            <div style={{ fontSize: 10, color: "#9ab0c8", padding: "2px 2px" }}>
              <span style={{ color: "#e0a060" }}>Exports →</span> {cityList(outbound, (l) => l.b_name)}
            </div>
            <div onClick={focusCity} style={{ marginTop: 6, cursor: "pointer", color: "#ffcf3f", fontSize: 10 }}
              title="Highlight this city's futures network on the map">
              📜 Show this city's futures network on the map
            </div>
          </>
        );
      })()}

      {/* ════════════ PEOPLE (society + history) ════════════ */}
      {tab === "people" && (
        <>
          <div style={sectionHdr}>Society</div>
          <div style={{ display: "flex", gap: 4 }}>
            <ClassTile label="Nobility" value={hub.nobility ?? 0} level={hub.elite_level ?? 0} color="#e0c060" />
            <ClassTile label="Merchants" value={hub.merchants ?? 0} level={hub.merchant_level ?? 0} color="#5fc8a8" />
            <ClassTile label="Commoners" value={hub.commoners ?? 0} level={1} color="#8aa0c0" />
          </div>

          {/* How goods reach this city — by ship (sea) vs caravan (land). Every
              shipment, house or guild, is tagged by how it travelled. */}
          {(() => {
            const sea = detail?.in_by_sea ?? 0;
            const land = detail?.in_by_land ?? 0;
            const tot = sea + land;
            const seaPct = tot > 1e-4 ? Math.round((sea / tot) * 100) : 0;
            return (
              <>
                <div style={{ ...sectionHdr, marginTop: 6 }}>How goods arrive</div>
                {tot < 1e-4 ? (
                  <div style={{ color: "#7a90a8", fontSize: 10 }}>No trade arriving yet.</div>
                ) : (
                  <>
                    <div style={{ display: "flex", height: 8, borderRadius: 3, overflow: "hidden", background: "#0a1018" }}>
                      <div style={{ width: `${seaPct}%`, background: "#4a6a8a" }} title={`Ships (sea): ${seaPct}%`} />
                      <div style={{ width: `${100 - seaPct}%`, background: "#b5894a" }} title={`Caravans (land): ${100 - seaPct}%`} />
                    </div>
                    <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: "#9ab0c8", marginTop: 2 }}>
                      <span>🚢 ships {seaPct}%</span>
                      <span>{100 - seaPct}% caravans 🐫</span>
                    </div>
                  </>
                )}
              </>
            );
          })()}

          {/* Merchant fleets at this settlement — resident houses' real ships/boats/
              caravans, plus an estimate of the independent local-merchant and guild
              vessels (scaled from their trade share at the same throughput-per-vessel
              as the houses). Answers "how many ships & caravans work this port". */}
          {(() => {
            const hs = detail?.houses ?? [];
            const hSea = hs.reduce((s, h) => s + (h.fleet_sea ?? 0), 0);
            const hRiver = hs.reduce((s, h) => s + (h.fleet_river ?? 0), 0);
            const hCar = hs.reduce((s, h) => s + (h.fleet_caravan ?? 0), 0);
            const hVessels = hSea + hRiver + hCar;
            const houseVol = hs.reduce((s, h) => s + Math.max(0, h.volume ?? h.wealth), 0);
            const mlev = hub.merchant_level ?? 0.3;
            const independent = houseVol * (0.25 + 0.7 * mlev) + 0.5;
            const guildVol = independent * mlev;
            const localVol = independent * (1 - mlev);
            // Vessels per unit of trade volume, inferred from the houses (fallback: a
            // light rate off the merchant population when no house fleet exists yet).
            const perVol = houseVol > 1e-4 && hVessels > 0 ? hVessels / houseVol : 0;
            const estLocal = perVol > 0 ? localVol * perVol : (hub.merchants ?? 0) * 0.0008 * (1 - mlev);
            const estGuild = perVol > 0 ? guildVol * perVol : (hub.merchants ?? 0) * 0.0008 * mlev;
            const seaPctOfHub = (() => {
              const sea = detail?.in_by_sea ?? 0, land = detail?.in_by_land ?? 0;
              return sea + land > 1e-4 ? sea / (sea + land) : (hub.coastal ? 0.5 : 0);
            })();
            const splitVessels = (n: number) => {
              const ships = Math.round(n * seaPctOfHub);
              const land = Math.max(0, Math.round(n) - ships);
              return { ships, land };
            };
            const loc = splitVessels(estLocal), gld = splitVessels(estGuild);
            return (
              <>
                <div style={{ ...sectionHdr, marginTop: 6 }}>Ships &amp; caravans working this port</div>
                <div style={{ fontSize: 10, color: "#9ab0c8", lineHeight: 1.6 }}>
                  <div>
                    <span style={{ color: "#cdbb88", fontWeight: 600 }}>Houses</span>
                    {hVessels > 0
                      ? <> · 🚢 {hSea} · 🛶 {hRiver} · 🐫 {hCar}</>
                      : <span style={{ color: "#6a86a6" }}> · no resident house fleet</span>}
                  </div>
                  <div style={{ color: LOCAL_COLOR }}>
                    <span style={{ fontWeight: 600 }}>Local merchants</span>
                    <span style={{ color: "#7a90a8" }}> ≈ 🚢 {loc.ships} · 🐫 {loc.land}</span>
                  </div>
                  <div style={{ color: GUILD_COLOR }}>
                    <span style={{ fontWeight: 600 }}>Merchant guilds</span>
                    <span style={{ color: "#7a90a8" }}> ≈ 🚢 {gld.ships} · 🐫 {gld.land}</span>
                  </div>
                  <div style={{ color: "#56708e", fontSize: 8 }}>house counts exact · local/guild estimated from trade share</div>
                </div>
              </>
            );
          })()}

          {/* Foreign offices hosted here — houses/guilds based elsewhere who have
              opened a counting-house in this city (origin, % of trade, goods). */}
          {detail && (detail.offices_here?.length ?? 0) > 0 && (
            <>
              <div style={{ ...sectionHdr, marginTop: 6 }}>Foreign offices here ({detail.offices_here!.length})</div>
              {[...detail.offices_here!]
                .sort((a, b) => b.throughput_pct - a.throughput_pct)
                .map((o, i) => (
                  <div key={o.holder + i} style={{ marginBottom: 4 }}>
                    <div style={{ display: "flex", alignItems: "baseline", gap: 5, fontSize: 10 }}>
                      <span style={{ width: 8, height: 8, borderRadius: 2, background: o.color, flex: "0 0 auto", alignSelf: "center" }} />
                      <span style={{ color: "#e8dcc0", fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{o.holder}</span>
                      {o.is_guild && <span style={{ fontSize: 8, color: "#7fd0c0" }}>GUILD</span>}
                      <span style={{ flex: 1 }} />
                      <span style={{ color: "#9ab0c8", fontSize: 9 }} title="share of this city's live trade throughput">
                        {o.throughput_pct >= 0.5 ? `${Math.round(o.throughput_pct)}%` : "—"}
                      </span>
                    </div>
                    <div style={{ color: "#7a90a8", fontSize: 9, paddingLeft: 13 }}>
                      from {o.origin || "—"}
                      {o.goods.length > 0 && <> · {o.goods.slice(0, 5).map((g) => `${iconFor(g)} ${labelFor(g)}`).join(", ")}</>}
                    </div>
                  </div>
                ))}
            </>
          )}

          {/* Who controls the trade — ALWAYS shown. With no resident houses the
              circle is just local merchants + guilds. */}
          {(() => {
            const hs = detail?.houses ?? [];
            const total = Math.max(1e-6, hs.reduce((s, h) => s + Math.max(0, h.wealth), 0));
            // Local merchants & guilds always move some trade. The independent
            // (non-house) volume scales with the merchant class (merchant_level
            // 0..1); a base term keeps it present even with no houses. It splits
            // into organised GUILDS (∝ merchant_level) and unaffiliated LOCALS.
            const mlev = hub.merchant_level ?? 0.3;
            const houseVol = hs.reduce((s, h) => s + Math.max(0, h.volume ?? h.wealth), 0);
            const independent = houseVol * (0.25 + 0.7 * mlev) + 0.5;
            const guildVolume = independent * mlev;
            const localVolume = independent * (1 - mlev);
            return (
              <>
                <div style={{ ...sectionHdr, marginTop: 6 }}>Who controls the trade (houses · merchants · guilds)</div>
                <HouseSharePie houses={hs} localVolume={localVolume} guildVolume={guildVolume} merchants={hub.merchants ?? 0} />
                {hs.map((h, i) => (
                  <div key={h.name + i} style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 3 }}>
                    <CoatOfArms name={h.name} size={20} guild={h.is_guild} />
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
                        <span style={{ color: "#e8dcc0", fontSize: 11, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{h.name}</span>
                        <span style={{ flex: 1 }} />
                        <span style={{ color: "#c9a227", fontSize: 10 }}>{Math.round((Math.max(0, h.wealth) / total) * 100)}%</span>
                      </div>
                      <div style={{ height: 3, background: "#0a1018", borderRadius: 2, overflow: "hidden" }}>
                        <div style={{ width: `${(Math.max(0, h.wealth) / total) * 100}%`, height: "100%", background: "#c9a227" }} />
                      </div>
                      {h.specialties.length > 0 && (
                        <div style={{ color: "#9ab0c8", fontSize: 9 }}>
                          {h.head_name} · {h.specialties.join(", ")}
                          {h.monopolies.length > 0 && (
                            <span style={{ color: "#e0b060" }}> ({h.monopolies.map(([g, s]) => `${g} ${Math.round(s * 100)}%`).join(", ")})</span>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                ))}
              </>
            );
          })()}

          {hub.luxuries && hub.luxuries.length > 0 && (
            <>
              <div style={{ ...sectionHdr, marginTop: 6 }}>Luxury market (demand vs. arrivals · price)</div>
              {(() => {
                const lux = hub.luxuries!;
                const maxD = Math.max(1e-6, ...lux.map((l) => Math.max(l.demand, l.received)));
                return lux.slice(0, 8).map((l) => {
                  const klass = desireClass(l.good_name);
                  return (
                    <div key={l.good} style={{ marginBottom: 3 }}>
                      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: "#c0d0e0" }}>
                        <span>{iconFor(l.good_name)} {labelFor(l.good_name)}
                          {klass && <span style={{ color: "#7a90a8" }}> · {klass} crave it</span>}</span>
                        <span style={{ color: l.received < l.demand * 0.6 ? "#ff9a6a" : "#9ab0c8" }}>
                          {l.received < l.demand * 0.6 ? "shortage " : ""}{l.price.toFixed(1)}×
                        </span>
                      </div>
                      <div style={{ position: "relative", height: 6, background: "#13202e", borderRadius: 2 }}>
                        <div style={{ position: "absolute", inset: 0, width: `${(l.demand / maxD) * 100}%`, background: "#2a4060", borderRadius: 2 }} title={`demand ${l.demand.toFixed(2)}`} />
                        <div style={{ position: "absolute", inset: 0, width: `${(l.received / maxD) * 100}%`, background: "#5fc8a8", borderRadius: 2 }} title={`received ${l.received.toFixed(2)}`} />
                      </div>
                    </div>
                  );
                });
              })()}
            </>
          )}

          {hub.shortages && hub.shortages.length > 0 && (
            <>
              <div style={{ ...sectionHdr, marginTop: 6 }}>Shortages — why goods don't arrive</div>
              {hub.shortages.map((s) => (
                <div key={s.good} style={{ ...row, fontSize: 10 }}>
                  <span style={{ minWidth: 14 }}>{iconFor(s.good_name)}</span>
                  <span style={{ color: "#e0b090", minWidth: 64 }}>{labelFor(s.good_name)}</span>
                  <span style={{ flex: 1, color: "#9ab0c8", fontStyle: "italic" }}>{shortageReason(s.reason)}</span>
                  <span style={{ color: "#ff9a6a", fontSize: 9 }}>{Math.round(s.severity * 100)}% short</span>
                </div>
              ))}
            </>
          )}

          {(outCargo.length > 0 || inCargo.length > 0) && (
            <>
              <div style={{ ...sectionHdr, marginTop: 6 }}>Cargo both ways</div>
              <div style={{ display: "flex", gap: 8 }}>
                {([["→ outbound", outCargo], ["← inbound", inCargo]] as const).map(([title, col]) => (
                  <div key={title} style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ color: "#6a86a6", fontSize: 9, marginBottom: 3 }}>{title} · {col.length}</div>
                    {col.length === 0 && <div style={emptyTxt}>—</div>}
                    {col.slice(0, 10).map((g) => (
                      <div key={g.good_name} style={{ marginBottom: 2 }}>
                        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, color: "#c0d0e0" }}>
                          <span>{iconFor(g.good_name)} {labelFor(g.good_name)}</span>
                          <span style={{ color: "#9ab0c8" }}>{g.value.toFixed(1)}</span>
                        </div>
                        <div style={{ height: 4, background: "#13202e", borderRadius: 2 }}>
                          <div style={{ height: 4, width: `${(g.value / cargoMax) * 100}%`, background: "#5fc8a8", borderRadius: 2 }} />
                        </div>
                      </div>
                    ))}
                  </div>
                ))}
              </div>
            </>
          )}

          {economy.class_stats && economy.class_stats.length > 0 && (
            <>
              <div style={{ ...sectionHdr, marginTop: 6 }}>World trade nodes</div>
              <div style={{ display: "flex", gap: 4 }}>
                {economy.class_stats.map((cs) => (
                  <div key={cs.label} style={{ ...statTile, flex: 1 }}>
                    <div style={{ color: cs.label === "emporiums" ? "#e63030" : cs.label === "outposts" ? "#aaaaaa" : "#5fc8d8", fontSize: 12, fontWeight: 700 }}>{cs.count}</div>
                    <div style={{ color: "#6a86a6", fontSize: 8, textTransform: "capitalize" }}>{cs.label}</div>
                    <div style={{ color: "#7a90a8", fontSize: 7 }}>{cs.population.toLocaleString()} ppl</div>
                  </div>
                ))}
              </div>
            </>
          )}

          <div style={{ ...sectionHdr, marginTop: 6 }}>Wealthiest hubs</div>
          {wealthSorted.slice(0, 5).map((h, i) => (
            <div key={h.id} onClick={() => setSelectedHub(h.id)}
              style={{ ...row, cursor: "pointer", background: h.id === hub.id ? "#1a2c40" : "transparent" }}>
              <span style={{ color: "#6a86a6", minWidth: 16 }}>#{i + 1}</span>
              <span style={{ flex: 1, color: h.id === hub.id ? "#e8d8b0" : "#c0d0e0", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {h.id === topHub?.id ? "🟨 " : h.emporium ? "🔺 " : ""}{h.name}
              </span>
              <span style={{ color: "#9ab0c8", fontSize: 9 }}>{h.population.toLocaleString()}</span>
              <span style={{ color: "#7fd0a0", fontSize: 10, minWidth: 32, textAlign: "right" }}>{Math.round(h.wealth * 100)}%</span>
            </div>
          ))}
        </>
      )}

      {/* ════════════ PEOPLE — history charts + chronicle ════════════ */}
      {tab === "people" && (
        detail ? (
          <>
            {detail.history.length > 1 ? (
              <>
                <SeriesStat label="Population" values={detail.history.map((s) => s.population)}
                  color="#5fc8a8" fmt={(v) => Math.round(v).toLocaleString()} />
                <Sparkline values={detail.history.map((s) => s.population)} color="#5fc8a8" />
                <SeriesStat label="Wealth" values={detail.history.map((s) => s.wealth)}
                  color="#e0c060" fmt={(v) => v.toFixed(2)} mt />
                <Sparkline values={detail.history.map((s) => s.wealth)} color="#e0c060" />
                <SeriesStat label="Mood" values={detail.history.map((s) => s.mood)}
                  color="#9ab0c8" fmt={(v) => `${Math.round(v * 100)}%`} mt />
                <Sparkline values={detail.history.map((s) => s.mood)} color="#9ab0c8" min={0} max={1} />

                <div style={{ ...sectionHdr, marginTop: 8 }}>People lacking goods (% of demand unmet)</div>
                <MultiSeriesChart
                  min={0} max={1} fmt={(v) => `${Math.round(v * 100)}%`}
                  series={[
                    { label: "Basic", color: "#ff8a6a", values: detail.history.map((s) => s.lack_basic ?? 0) },
                    { label: "Comfort", color: "#e0c060", values: detail.history.map((s) => s.lack_comfort ?? 0) },
                    { label: "Luxury", color: "#9ab0c8", values: detail.history.map((s) => s.lack_luxury ?? 0) },
                  ]}
                />

                <div style={{ ...sectionHdr, marginTop: 8 }}>Merchant population by class</div>
                <MultiSeriesChart
                  min={0} fmt={(v) => Math.round(v).toLocaleString()}
                  series={[
                    { label: "🧺 Local", color: "#7fd0a0", values: detail.history.map((s) => s.pop_local ?? 0) },
                    { label: "🏛 Houses", color: "#e0c060", values: detail.history.map((s) => s.pop_house ?? 0) },
                    { label: "⚖ Guilds", color: "#8aa0c0", values: detail.history.map((s) => s.pop_guild ?? 0) },
                  ]}
                />
              </>
            ) : (
              <div style={emptyTxt}>Advance the campaign a few weeks to chart this city's history.</div>
            )}
            <div style={{ ...sectionHdr, marginTop: 6 }}>Chronicle</div>
            {detail.events.length === 0 && <div style={emptyTxt}>No notable events yet.</div>}
            {[...detail.events].reverse().slice(0, 30).map((e, i) => (
              <div key={i} style={{ display: "flex", gap: 6, fontSize: 9, padding: "1px 0" }}>
                <span style={{ color: "#56708e", minWidth: 44 }}>Yr {Math.floor(e.tick / 365)}</span>
                <span style={{
                  flex: 1,
                  color: e.kind === "starvation" ? "#ff7a6a" : e.kind === "estate" ? "#7fd0a0"
                    : e.kind === "succession" ? "#c0a0e0" : "#b8c8da",
                }}>{e.text}</span>
              </div>
            ))}
          </>
        ) : (
          <div style={emptyTxt}>Begin the campaign (Step 11) — this city's history and charts fill in as time passes.</div>
        )
      )}
    </div>
  );
}

/** Population mood: a headline face + the three driver bars. */
/** DLC 3.5 · the city's treasury books — taxes in, spending out, plus coin & war.
 *  Shows the last completed year (`prev`) when available, else the running year. */
function CityFinances({ detail }: { detail: HubDetail }) {
  const f = detail.finance?.prev ?? detail.finance ?? null;
  const Line = ({ label, amt, neg }: { label: string; amt: number; neg?: boolean }) => (
    amt > 0.01 ? (
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9.5 }}>
        <span style={{ color: "#9ab0c8" }}>{label}</span>
        <span style={{ color: neg ? "#e0a0a0" : "#7fcf8f" }}>{neg ? "−" : "+"}{fmtN(amt)}</span>
      </div>
    ) : null
  );
  return (
    <>
      <div style={{ ...sectionHdr, marginTop: 6 }}>City finances</div>
      <div style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 10, flexWrap: "wrap" }}>
        <span style={{ color: "#d8c878", fontWeight: 700 }}>🏛 Treasury {fmtN(detail.treasury ?? 0)}</span>
        {detail.coin_name ? (
          <span style={{ display: "inline-flex", alignItems: "center", gap: 3, color: "#d8c878" }}
            title={`${detail.coin_name} · value ${(detail.coin_value ?? 0).toFixed(2)}×`}>
            <CoinIcon issuer={detail.name} value={detail.coin_value} size={15} />
            {detail.coin_name} {(detail.coin_value ?? 0).toFixed(2)}×
          </span>
        ) : null}
        {detail.war_with ? <span style={{ color: "#e88" }}>⚔ war: {detail.war_with}</span> : null}
      </div>
      {f && (f.year > 0 || (detail.treasury ?? 0) > 0) && (
        <div style={{ display: "flex", gap: 12, marginTop: 4 }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ color: "#6a86a6", fontSize: 8.5, textTransform: "uppercase" }}>Income · yr {f.year}</div>
            <Line label="Trade tariffs" amt={f.tax_trade} />
            <Line label="Estate tax" amt={f.tax_estate} />
            <Line label="Manufacturing tax" amt={f.tax_manufacture} />
            <Line label="Wealth tax" amt={f.tax_wealth} />
            <Line label="Seigniorage" amt={f.seigniorage} />
            <Line label="War levy" amt={f.war_levy} />
            <Line label="Reparations" amt={f.reparations_in} />
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ color: "#6a86a6", fontSize: 8.5, textTransform: "uppercase" }}>Spending</div>
            <Line label="To the people" amt={f.spent_civic} neg />
            <Line label="War effort" amt={f.spent_war} neg />
            <Line label="Public works" amt={f.spent_works} neg />
            <Line label="Reparations paid" amt={f.reparations_out} neg />
          </div>
        </div>
      )}
    </>
  );
}

function MoodCard({ detail }: { detail: HubDetail }) {
  const mood = detail.mood;
  const face = mood > 0.75 ? "😄 Joyful" : mood > 0.58 ? "🙂 Content" : mood > 0.42 ? "😐 Uneasy"
    : mood > 0.25 ? "😟 Discontent" : "😠 Rebellious";
  const moodColor = mood > 0.58 ? "#7fd0a0" : mood > 0.42 ? "#e0c060" : "#ff8a6a";
  const stab = detail.sent_stability;
  const stabNote = stab < 0.5 ? " (recent disasters)" : "";
  return (
    <div style={{ margin: "2px 0 4px" }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginBottom: 3 }}>
        <span style={{ color: moodColor, fontSize: 12, fontWeight: 700 }}>{face}</span>
        <span style={{ color: "#7a90a8", fontSize: 10 }}>{Math.round(mood * 100)}%</span>
        {detail.starving > 0.4 && <span style={{ color: "#ff6a4a", fontSize: 9 }}>· starving</span>}
      </div>
      <DriverBar label="Food" frac={detail.sent_food} color="#5fc8a8" />
      <DriverBar label="Prosperity" frac={detail.sent_prosperity} color="#e0c060" />
      <DriverBar label={`Stability${stabNote}`} frac={detail.sent_stability} color="#8aa0c0" />
    </div>
  );
}

function DriverBar({ label, frac, color }: { label: string; frac: number; color: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6, margin: "1px 0" }}>
      <span style={{ color: "#6a86a6", fontSize: 9, minWidth: 96 }}>{label}</span>
      <div style={{ flex: 1, height: 5, background: "#13202e", borderRadius: 2 }}>
        <div style={{ height: 5, width: `${Math.max(2, Math.min(100, frac * 100))}%`, background: color, borderRadius: 2 }} />
      </div>
    </div>
  );
}

/** History header row: label + current value + change this month + since start. */
function SeriesStat({ label, values, color, fmt, mt }: {
  label: string; values: number[]; color: string; fmt: (v: number) => string; mt?: boolean;
}) {
  const n = values.length;
  const cur = values[n - 1];
  const prev = n >= 2 ? values[n - 2] : cur;     // last monthly sample
  const first = values[0];
  const dMonth = cur - prev;
  const dAll = cur - first;
  const pct = (d: number, base: number) => (Math.abs(base) < 1e-6 ? 0 : (d / Math.abs(base)) * 100);
  const sign = (d: number) => (d > 0 ? "+" : d < 0 ? "−" : "±");
  const col = (d: number) => (d > 0 ? "#7fd0a0" : d < 0 ? "#ff8a6a" : "#6a86a6");
  return (
    <div style={{ display: "flex", alignItems: "baseline", gap: 8, marginTop: mt ? 6 : 0 }}>
      <span style={{ ...sectionHdr, margin: 0, flex: "0 0 auto" }}>{label}</span>
      <span style={{ color, fontSize: 13, fontWeight: 700 }}>{fmt(cur)}</span>
      <span style={{ flex: 1 }} />
      <span style={{ color: col(dMonth), fontSize: 9 }}>
        {sign(dMonth)}{fmt(Math.abs(dMonth))}/mo
      </span>
      <span style={{ color: col(dAll), fontSize: 9 }}>
        {sign(dAll)}{Math.abs(pct(dAll, first)).toFixed(0)}% all-time
      </span>
    </div>
  );
}

/** Tiny SVG sparkline for the History tab. */
function Sparkline({ values, color, min, max, baseline }: {
  values: number[]; color: string; min?: number; max?: number; baseline?: number;
}) {
  if (values.length < 2) return <div style={emptyTxt}>—</div>;
  const lo = min ?? Math.min(...values);
  const hi = max ?? Math.max(...values);
  const span = Math.max(1e-6, hi - lo);
  const W = 320, H = 44;
  const pts = values.map((v, i) => {
    const x = (i / (values.length - 1)) * W;
    const y = H - ((v - lo) / span) * (H - 4) - 2;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  const baseY = baseline !== undefined ? H - ((baseline - lo) / span) * (H - 4) - 2 : null;
  return (
    <svg width="100%" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" style={{ display: "block", background: "#0b1622", borderRadius: 3 }}>
      {baseY !== null && baseY >= 0 && baseY <= H && (
        <line x1={0} y1={baseY} x2={W} y2={baseY} stroke="#2a3a50" strokeWidth={1} strokeDasharray="3 3" />
      )}
      <polyline points={pts} fill="none" stroke={color} strokeWidth={1.6} />
    </svg>
  );
}

/** Several series overlaid on one tiny chart, with a legend showing each series'
 *  current value. Used for the per-tier shortage and the merchant-class charts. */
function MultiSeriesChart({ series, min, max, fmt }: {
  series: { label: string; color: string; values: number[] }[];
  min?: number; max?: number; fmt: (v: number) => string;
}) {
  const len = Math.max(0, ...series.map((s) => s.values.length));
  if (len < 2) return <div style={emptyTxt}>—</div>;
  const all = series.flatMap((s) => s.values);
  const lo = min ?? Math.min(...all);
  const hi = max ?? Math.max(...all, lo + 1e-6);
  const span = Math.max(1e-6, hi - lo);
  const W = 320, H = 48;
  return (
    <>
      <svg width="100%" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none"
        style={{ display: "block", background: "#0b1622", borderRadius: 3 }}>
        {series.map((s, si) => {
          const pts = s.values.map((v, i) => {
            const x = (i / (s.values.length - 1)) * W;
            const y = H - ((v - lo) / span) * (H - 4) - 2;
            return `${x.toFixed(1)},${y.toFixed(1)}`;
          }).join(" ");
          return <polyline key={si} points={pts} fill="none" stroke={s.color} strokeWidth={1.6} />;
        })}
      </svg>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 2 }}>
        {series.map((s, si) => (
          <span key={si} style={{ display: "inline-flex", alignItems: "center", gap: 3, fontSize: 9, color: "#9ab0c8" }}>
            <span style={{ width: 8, height: 8, borderRadius: 2, background: s.color }} />
            {s.label} <span style={{ color: s.color, fontWeight: 700 }}>{fmt(s.values[s.values.length - 1] ?? 0)}</span>
          </span>
        ))}
      </div>
    </>
  );
}

/** A fuller character sketch: climate & terrain, what the people are known for and
 *  their social character, population/scale, and a touch of founding history. */
function peopleSummary(hub: EconHub, labelFor: (id: string) => string, topName?: string, isTop?: boolean): string {
  const known = hub.monopolies && hub.monopolies.length > 0
    ? hub.monopolies.map(labelFor)
    : hub.produces.slice(0, 2).map((p) => labelFor(p.good_name));
  const wants = hub.receives.slice(0, 2).map((r) => labelFor(r.good_name));
  const { clim, analogue } = climatePhrase(hub.koppen ?? 0, hub.elevation ?? 0, !!hub.coastal);

  const society = hub.emporium ? "a cosmopolitan merchant republic, its quays thronged with foreign tongues"
    : hub.coastal && hub.wealth > 0.4 ? "a proud guild port of shipwrights and factors"
    : hub.wealth > 0.6 ? "a prosperous burgher city"
    : (hub.koppen === 21 || hub.koppen === 16 || hub.koppen === 17) ? "a hardy frontier community"
    : hub.wealth > 0.3 ? "an industrious market town" : "a modest farming community";

  const founders = [
    "founded where an old road forded the river",
    "grown from a sheltered anchorage",
    "raised around a hill-fort and its market",
    "settled by traders drawn to its ore and springs",
    "begun as a temple town and pilgrim halt",
    "planted as a colony at the edge of the known world",
  ];
  const founding = founders[hub.id % founders.length];

  const eliteLvl = hub.elite_level ?? 0;
  const merchLvl = hub.merchant_level ?? 0;
  const eliteWord = eliteLvl > 0.6 ? "a broad and gilded patrician class"
    : eliteLvl > 0.3 ? "a comfortable upper class" : "few of great wealth";
  const merchWord = merchLvl > 0.6 ? "a teeming merchant quarter"
    : merchLvl > 0.3 ? "an active body of traders" : "only a handful of traders";

  let s = `${clim.charAt(0).toUpperCase()}${clim.slice(1)} of roughly ${hub.population.toLocaleString()} souls — ${society}, ${founding}.`;
  s += ` Its climate recalls ${analogue}.`;
  s += ` Home to ${eliteWord} and ${merchWord}`;
  if (hub.top_export) s += `, who grow richest on ${labelFor(hub.top_export)}`;
  s += ".";
  if (known.length > 0) s += ` Renowned for its ${known.join(" and ")}.`;
  if (wants.length > 0) s += ` Its merchants hunger for ${wants.join(" and ")} from afar.`;
  if (isTop && hub.nearest_ref) s += ` The world's pre-eminent entrepôt — a rival to ${hub.nearest_ref} of old.`;
  else if (topName) s += ` In trade it looks up to ${topName}, the realm's greatest market.`;
  return s;
}

function ClassTile({ label, value, level, color }: { label: string; value: number; level: number; color: string }) {
  const lvlWord = level > 0.6 ? "high" : level > 0.3 ? "moderate" : "low";
  return (
    <div style={{ ...statTile, flex: 1 }}>
      <div style={{ color, fontSize: 12, fontWeight: 700 }}>{value.toLocaleString()}</div>
      <div style={{ color: "#6a86a6", fontSize: 8 }}>{label}</div>
      <div style={{ height: 3, background: "#13202e", borderRadius: 2, marginTop: 2 }}>
        <div style={{ height: 3, width: `${Math.min(100, level * 100)}%`, background: color, borderRadius: 2 }} title={`${lvlWord} share`} />
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div style={statTile}>
      <div style={{ color: "#d8e4f0", fontSize: 12, fontWeight: 700 }}>{value}</div>
      <div style={{ color: "#6a86a6", fontSize: 8 }}>{label}</div>
    </div>
  );
}

/** Explained currency card: for each emergent currency good show WHY it became
 *  money — liquidity, value density and price stability — plus live exchange
 *  ratios from the grain-equivalent prices (1 silver ≈ 12 grain ≈ 3 salt). */
function CurrencyCard({ currencies, iconFor, labelFor }: {
  currencies: HubCurrency[];
  iconFor: (id: string) => string;
  labelFor: (id: string) => string;
}) {
  const maxLiq = Math.max(1, ...currencies.map((c) => c.liquidity));
  const maxVal = Math.max(1e-6, ...currencies.map((c) => c.value));
  const primary = currencies[0];
  const ratios: string[] = [];
  if (primary && primary.price > 1e-6) {
    ratios.push(`${primary.price.toFixed(primary.price >= 10 ? 0 : 1)} grain`);
    for (const c of currencies.slice(1)) {
      if (c.price > 1e-6) ratios.push(`${(primary.price / c.price).toFixed(1)} ${labelFor(c.name)}`);
    }
  }
  return (
    <div style={currencyBox}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: 3 }}>
        <span style={{ color: "#e0c060", fontSize: 10, fontWeight: 700 }}>💰 Coin of the realm</span>
        <span style={{ color: "#56708e", fontSize: 8 }}>why these goods are money</span>
      </div>
      {currencies.map((c, i) => (
        <div key={c.good} style={{ marginBottom: 4 }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 5, fontSize: 10 }}>
            <span style={{ color: "#e8d8b0", fontWeight: 600 }}>{iconFor(c.name)} {labelFor(c.name)}</span>
            <span style={{ color: "#7a90a8", fontSize: 8 }}>{i === 0 ? "primary money" : "everyday change"}</span>
            <span style={{ flex: 1 }} />
            <span style={{ color: "#9ab0c8", fontSize: 9 }}>{c.price.toFixed(1)} grain</span>
          </div>
          <div style={{ display: "flex", gap: 4, marginTop: 2 }}>
            <CurBar label="Liquidity" frac={c.liquidity / maxLiq} color="#5fc8a8" hint={`${c.liquidity.toFixed(0)} trade partners`} />
            <CurBar label="Value" frac={c.value / maxVal} color="#e0c060" hint={`base value ${c.value.toFixed(1)}`} />
            <CurBar label="Stability" frac={c.stability} color="#8aa0c0" hint={`${Math.round(c.stability * 100)}% steady price`} />
          </div>
        </div>
      ))}
      {ratios.length > 0 && (
        <div style={{ color: "#a8bcd4", fontSize: 9, marginTop: 1, borderTop: "1px solid #1e2e42", paddingTop: 3 }}>
          <span style={{ color: "#6a86a6" }}>Exchange: </span>
          1 {labelFor(primary.name)} ≈ {ratios.join(" ≈ ")}
        </div>
      )}
    </div>
  );
}

function CurBar({ label, frac, color, hint }: { label: string; frac: number; color: string; hint: string }) {
  return (
    <div style={{ flex: 1, minWidth: 0 }} title={hint}>
      <div style={{ color: "#6a86a6", fontSize: 7 }}>{label}</div>
      <div style={{ height: 4, background: "#13202e", borderRadius: 2 }}>
        <div style={{ height: 4, width: `${Math.max(4, Math.min(100, frac * 100))}%`, background: color, borderRadius: 2 }} />
      </div>
    </div>
  );
}

const fmtN = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));

/** One shipment row in the Market flow (arrivals / departures), tagged with its
 *  owner (house/guild/local), origin or destination, carrier, good, amount and
 *  price — ranked by value upstream. A round-trip return leg is marked ↩. */
function ShipRow({ s, side, icon, label, faded }: {
  s: import("../types").ShipmentRow; side: "in" | "out";
  icon: (id: string) => string; label: (id: string) => string; faded?: boolean;
}) {
  const carrier = s.sea ? "🚢" : "🐫";
  const arrow = <span style={{ color: "#5a7090" }}>─▶</span>;
  return (
    <div style={{ fontSize: 8.5, marginBottom: 2, lineHeight: 1.25, opacity: faded ? 0.6 : 1 }}
      title={`${s.owner}${s.is_guild ? " (guild)" : ""} · ${s.other} · ${label(s.good)} ${fmtN(s.amount)} · value ${fmtN(s.value)}`}>
      <div style={{ display: "flex", alignItems: "center", gap: 3 }}>
        {side === "out" && arrow}
        <span style={{ width: 7, height: 7, borderRadius: 2, background: s.color, flex: "0 0 auto" }} />
        <span style={{ flex: 1, color: s.is_guild ? "#9ab0c8" : "#cfe0f4", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {s.returning_home ? "↩ " : ""}{s.owner}
        </span>
        {side === "in" && arrow}
      </div>
      <div style={{ display: "flex", gap: 3, color: "#9ab0c8" }}>
        <span>{carrier}</span>
        <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 52 }}>{s.other}</span>
        <span style={{ flex: 1, textAlign: "right", color: "#c0d0e0" }}>{icon(s.good)} {fmtN(s.amount)}</span>
        <span style={{ color: "#e0c060" }}>{s.price.toFixed(1)}×</span>
      </div>
    </div>
  );
}

/** Two-line price spark: local price (solid gold) vs the world average (dashed
 *  slate), sharing a y-scale, with the 1.0× world-standard line dotted. */
function DualSpark({ local, world }: { local: number[]; world: number[] }) {
  const all = [...local, ...world, 1];
  const lo = Math.min(...all), hi = Math.max(...all);
  const span = Math.max(1e-6, hi - lo);
  const W = 220, H = 30;
  const path = (arr: number[]) => arr.map((v, i) =>
    `${((i / Math.max(1, arr.length - 1)) * W).toFixed(1)},${(H - ((v - lo) / span) * (H - 4) - 2).toFixed(1)}`).join(" ");
  const oneY = H - ((1 - lo) / span) * (H - 4) - 2;
  return (
    <svg width="100%" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" style={{ display: "block", background: "#0b1622", borderRadius: 3 }}>
      <line x1={0} y1={oneY} x2={W} y2={oneY} stroke="#2a3a50" strokeWidth={0.5} strokeDasharray="2 2" />
      <polyline points={path(world)} fill="none" stroke="#8aa0c0" strokeWidth={1} strokeDasharray="3 2" />
      <polyline points={path(local)} fill="none" stroke="#e0c060" strokeWidth={1.4} />
    </svg>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 12, right: 12, width: 360, maxHeight: "90vh", overflowY: "auto",
  background: "rgba(12,18,26,0.97)", border: "1px solid #24364e", borderRadius: 8,
  padding: "10px 12px", zIndex: 110, boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
const statGrid: React.CSSProperties = {
  display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 4, marginTop: 2,
};
const statTile: React.CSSProperties = {
  background: "#0e1925", border: "1px solid #1c2c40", borderRadius: 4, padding: "3px 5px", textAlign: "center",
};
const blurbBox: React.CSSProperties = {
  marginTop: 6, padding: "5px 7px", background: "#0b1622", borderLeft: "2px solid #2a4868",
  borderRadius: 3, color: "#a8bcd4", fontSize: 10, fontStyle: "italic", lineHeight: 1.4,
};
const sectionHdr: React.CSSProperties = {
  color: "#6a86a6", fontSize: 10, fontWeight: 700, textTransform: "uppercase",
  letterSpacing: 0.5, borderBottom: "1px solid #1e2e42", paddingBottom: 2, marginBottom: 3,
};
const currencyBox: React.CSSProperties = {
  margin: "2px 0 6px", padding: "5px 7px", background: "#0d1a14",
  border: "1px solid #2a4838", borderRadius: 5,
};
const estateBox: React.CSSProperties = {
  margin: "5px 0 3px", padding: "5px 7px", background: "#161208",
  border: "1px solid #4a3f1e", borderRadius: 5,
};
const row: React.CSSProperties = { display: "flex", alignItems: "center", gap: 4, fontSize: 11, padding: "1px 2px" };
const emptyTxt: React.CSSProperties = { color: "#506680", fontSize: 10, fontStyle: "italic", padding: "2px 3px" };
