import { useEffect, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useGoodsStore } from "../state/goodsStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetHub } from "../bridge/tauri";
import type { EconHub, HubCurrency, HubDetail } from "../types";
import { climatePhrase } from "./climate";
import { CoatOfArms } from "./CoatOfArms";

type Tab = "overview" | "market" | "society" | "history";

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

  const [tab, setTab] = useState<Tab>("overview");
  const [detail, setDetail] = useState<HubDetail | null>(null);

  // Reset to the Overview tab whenever a different hub is opened.
  useEffect(() => { setTab("overview"); }, [selectedHub]);

  // Pull live per-hub detail (sentiment/market/history) while a campaign runs.
  useEffect(() => {
    let alive = true;
    if (selectedHub === null || !campActive) { setDetail(null); return; }
    campaignGetHub(selectedHub).then((d) => { if (alive) setDetail(d); }).catch(() => { if (alive) setDetail(null); });
    return () => { alive = false; };
  }, [selectedHub, campActive]);

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
    { id: "overview", label: "Overview" },
    { id: "market", label: "Market" },
    { id: "society", label: "Society" },
    { id: "history", label: "History" },
  ];

  return (
    <div style={panel}>
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

      {/* ════════════ OVERVIEW ════════════ */}
      {tab === "overview" && (
        <>
          <div style={statGrid}>
            <Stat label="Throughput" value={fmt(hub.throughput ?? 0)} />
            <Stat label="Exports →" value={fmt(hub.exports ?? 0)} />
            <Stat label="← Imports" value={fmt(hub.imports ?? 0)} />
            <Stat label="Partners" value={String(hub.partners ?? 0)} />
            <Stat label="Wealth" value={`${Math.round(hub.wealth * 100)}%`} />
            <Stat label="Population" value={(detail?.population ?? hub.population).toLocaleString()} />
          </div>
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

      {/* ════════════ MARKET ════════════ */}
      {tab === "market" && (
        <>
          <div style={{ ...sectionHdr }}>
            Market — price × world standard (green cheap · red dear)
          </div>
          {hub.market && (
            <div style={{ display: "flex", gap: 4, margin: "2px 0 4px" }}>
              <Stat label="Grain wealth" value={(detail?.grain_wealth ?? hub.market.grain_wealth).toFixed(2)} />
              <Stat label="Trade wealth" value={(detail?.trade_wealth ?? hub.market.trade_wealth).toFixed(2)} />
            </div>
          )}
          {hub.market?.currencies && hub.market.currencies.length > 0 ? (
            <CurrencyCard currencies={hub.market.currencies} iconFor={iconFor} labelFor={labelFor} />
          ) : hub.market && hub.market.currency_goods.length > 0 ? (
            <div style={{ color: "#e0c060", fontSize: 10, margin: "0 0 4px" }}>
              <span style={{ color: "#6a86a6" }}>Currency here: </span>
              {hub.market.currency_goods.map((c) => `${iconFor(c)} ${labelFor(c)}`).join(", ")}
            </div>
          ) : null}

          {/* Live market (campaign): supply/demand + cheapest/dearest */}
          {detail ? (
            <>
              {detail.history.length > 1 && (
                <div style={{ margin: "2px 0 6px" }}>
                  <div style={{ color: "#6a86a6", fontSize: 9 }}>Local price index over time</div>
                  <Sparkline values={detail.history.map((s) => s.price_index)} color="#e0c060" baseline={1} />
                </div>
              )}
              {[...detail.goods]
                .filter((g) => g.production > 0 || g.stock > 0.01 || g.price > g.base_value * 1.05)
                .sort((a, b) => (b.price / b.base_value) - (a.price / a.base_value))
                .slice(0, 16)
                .map((g) => {
                  const xw = g.price / Math.max(1e-6, g.base_value);
                  const supply = g.stock; const demand = Math.max(1e-6, g.need);
                  const ratio = Math.min(1, supply / (demand * 6));
                  return (
                    <div key={g.good} style={{ marginBottom: 3 }}>
                      <div style={{ display: "flex", alignItems: "baseline", gap: 5, fontSize: 10 }}>
                        <span style={{ minWidth: 92, color: "#9ab0c8", overflow: "hidden", whiteSpace: "nowrap", textOverflow: "ellipsis" }}>
                          {iconFor(g.name)} {labelFor(g.name)}
                        </span>
                        <span style={{ minWidth: 40, textAlign: "right", fontWeight: 600,
                          color: xw > 1.3 ? "#e08080" : xw < 0.77 ? "#7fd0a0" : "#c0d0e0" }}>{xw.toFixed(2)}×</span>
                        <span style={{ color: "#6a86a6", flex: 1, fontSize: 9, textAlign: "right", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                          title={`cheapest ${g.world_min.toFixed(1)}× at ${g.world_min_hub} · dearest ${g.world_max.toFixed(1)}× at ${g.world_max_hub}`}>
                          ▼{g.world_min_hub.slice(0, 7)} ▲{g.world_max_hub.slice(0, 7)}
                        </span>
                      </div>
                      {/* supply vs demand bar */}
                      <div style={{ position: "relative", height: 4, background: "#3a1e1e", borderRadius: 2 }} title={`stock ${supply.toFixed(1)} vs need ${demand.toFixed(1)}`}>
                        <div style={{ position: "absolute", inset: 0, width: `${ratio * 100}%`, background: ratio > 0.5 ? "#5fc8a8" : "#c89a4a", borderRadius: 2 }} />
                      </div>
                    </div>
                  );
                })}
            </>
          ) : hub.market ? (
            hub.market.prices.slice(0, 14).map((p) => (
              <div key={p.good} style={{ display: "flex", alignItems: "baseline", gap: 6, fontSize: 10, padding: "1px 0" }}>
                <span style={{ minWidth: 92, color: "#9ab0c8", overflow: "hidden", whiteSpace: "nowrap", textOverflow: "ellipsis" }}>
                  {iconFor(p.good_name)} {labelFor(p.good_name)}
                </span>
                <span style={{ minWidth: 44, textAlign: "right", fontWeight: 600,
                  color: p.price > p.base_value * 1.3 ? "#e08080" : p.price < p.base_value * 0.77 ? "#7fd0a0" : "#c0d0e0" }}>
                  {p.price.toFixed(2)}
                </span>
                <span style={{ color: "#56708e", minWidth: 38, textAlign: "right" }}>{p.base_value.toFixed(1)}</span>
                <span style={{ color: "#6a86a6", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {p.exchanged_for.slice(0, 2).map((x) => `1 ⇄ ${x.ratio.toFixed(1)} ${labelFor(x.good_name)}`).join(" · ")}
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
        </>
      )}

      {/* ════════════ SOCIETY ════════════ */}
      {tab === "society" && (
        <>
          <div style={sectionHdr}>Society</div>
          <div style={{ display: "flex", gap: 4 }}>
            <ClassTile label="Nobility" value={hub.nobility ?? 0} level={hub.elite_level ?? 0} color="#e0c060" />
            <ClassTile label="Merchants" value={hub.merchants ?? 0} level={hub.merchant_level ?? 0} color="#5fc8a8" />
            <ClassTile label="Commoners" value={hub.commoners ?? 0} level={1} color="#8aa0c0" />
          </div>

          {/* Merchant houses resident here + their share of the city's house wealth */}
          {detail && detail.houses && detail.houses.length > 0 && (() => {
            const hs = detail.houses;
            const total = Math.max(1e-6, hs.reduce((s, h) => s + Math.max(0, h.wealth), 0));
            return (
              <>
                <div style={{ ...sectionHdr, marginTop: 6 }}>Merchant houses (share of local trade wealth)</div>
                {hs.map((h, i) => (
                  <div key={h.name + i} style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 3 }}>
                    <CoatOfArms name={h.name} size={20} />
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

      {/* ════════════ HISTORY ════════════ */}
      {tab === "history" && (
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
const row: React.CSSProperties = { display: "flex", alignItems: "center", gap: 4, fontSize: 11, padding: "1px 2px" };
const emptyTxt: React.CSSProperties = { color: "#506680", fontSize: 10, fontStyle: "italic", padding: "2px 3px" };
