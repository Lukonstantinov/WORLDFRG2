import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { useGoodsStore } from "@state/goodsStore";
import { useCampaignStore } from "@state/campaignStore";
import { useViewportStore } from "@state/viewportStore";
import { campaignGetWorldEconomy, campaignGoodAtlas } from "@bridge";
import type { EconChain, WorldGoodPrice, GoodAtlas, AtlasHub } from "@types";
import { commodityHistory } from "@app/commodityHistory";
import { goodOverlayKey, GOOD_DEFS } from "@goods";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { GoodIcon } from "./GoodIcon";

/** 📖 Goods Codex — redesigned (UI/UX pass).
 *
 *  ONE always-on header answers "what state is this good in?" at a glance —
 *  identity, a VITAL-SIGNS strip (price · producing cities · quality · scarcity ·
 *  traded), and a SMART BANNER that DIAGNOSES an empty state ("belt on the map but
 *  no producer → outside every catchment") instead of a dead "not produced yet".
 *  Below it, four purpose-built tabs (Overview · Places · Markets · Story) replace
 *  the old seven co-equal ones, and every city row FLIES the map to that place.
 *
 *  Data: `campaign_good_atlas` (live per-good facets) + the worldgen economy
 *  snapshot (routes, prices) + `campaign_get_world_economy` (live world prices). */
export function GoodsCodexPanel() {
  const open = useUIStore((s) => s.showGoodsCodex);
  const codexGood = useUIStore((s) => s.codexGood);
  const setCodexGood = useUIStore((s) => s.setCodexGood);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const scarcityOn = useUIStore((s) => s.overlayVisibility.goodScarcity);
  const heatGoodOn = useUIStore((s) => s.heatGood !== null && s.heatGood === s.codexGood);
  const economy = useWorldStore((s) => s.economy);
  const specs = useGoodsStore((s) => s.specs);
  const meta = useGoodsStore((s) => s.meta);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const focusOn = useViewportStore((s) => s.focusOn);
  const active = !!snapshot?.active;
  const year = snapshot?.clock?.year ?? 0;

  const [tab, setTab] = useState<"overview" | "places" | "markets" | "story">("overview");
  const [query, setQuery] = useState("");
  const overlayVis = useUIStore((s) => s.overlayVisibility);
  const flowOn = !!overlayVis.goodFlow;
  const goodFlowColors = useUIStore((s) => s.goodFlowColors);
  const goodFlowColorGlobal = useUIStore((s) => s.goodFlowColorGlobal);
  const setGoodFlowColor = useUIStore((s) => s.setGoodFlowColor);
  const setGoodFlowColorGlobal = useUIStore((s) => s.setGoodFlowColorGlobal);

  // Live per-good facets (quality / trade / control / flow), refreshed each campaign year.
  const [atlas, setAtlas] = useState<GoodAtlas | null>(null);
  useEffect(() => {
    if (!open || !active || !codexGood) { setAtlas(null); return; }
    let alive = true;
    campaignGoodAtlas(codexGood).then((a) => { if (alive) setAtlas(a); }).catch(() => { if (alive) setAtlas(null); });
    return () => { alive = false; };
  }, [open, active, codexGood, year]);
  // Live per-good world prices from the running campaign.
  const [liveGoods, setLiveGoods] = useState<WorldGoodPrice[]>([]);
  useEffect(() => {
    if (!open || !active) { setLiveGoods([]); return; }
    let alive = true;
    campaignGetWorldEconomy().then((we) => { if (alive) setLiveGoods(we.goods); }).catch(() => { if (alive) setLiveGoods([]); });
    return () => { alive = false; };
  }, [open, active, year]);

  const goodIds = useMemo(() => {
    if (specs.length > 0) return specs.filter((s) => s.enabled).map((s) => s.id);
    return economy?.goods ?? [];
  }, [specs, economy]);

  const filteredIds = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return goodIds;
    return goodIds.filter((id) => meta(id).name.toLowerCase().includes(q) || id.toLowerCase().includes(q));
  }, [goodIds, query, meta]);

  useEffect(() => {
    if (open && !codexGood && goodIds.length > 0) setCodexGood(goodIds[0]);
  }, [open, codexGood, goodIds, setCodexGood]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.goodbrowser);
  if (!open) return null;
  const close = () => {
    setOverlayVisible("goodScarcity", false);
    setOverlayVisible("goodFlow", false);
    useUIStore.getState().setHeatGood(null);
    useUIStore.getState().setShowGoodsCodex(false);
  };

  const spec = specs.find((s) => s.id === codexGood) ?? null;
  const m = codexGood ? meta(codexGood) : null;
  const hist = codexGood ? commodityHistory(codexGood) : null;
  const qkey = codexGood ? goodOverlayKey(codexGood) : "";
  const qualityOn = !!overlayVis[qkey];

  const hubName = (id: number) => economy?.hubs.find((h) => h.id === id)?.name ?? `Hub ${id}`;
  const routes: EconChain[] = (economy?.chains ?? [])
    .filter((c) => c.good_name === codexGood)
    .sort((a, b) => b.value - a.value)
    .slice(0, 6);

  const premiums = (economy?.hubs ?? [])
    .map((h) => {
      const mg = h.market?.prices.find((p) => p.good_name === codexGood);
      return mg && mg.base_value > 0 ? { name: h.name, premium: mg.price / mg.base_value } : null;
    })
    .filter((x): x is { name: string; premium: number } => x !== null)
    .sort((a, b) => a.premium - b.premium);
  const cheapest = premiums.slice(0, 4);
  const dearest = premiums.slice(-4).reverse();

  // ── Vital signs — the good's state in one row ──
  const lg = active ? liveGoods.find((g) => g.name === codexGood) ?? null : null;
  const producingCities = atlas ? atlas.quality_hist.reduce((a, b) => a + b, 0) : (lg?.producers ?? 0);
  const worldPrice = lg?.world_price ?? null;
  const scar = scarcity(worldPrice, active ? producingCities : undefined);
  const topHouse = atlas && atlas.houses.length > 0 ? [...atlas.houses].sort((a, b) => b.share - a.share)[0] : null;

  const idx = codexGood ? goodIds.indexOf(codexGood) : -1;
  const step = (d: number) => { if (idx >= 0 && goodIds.length > 0) setCodexGood(goodIds[(idx + d + goodIds.length) % goodIds.length]); };
  const flyTo = (h: AtlasHub) => focusOn(h.x, h.y);

  const manufactured = spec?.distribution === "manufactured";
  const subtitle = spec ? [spec.category, spec.distribution, manufactured ? null : spec.domain].filter(Boolean).join(" · ") : "";
  const usedIn = spec ? specs.filter((s) => s.enabled && (s.inputs ?? []).some((i) => i.good === codexGood)) : [];

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }} onPointerDown={onPointerDown}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>📖 Goods Codex</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>

      {/* Picker: search + select + prev/next */}
      <div data-no-drag style={{ padding: "8px 10px 4px", display: "flex", alignItems: "center", gap: 6 }}>
        <input placeholder="🔎 find a good…" value={query} onChange={(e) => setQuery(e.target.value)} style={search} />
        <select value={codexGood ?? ""} onChange={(e) => {
          const id = e.target.value || null;
          setCodexGood(id);
          if (useUIStore.getState().heatGood && id) useUIStore.getState().setHeatGood(id);
        }} style={select}>
          {filteredIds.map((id) => <option key={id} value={id}>{meta(id).name}</option>)}
        </select>
        <button data-no-drag onClick={() => step(-1)} style={navBtn} title="Previous good">◀</button>
        <button data-no-drag onClick={() => step(1)} style={navBtn} title="Next good">▶</button>
      </div>

      {/* Identity */}
      <div style={{ padding: "2px 12px 6px", display: "flex", alignItems: "center", gap: 10 }}>
        {codexGood
          ? <GoodIcon name={codexGood} size={48} treatment="victorian" title={m?.name} />
          : <span style={{ fontSize: 30, lineHeight: 1 }}>📦</span>}
        <div style={{ minWidth: 0 }}>
          <div style={{ color: "#eaf2fb", fontWeight: 700, fontSize: 15 }}>{m?.name ?? "—"}</div>
          <div style={{ color: "#6a86a6", fontSize: 9.5, textTransform: "capitalize" }}>{subtitle}</div>
        </div>
        <span style={{ flex: 1 }} />
        <span style={{ ...badge, color: scar.color, background: scar.bg, borderColor: scar.color + "55" }}>{scar.label}</span>
      </div>

      {/* VITAL SIGNS */}
      <div style={{ display: "flex", gap: 6, padding: "0 10px 8px" }}>
        <Vital label="world price" value={worldPrice != null ? `${worldPrice.toFixed(2)}×` : "—"}
          color={worldPrice == null ? "#cfe2f6" : worldPrice >= 1.3 ? "#e88" : worldPrice <= 0.8 ? "#7fd0a0" : "#cfe2f6"} />
        <Vital label="producing" value={active ? `${producingCities}` : "—"} color={active && producingCities === 0 ? "#e88" : "#cfe2f6"} />
        <Vital label="avg quality" value={atlas ? `${Math.round(atlas.avg_quality * 100)}%` : "—"} />
        <Vital label="traded/yr" value={atlas ? fmtn(atlas.total_traded) : "—"} />
      </div>

      {/* SMART BANNER — diagnose the empty state, don't just report it */}
      {(() => {
        if (!economy) return <Banner tone="info" text="Run the Economy step (10) so prices, routes and production are available." />;
        if (!active) return <Banner tone="info" text="Begin the campaign — the Codex then reads the living economy: who makes it, who runs it, where it's dear." />;
        if (atlas && producingCities === 0) return (
          <Banner tone="warn"
            text={`No city produces ${m?.name ?? "this good"} — its belt sits outside every settlement's catchment. Show the belt to see where a winery could be founded.`}
            action={{ label: qualityOn ? "Hide belt" : "Show belt on map", onClick: () => setOverlayVisible(qkey, !qualityOn) }} />
        );
        if (manufactured) return <Banner tone="info" text={`Manufactured in cities from a recipe — it has no map belt of its own. See "Made from" in the Story tab.`} />;
        return null;
      })()}

      {/* Overlay chips — one consistent place for every map layer this good drives */}
      {active && !manufactured && (
        <div style={{ display: "flex", gap: 5, padding: "0 10px 6px", flexWrap: "wrap" }}>
          <Chip on={qualityOn} onClick={() => setOverlayVisible(qkey, !qualityOn)}>⭐ Quality</Chip>
          <Chip on={flowOn} onClick={() => setOverlayVisible("goodFlow", !flowOn)}>🔀 Flow</Chip>
          <Chip on={heatGoodOn} onClick={() => {
            const s = useUIStore.getState();
            if (s.heatGood === codexGood) { s.setHeatGood(null); return; }
            s.setHeatGood(codexGood); s.setOverlayVisible("tradeHeat", true);
          }}>🔥 Heat</Chip>
          <Chip on={scarcityOn} onClick={() => setOverlayVisible("goodScarcity", !scarcityOn)}>⚖ Scarcity</Chip>
        </div>
      )}

      {/* Tabs */}
      <div style={{ display: "flex", gap: 2, padding: "0 8px", borderBottom: "1px solid #1e2e42" }}>
        {([["overview", "Overview"], ["places", "Places"], ["markets", "Markets"], ["story", "Story"]] as const).map(([id, lbl]) => (
          <div key={id} onClick={() => setTab(id)}
            style={{ padding: "5px 10px", cursor: "pointer", fontSize: 11, fontWeight: tab === id ? 700 : 400,
              color: tab === id ? "#cfe2f6" : "#6a86a6",
              borderBottom: tab === id ? "2px solid #6a9adf" : "2px solid transparent" }}>
            {lbl}
          </div>
        ))}
      </div>

      <div style={{ overflowY: "auto", padding: "8px 10px 12px", maxHeight: "52vh" }}>
        {/* ── OVERVIEW ── */}
        {tab === "overview" && (
          !active ? <div style={empty}>Begin the campaign to see this good's living economy.</div> :
          !atlas ? <div style={empty}>Loading…</div> : (
            <>
              <div style={sectionHdr}>Where it's made</div>
              {atlas.top_quality.length === 0 ? <div style={empty}>Not produced anywhere yet — see the banner above.</div> :
                atlas.producers.slice(0, 4).map((h) => (
                  <div key={h.hub} style={clickRow} onClick={() => flyTo(h)} title="Fly to this city">
                    <span>📍 {h.name}</span><span style={{ color: "#7fd0a0" }}>{fmtn(h.amount)}/yr</span>
                  </div>
                ))}

              <div style={{ ...sectionHdr, marginTop: 10 }}>Who commands it</div>
              {topHouse ? (
                <div style={{ ...rankRow, alignItems: "center" }}>
                  <span>{topHouse.is_guild ? "⚒ " : "🏛 "}{topHouse.name}</span>
                  <span style={{ color: "#d9a441" }}>{Math.round(topHouse.share * 100)}%</span>
                </div>
              ) : <div style={empty}>No house or guild dominates its trade yet.</div>}

              <div style={{ ...sectionHdr, marginTop: 10 }}>Quality spread</div>
              <QualityHist hist={atlas.quality_hist} />

              {(spec && ((spec.inputs?.length ?? 0) > 0 || usedIn.length > 0)) && (
                <>
                  <div style={{ ...sectionHdr, marginTop: 10 }}>Supply chain</div>
                  {manufactured && (spec.inputs?.length ?? 0) > 0 && (
                    <ChipRow label="made from" items={(spec.inputs ?? []).map((i) => ({ id: i.good, qty: i.qty }))} meta={meta} onPick={setCodexGood} />
                  )}
                  {usedIn.length > 0 && (
                    <ChipRow label="used in" items={usedIn.map((s) => ({ id: s.id }))} meta={meta} onPick={setCodexGood} />
                  )}
                </>
              )}
            </>
          )
        )}

        {/* ── PLACES ── */}
        {tab === "places" && (
          !active ? <div style={empty}>Begin the campaign to map production and trade.</div> :
          !atlas ? <div style={empty}>Loading…</div> : (
            <>
              <div style={sectionHdr}>Finest sources</div>
              {atlas.top_quality.length === 0 ? <div style={empty}>Not produced anywhere yet.</div> :
                atlas.top_quality.slice(0, 10).map((h) => (
                  <div key={h.hub} style={clickRow} onClick={() => flyTo(h)} title="Fly to this city">
                    <span>📍 {h.name}</span><span style={{ color: "#e3c14a" }}>{Math.round(h.amount * 100)}%</span>
                  </div>
                ))}
              {flowOn && (
                <>
                  <div style={{ ...sectionHdr, marginTop: 10 }}>Busiest lanes ({atlas.flows.length})</div>
                  {atlas.flows.length === 0 ? <div style={empty}>No shipments last year.</div> :
                    atlas.flows.slice(0, 10).map((f, i) => (
                      <div key={i} style={rankRow}>
                        <span style={ellip}>{hubName(f.from)} → {hubName(f.to)}</span>
                        <span style={{ color: "#8aa8c8" }}>{fmtn(f.amount)}</span>
                      </div>
                    ))}
                </>
              )}
              <div style={{ color: "#5a6a80", fontSize: 9, marginTop: 8 }}>
                Toggle ⭐ Quality / 🔀 Flow above to paint the map. Click a city to fly there.
              </div>
              {flowOn && codexGood && (() => {
                const def = GOOD_DEFS.find((d) => d.name === codexGood);
                const effective = goodFlowColorGlobal ?? goodFlowColors[codexGood] ?? def?.color ?? "#e0b24a";
                return (
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
                    <span style={{ color: "#8aa0b8", fontSize: 10 }}>Lane colour</span>
                    <input type="color" value={effective} disabled={!!goodFlowColorGlobal}
                      onChange={(e) => setGoodFlowColor(codexGood, e.target.value)}
                      style={{ width: 30, height: 20, padding: 0, border: "1px solid #2a3a4a", borderRadius: 4, background: "none" }} />
                    <label style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 9.5, color: "#8aa0b8" }}>
                      <input type="checkbox" checked={!!goodFlowColorGlobal}
                        onChange={(e) => setGoodFlowColorGlobal(e.target.checked ? effective : null)} /> same for all
                    </label>
                  </div>
                );
              })()}
            </>
          )
        )}

        {/* ── MARKETS ── */}
        {tab === "markets" && (
          <>
            {lg && (
              <div style={liveCard}>
                <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                  <span style={{ color: "#7fd0a0", fontWeight: 700, fontSize: 11 }}>Live market</span>
                  <span style={{ color: "#5a7a66", fontSize: 9 }}>year {year}</span>
                  <span style={{ flex: 1 }} />
                  <span style={{ color: lg.world_price >= 1.3 ? "#e08080" : lg.world_price <= 0.77 ? "#7fd0a0" : "#cfe2f6", fontWeight: 700, fontSize: 12 }}>{lg.world_price.toFixed(2)}×</span>
                </div>
                <div style={{ color: "#8aa0b8", fontSize: 9.5, marginTop: 2 }}>
                  {lg.producers} producing {lg.producers === 1 ? "city" : "cities"}{lg.top_hub ? <> · top: <span style={{ color: "#cfe2f6" }}>{lg.top_hub}</span></> : null}
                </div>
              </div>
            )}
            {atlas && (
              <>
                <div style={sectionHdr}>Top producers</div>
                <AtlasBars rows={atlas.producers.map((p) => ({ label: p.name, value: p.amount }))} color="#7fd0a0" fmt={fmtn} />
                <div style={{ ...sectionHdr, marginTop: 8 }}>Consumed most</div>
                <AtlasBars rows={atlas.consumers.map((p) => ({ label: p.name, value: p.amount }))} color="#ff9a6a" fmt={fmtn} />
              </>
            )}
            <div style={{ ...sectionHdr, marginTop: 10 }}>Cheapest vs dearest {active ? "(worldgen prices)" : ""}</div>
            {premiums.length === 0 ? <div style={empty}>No price data — run the Economy step.</div> : (
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
                <div>
                  <div style={{ ...sectionHdr, color: "#7fd0a0" }}>Cheapest</div>
                  {cheapest.map((c) => <div key={c.name} style={rankRow}><span style={ellip}>{c.name}</span><span style={{ color: "#7fd0a0" }}>×{c.premium.toFixed(2)}</span></div>)}
                </div>
                <div>
                  <div style={{ ...sectionHdr, color: "#ff9a6a" }}>Dearest</div>
                  {dearest.map((c) => <div key={c.name} style={rankRow}><span style={ellip}>{c.name}</span><span style={{ color: "#ff9a6a" }}>×{c.premium.toFixed(2)}</span></div>)}
                </div>
              </div>
            )}
            <div style={{ color: "#5a6a80", fontSize: 9, marginTop: 8 }}>Premium = local price ÷ base value. A merchant buys cheap and carries to the dear markets.</div>
          </>
        )}

        {/* ── STORY ── */}
        {tab === "story" && (
          <>
            {atlas && atlas.houses.length > 0 && (
              <>
                <div style={sectionHdr}>Control — share of this good</div>
                <AtlasBars rows={atlas.houses.map((h) => ({ label: (h.is_guild ? "⚒ " : "🏛 ") + h.name, value: h.share }))} color="#6a9adf" fmt={(v) => `${Math.round(v * 100)}%`} norm={1} />
              </>
            )}
            {manufactured && (spec?.inputs?.length ?? 0) > 0 && (
              <div style={{ marginTop: 10 }}>
                <div style={sectionHdr}>Made from</div>
                <ChipRow label="" items={(spec?.inputs ?? []).map((i) => ({ id: i.good, qty: i.qty }))} meta={meta} onPick={setCodexGood} />
              </div>
            )}
            <div style={{ ...sectionHdr, marginTop: 10 }}>Trade routes ({routes.length})</div>
            {routes.length === 0 && <div style={empty}>No routed shipments for this good.</div>}
            {routes.map((r) => {
              const first = r.stops[0]; const last = r.stops[r.stops.length - 1];
              return (
                <div key={r.id} style={routeCard}>
                  <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                    <span style={{ color: "#7fd0a0", fontWeight: 600, fontSize: 11 }}>{hubName(first?.hub)}</span>
                    <span style={{ color: "#5a6a80" }}>→</span>
                    <span style={{ color: "#ff9a6a", fontWeight: 600, fontSize: 11 }}>{hubName(last?.hub)}</span>
                    <span style={{ flex: 1 }} />
                    <span style={{ color: "#8aa8c8", fontSize: 9 }}>{Math.round(r.km)} km · {r.days.toFixed(0)}d</span>
                  </div>
                  <div style={{ color: "#9ab0c8", fontSize: 9, marginTop: 2 }}>{r.stops.map((s) => hubName(s.hub)).join(" → ")}</div>
                </div>
              );
            })}
            <div style={{ ...sectionHdr, marginTop: 10 }}>Real-world history</div>
            {hist ? (
              <div style={{ background: "#0e1a14", border: "1px solid #1c3326", borderRadius: 6, padding: 10 }}>
                <div style={{ color: "#d8c878", fontSize: 10, fontWeight: 700 }}>{hist.era}</div>
                <div style={{ color: "#cfe2f6", fontSize: 11, lineHeight: 1.6, marginTop: 5 }}>{hist.note}</div>
              </div>
            ) : <div style={empty}>No historical card yet — it trades as a regional commodity in this world.</div>}
          </>
        )}
      </div>
    </div>
  );
}

// ── helpers ──
const fmtn = (n: number) => Math.round(n).toLocaleString();

function scarcity(price: number | null, producing?: number): { label: string; color: string; bg: string } {
  if (producing === 0) return { label: "ABSENT", color: "#ff8a8a", bg: "#2e1414" };
  if (price == null) return { label: "—", color: "#8aa0b8", bg: "#0c1622" };
  if (price >= 1.5) return { label: "VERY SCARCE", color: "#ff9a6a", bg: "#2e1a10" };
  if (price >= 1.2) return { label: "SCARCE", color: "#e0b060", bg: "#241f10" };
  if (price <= 0.8) return { label: "ABUNDANT", color: "#7fd0a0", bg: "#0e2418" };
  return { label: "NORMAL", color: "#9ab0c8", bg: "#0c1622" };
}

function Vital({ label, value, color = "#cfe2f6" }: { label: string; value: string; color?: string }) {
  return (
    <div style={{ flex: 1, background: "#0c1622", border: "1px solid #16283c", borderRadius: 5, padding: "4px 5px", minWidth: 0 }}>
      <div style={{ color, fontWeight: 700, fontSize: 12.5, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{value}</div>
      <div style={{ color: "#6a86a6", fontSize: 8, whiteSpace: "nowrap" }}>{label}</div>
    </div>
  );
}

function Banner({ tone, text, action }: { tone: "info" | "warn"; text: string; action?: { label: string; onClick: () => void } }) {
  const c = tone === "warn"
    ? { bg: "rgba(224,150,80,0.10)", bd: "rgba(224,150,80,0.35)", fg: "#f0cfa4" }
    : { bg: "rgba(90,140,220,0.08)", bd: "rgba(90,140,220,0.25)", fg: "#a9c4e6" };
  return (
    <div style={{ margin: "0 10px 8px", padding: "6px 8px", background: c.bg, border: `1px solid ${c.bd}`, borderRadius: 6 }}>
      <div style={{ fontSize: 10.5, color: c.fg, lineHeight: 1.45 }}>{tone === "warn" ? "⚠ " : "ℹ "}{text}</div>
      {action && (
        <button data-no-drag onClick={action.onClick}
          style={{ marginTop: 5, padding: "3px 8px", fontSize: 10, borderRadius: 4, cursor: "pointer", border: "1px solid #2a4a6a", background: "#123", color: "#bcd8f4" }}>
          {action.label}
        </button>
      )}
    </div>
  );
}

function Chip({ on, onClick, children }: { on: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button data-no-drag onClick={onClick} style={{
      padding: "3px 9px", borderRadius: 12, fontSize: 10, cursor: "pointer", whiteSpace: "nowrap",
      border: `1px solid ${on ? "#5a9adf" : "#1e2e42"}`, background: on ? "#12314e" : "transparent",
      color: on ? "#cfe6ff" : "#6a86a6", fontWeight: on ? 700 : 400,
    }}>{children}</button>
  );
}

function QualityHist({ hist }: { hist: number[] }) {
  const mx = Math.max(1, ...hist);
  const total = hist.reduce((a, b) => a + b, 0);
  return (
    <>
      <div style={{ display: "flex", alignItems: "flex-end", gap: 2, height: 44, margin: "4px 0 2px" }}>
        {hist.map((c, i) => (
          <div key={i} title={`${GRADES[Math.min(4, Math.floor(i / 2))]} · ${c} ${c === 1 ? "city" : "cities"}`}
            style={{ flex: 1, height: `${(c / mx) * 100}%`, minHeight: c > 0 ? 2 : 0, background: `hsl(${140 - i * 11} 55% 50%)`, borderRadius: 1 }} />
        ))}
      </div>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 8, color: "#5a6a80" }}>
        <span>coarse</span><span>ordinary</span><span>good</span><span>fine</span><span>exquisite</span>
      </div>
      {total === 0 && <div style={empty}>—</div>}
    </>
  );
}

function ChipRow({ label, items, meta, onPick }: {
  label: string; items: { id: string; qty?: number }[];
  meta: (id: string) => { icon: string; name: string }; onPick: (id: string) => void;
}) {
  if (items.length === 0) return null;
  return (
    <div style={{ margin: "3px 0" }}>
      {label && <span style={{ color: "#6a86a6", fontSize: 9, marginRight: 6 }}>{label}</span>}
      <span style={{ display: "inline-flex", flexWrap: "wrap", gap: 5 }}>
        {items.map((it) => (
          <button key={it.id} data-no-drag onClick={() => onPick(it.id)}
            style={{ ...tag, display: "inline-flex", alignItems: "center", gap: 4 }} title="Trace this good">
            <GoodIcon name={it.id} size={16} />
            {meta(it.id).name}{it.qty != null ? <span style={{ color: "#7fa0c0", marginLeft: 3 }}>×{it.qty}</span> : null}
          </button>
        ))}
      </span>
    </div>
  );
}

function AtlasBars({ rows, color, fmt, norm }: {
  rows: { label: string; value: number }[]; color: string; fmt: (v: number) => string; norm?: number;
}) {
  if (rows.length === 0) return <div style={empty}>—</div>;
  const mx = norm ?? Math.max(1e-6, ...rows.map((r) => r.value));
  return (
    <div>
      {rows.slice(0, 8).map((r, i) => (
        <div key={i} style={{ display: "flex", alignItems: "center", gap: 6, padding: "1.5px 0" }}>
          <span style={{ width: 92, fontSize: 9.5, color: "#bcd0e4", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.label}</span>
          <div style={{ flex: 1, height: 8, background: "#0c1622", borderRadius: 3, overflow: "hidden" }}>
            <div style={{ width: `${Math.min(100, (r.value / mx) * 100)}%`, height: "100%", background: color, borderRadius: 3 }} />
          </div>
          <span style={{ width: 46, textAlign: "right", fontSize: 9, color: "#8aa0b8" }}>{fmt(r.value)}</span>
        </div>
      ))}
    </div>
  );
}

const GRADES = ["coarse", "ordinary", "good", "fine", "exquisite"];

const panel: React.CSSProperties = {
  position: "absolute", top: 60, left: 70, width: 360, maxHeight: "84vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 40,
};
const headerBase: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e", color: "#cfe0f4", fontWeight: 700, fontSize: 12,
};
const header = headerBase;
const search: React.CSSProperties = {
  width: 110, background: "#080c12", border: "1px solid #1e2e42", color: "#cfe0f4",
  padding: "3px 6px", borderRadius: 4, fontSize: 10.5,
};
const select: React.CSSProperties = {
  flex: 1, background: "#080c12", border: "1px solid #1e2e42", color: "#cfe0f4",
  padding: "3px 5px", borderRadius: 4, fontSize: 11, minWidth: 0,
};
const navBtn: React.CSSProperties = {
  background: "#0c1622", border: "1px solid #1e2e42", color: "#8aa8c8",
  borderRadius: 4, padding: "3px 6px", fontSize: 10, cursor: "pointer",
};
const badge: React.CSSProperties = {
  fontSize: 9, fontWeight: 700, letterSpacing: 0.4, padding: "2px 7px", borderRadius: 10, border: "1px solid",
};
const sectionHdr: React.CSSProperties = { color: "#8aa0b8", fontSize: 10, fontWeight: 600, marginBottom: 2 };
const routeCard: React.CSSProperties = { background: "#0c1622", borderBottom: "1px solid #131e2a", padding: "5px 2px" };
const liveCard: React.CSSProperties = { marginBottom: 10, padding: "6px 8px", background: "#0d1a14", border: "1px solid #1e3a2a", borderRadius: 6 };
const tag: React.CSSProperties = { background: "#0c1622", border: "1px solid #1e2e42", color: "#cfe0f4", borderRadius: 5, padding: "3px 7px", fontSize: 10, cursor: "pointer" };
const rankRow: React.CSSProperties = { display: "flex", justifyContent: "space-between", fontSize: 10, color: "#bcd0e4", padding: "2px 0" };
const clickRow: React.CSSProperties = { ...rankRow, cursor: "pointer", borderRadius: 3 };
const ellip: React.CSSProperties = { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" };
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "6px 0" };
