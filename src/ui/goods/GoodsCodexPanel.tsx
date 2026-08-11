import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { useGoodsStore } from "@state/goodsStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetWorldEconomy, campaignGoodAtlas } from "@bridge";
import type { EconChain, WorldGoodPrice, GoodAtlas } from "@types";
import { commodityHistory } from "@app/commodityHistory";
import { goodOverlayKey } from "@goods";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";

/** #35/#36/#37 · Goods Codex. For a chosen good:
 *   • Provenance — how it is made (recipe inputs) and the real trade routes it
 *     travels (source → … → consumer with per-stop prices), from the worldgen
 *     market snapshot + recipe DAG.
 *   • History — a real-world commodity-history card.
 *   • Scarcity — toggle a map overlay colouring every hub by its local price
 *     premium, plus a cheapest/dearest city ranking.
 *  All read the already-computed economy snapshot — no campaign required. */
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
  const active = !!snapshot?.active;
  const year = snapshot?.clock?.year ?? 0;

  const [tab, setTab] = useState<"qual" | "trade" | "ctrl" | "flow" | "prov" | "hist" | "scar">("qual");
  const overlayVis = useUIStore((s) => s.overlayVisibility);
  const flowOn = !!overlayVis.goodFlow;

  // Goods Atlas · the live per-good facets (quality / trade / control / flow), refreshed
  // once per campaign YEAR so the atlas tracks the living economy.
  const [atlas, setAtlas] = useState<GoodAtlas | null>(null);
  useEffect(() => {
    if (!open || !active || !codexGood) { setAtlas(null); return; }
    let alive = true;
    campaignGoodAtlas(codexGood).then((a) => { if (alive) setAtlas(a); }).catch(() => { if (alive) setAtlas(null); });
    return () => { alive = false; };
  }, [open, active, codexGood, year]);
  // Live per-good world prices from the running campaign, refreshed once per YEAR so
  // the Codex reflects the living market, not just the frozen worldgen snapshot.
  const [liveGoods, setLiveGoods] = useState<WorldGoodPrice[]>([]);
  useEffect(() => {
    if (!open || !active) { setLiveGoods([]); return; }
    let alive = true;
    campaignGetWorldEconomy().then((we) => { if (alive) setLiveGoods(we.goods); }).catch(() => { if (alive) setLiveGoods([]); });
    return () => { alive = false; };
  }, [open, active, year]);

  // Goods that actually exist in this world (have specs); fall back to economy.goods.
  const goodIds = useMemo(() => {
    if (specs.length > 0) return specs.filter((s) => s.enabled).map((s) => s.id);
    return economy?.goods ?? [];
  }, [specs, economy]);

  // Default the selection to the first good once the panel opens.
  useEffect(() => {
    if (open && !codexGood && goodIds.length > 0) setCodexGood(goodIds[0]);
  }, [open, codexGood, goodIds, setCodexGood]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.goodbrowser);
  if (!open) return null;
  const close = () => {
    setOverlayVisible("goodScarcity", false);
    setOverlayVisible("goodFlow", false);
    useUIStore.getState().setHeatGood(null); // heat returns to all goods
    useUIStore.getState().setShowGoodsCodex(false);
  };

  const spec = specs.find((s) => s.id === codexGood) ?? null;
  const m = codexGood ? meta(codexGood) : null;
  const hist = codexGood ? commodityHistory(codexGood) : null;

  const hubName = (id: number) => economy?.hubs.find((h) => h.id === id)?.name ?? `Hub ${id}`;
  const routes: EconChain[] = (economy?.chains ?? [])
    .filter((c) => c.good_name === codexGood)
    .sort((a, b) => b.value - a.value)
    .slice(0, 6);

  // Scarcity ranking (cheapest & dearest cities by local price premium).
  const premiums = (economy?.hubs ?? [])
    .map((h) => {
      const mg = h.market?.prices.find((p) => p.good_name === codexGood);
      return mg && mg.base_value > 0 ? { name: h.name, premium: mg.price / mg.base_value } : null;
    })
    .filter((x): x is { name: string; premium: number } => x !== null)
    .sort((a, b) => a.premium - b.premium);
  const cheapest = premiums.slice(0, 4);
  const dearest = premiums.slice(-4).reverse();

  const toggleScarcity = () => setOverlayVisible("goodScarcity", !scarcityOn);

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }} onPointerDown={onPointerDown}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>📖 Goods Codex</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>

      {/* Good picker + Batch 1 per-good Trade Heat lens (🔥 this good only). */}
      <div style={{ padding: "8px 10px 6px", display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ fontSize: 14 }}>{m?.icon ?? "📦"}</span>
        <select value={codexGood ?? ""} onChange={(e) => {
          const id = e.target.value || null;
          setCodexGood(id);
          // A live heat filter follows the selected good.
          if (useUIStore.getState().heatGood && id) useUIStore.getState().setHeatGood(id);
        }} style={select}>
          {goodIds.map((id) => (
            <option key={id} value={id}>{meta(id).name}</option>
          ))}
        </select>
        <button
          data-no-drag
          onClick={() => {
            const s = useUIStore.getState();
            if (s.heatGood === codexGood) { s.setHeatGood(null); return; }
            s.setHeatGood(codexGood);
            s.setOverlayVisible("tradeHeat", true);
          }}
          title="Trade Heat for THIS good only (toggles the 🔥 overlay to a single commodity)"
          style={{
            padding: "3px 9px", borderRadius: 5, fontSize: 10.5, cursor: "pointer", whiteSpace: "nowrap",
            border: `1px solid ${heatGoodOn ? "#d9a441" : "#1e2e42"}`,
            background: heatGoodOn ? "#2a230e" : "transparent",
            color: heatGoodOn ? "#ffd75e" : "#5a7390",
            fontWeight: heatGoodOn ? 700 : 400,
          }}>
          🔥 {heatGoodOn ? "this good" : "heat"}
        </button>
      </div>

      {/* Tabs */}
      <div style={{ display: "flex", gap: 2, padding: "0 8px", borderBottom: "1px solid #1e2e42" }}>
        {([["qual", "⭐ Quality"], ["trade", "📦 Trade"], ["ctrl", "🏛 Control"], ["flow", "🔀 Flow"], ["prov", "🧭 Provenance"], ["hist", "📜 History"], ["scar", "⚖ Scarcity"]] as const).map(([id, lbl]) => (
          <div key={id} onClick={() => setTab(id)}
            style={{ padding: "4px 8px", cursor: "pointer", fontSize: 10.5, fontWeight: tab === id ? 700 : 400,
              color: tab === id ? "#cfe2f6" : "#6a86a6",
              borderBottom: tab === id ? "2px solid #6a9adf" : "2px solid transparent" }}>
            {lbl}
          </div>
        ))}
      </div>

      <div style={{ overflowY: "auto", padding: "8px 10px 12px", maxHeight: "60vh" }}>
        {!economy && (tab === "prov" || tab === "scar") && (
          <div style={empty}>Run the Economy step (10) so prices and trade routes are available.</div>
        )}

        {/* ── GOODS ATLAS — live per-good facets (need a running campaign) ── */}
        {(tab === "qual" || tab === "trade" || tab === "ctrl" || tab === "flow") && (
          !active ? (
            <div style={empty}>Begin the campaign — the Atlas reads the living economy (quality, trade volume, control and flow).</div>
          ) : !atlas ? (
            <div style={empty}>Loading…</div>
          ) : tab === "qual" ? (
            <>
              {codexGood && (() => {
                const qkey = goodOverlayKey(codexGood);
                const qOn = !!overlayVis[qkey];
                return (
                  <button onClick={() => setOverlayVisible(qkey, !qOn)} style={{ ...btn, background: qOn ? "#7a3a3a" : "#2060a0", marginBottom: 8 }}>
                    {qOn ? "Hide quality on map" : "Show quality on map"}
                  </button>
                );
              })()}
              <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
                <AtlasStat label="avg quality" value={`${Math.round(atlas.avg_quality * 100)}%`} />
                <AtlasStat label="produced/yr" value={fmtn(atlas.total_produced)} />
                <AtlasStat label="cities" value={String(atlas.quality_hist.reduce((a, b) => a + b, 0))} />
              </div>
              <div style={sectionHdr}>Quality distribution</div>
              <div style={{ display: "flex", alignItems: "flex-end", gap: 2, height: 46, margin: "4px 0 2px" }}>
                {atlas.quality_hist.map((c, i) => {
                  const mx = Math.max(1, ...atlas.quality_hist);
                  return <div key={i} title={`${i * 10}–${i * 10 + 10}% · ${c} ${c === 1 ? "city" : "cities"}`}
                    style={{ flex: 1, height: `${(c / mx) * 100}%`, minHeight: c > 0 ? 2 : 0, background: `hsl(${140 - i * 11} 55% 50%)`, borderRadius: 1 }} />;
                })}
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 8, color: "#5a6a80" }}><span>marginal</span><span>prime</span></div>
              <div style={{ ...sectionHdr, marginTop: 8 }}>Finest sources</div>
              {atlas.top_quality.slice(0, 8).map((h) => (
                <div key={h.hub} style={rankRow}><span>{h.name}</span><span style={{ color: "#e3c14a" }}>{Math.round(h.amount * 100)}%</span></div>
              ))}
              {atlas.top_quality.length === 0 && <div style={empty}>Not produced anywhere yet.</div>}
            </>
          ) : tab === "trade" ? (
            <>
              <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
                <AtlasStat label="traded/yr" value={fmtn(atlas.total_traded)} />
                <AtlasStat label="produced/yr" value={fmtn(atlas.total_produced)} />
              </div>
              <div style={sectionHdr}>Top producers</div>
              <AtlasBars rows={atlas.producers.map((p) => ({ label: p.name, value: p.amount }))} color="#7fd0a0" fmt={fmtn} />
              <div style={{ ...sectionHdr, marginTop: 8 }}>Consumed most (hotspots)</div>
              <AtlasBars rows={atlas.consumers.map((p) => ({ label: p.name, value: p.amount }))} color="#ff9a6a" fmt={fmtn} />
            </>
          ) : tab === "ctrl" ? (
            <>
              <div style={sectionHdr}>Control — share of this good</div>
              <AtlasBars rows={atlas.houses.map((h) => ({ label: (h.is_guild ? "⚒ " : "🏛 ") + h.name, value: h.share }))} color="#6a9adf" fmt={(v) => `${Math.round(v * 100)}%`} norm={1} />
              <div style={{ ...sectionHdr, marginTop: 10 }}>Who trades the most — total volume</div>
              <AtlasBars rows={[...atlas.houses].sort((a, b) => b.total_volume - a.total_volume).map((h) => ({ label: (h.is_guild ? "⚒ " : "🏛 ") + h.name, value: h.total_volume }))} color="#d9a441" fmt={fmtn} />
              {atlas.houses.length === 0 && <div style={empty}>No house or guild trades this good yet.</div>}
            </>
          ) : (
            <>
              <button onClick={() => setOverlayVisible("goodFlow", !flowOn)} style={{ ...btn, background: flowOn ? "#7a3a3a" : "#2060a0" }}>
                {flowOn ? "Hide flow on map" : "Show flow on map"}
              </button>
              <div style={{ color: "#5a6a80", fontSize: 9, margin: "6px 0 8px" }}>
                Arrows follow the EXISTING trade routes, exporter → importer, width ∝ volume — redrawn each year.
              </div>
              <div style={sectionHdr}>Busiest lanes ({atlas.flows.length})</div>
              {atlas.flows.length === 0 ? <div style={empty}>No shipments of this good last year.</div> :
                atlas.flows.slice(0, 12).map((f, i) => (
                  <div key={i} style={rankRow}>
                    <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{hubName(f.from)} → {hubName(f.to)}</span>
                    <span style={{ color: "#8aa8c8" }}>{fmtn(f.amount)}</span>
                  </div>
                ))}
            </>
          )
        )}

        {/* PROVENANCE */}
        {tab === "prov" && (
          <>
            {/* Live market (refreshed once per campaign year) for the selected good. */}
            {(() => {
              const lg = active ? liveGoods.find((g) => g.name === codexGood) : null;
              if (!lg) return null;
              return (
                <div style={{ marginBottom: 10, padding: "6px 8px", background: "#0d1a14", border: "1px solid #1e3a2a", borderRadius: 6 }}>
                  <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                    <span style={{ color: "#7fd0a0", fontWeight: 700, fontSize: 11 }}>Live market</span>
                    <span style={{ color: "#5a7a66", fontSize: 9 }}>year {year}</span>
                    <span style={{ flex: 1 }} />
                    <span style={{ color: lg.world_price >= 1.3 ? "#e08080" : lg.world_price <= 0.77 ? "#7fd0a0" : "#cfe2f6", fontWeight: 700, fontSize: 12 }}>
                      {lg.world_price.toFixed(2)}×
                    </span>
                  </div>
                  <div style={{ color: "#8aa0b8", fontSize: 9.5, marginTop: 2 }}>
                    world avg price · {lg.producers} producing {lg.producers === 1 ? "city" : "cities"}
                    {lg.top_hub ? <> · top: <span style={{ color: "#cfe2f6" }}>{lg.top_hub}</span></> : null}
                  </div>
                </div>
              );
            })()}
            {spec && spec.distribution === "manufactured" && (spec.inputs?.length ?? 0) > 0 ? (
              <div style={{ marginBottom: 10 }}>
                <div style={sectionHdr}>Made from</div>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 4 }}>
                  {(spec.inputs ?? []).map((inp) => (
                    <button key={inp.good} onClick={() => setCodexGood(inp.good)} style={tag}
                      title="Trace this input good">
                      {meta(inp.good).icon} {meta(inp.good).name}
                      <span style={{ color: "#7fa0c0", marginLeft: 4 }}>×{inp.qty}</span>
                    </button>
                  ))}
                </div>
                <div style={{ color: "#5a6a80", fontSize: 9, marginTop: 4 }}>
                  Manufactured in cities — no map belt of its own. Click an input to trace it.
                </div>
              </div>
            ) : spec ? (
              <div style={{ color: "#8aa0b8", fontSize: 10, marginBottom: 10 }}>
                A raw/extracted good gathered from its climate belt (see Trade-Goods overlay).
              </div>
            ) : null}

            <div style={sectionHdr}>Trade routes ({routes.length})</div>
            {routes.length === 0 && <div style={empty}>No routed shipments for this good.</div>}
            {routes.map((r) => {
              const first = r.stops[0];
              const last = r.stops[r.stops.length - 1];
              return (
                <div key={r.id} style={routeCard}>
                  <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                    <span style={{ color: "#7fd0a0", fontWeight: 600, fontSize: 11 }}>{hubName(first?.hub)}</span>
                    <span style={{ color: "#5a6a80" }}>→</span>
                    <span style={{ color: "#ff9a6a", fontWeight: 600, fontSize: 11 }}>{hubName(last?.hub)}</span>
                    <span style={{ flex: 1 }} />
                    <span style={{ color: "#8aa8c8", fontSize: 9 }}>{Math.round(r.km)} km · {r.days.toFixed(0)}d</span>
                  </div>
                  <div style={{ color: "#9ab0c8", fontSize: 9, marginTop: 2 }}>
                    {r.stops.map((s) => hubName(s.hub)).join(" → ")}
                  </div>
                  <div style={{ display: "flex", gap: 8, fontSize: 9, color: "#7fa0c0", marginTop: 2 }}>
                    <span>origin ×{first?.price.toFixed(2)}</span>
                    <span>delivered ×{last?.price.toFixed(2)}</span>
                  </div>
                </div>
              );
            })}
          </>
        )}

        {/* HISTORY */}
        {tab === "hist" && (
          <div>
            {hist ? (
              <div style={{ background: "#0e1a14", border: "1px solid #1c3326", borderRadius: 6, padding: 12 }}>
                <div style={{ color: "#d8c878", fontSize: 10, fontWeight: 700, letterSpacing: 0.3 }}>{hist.era}</div>
                <div style={{ color: "#cfe2f6", fontSize: 11.5, lineHeight: 1.65, marginTop: 6 }}>{hist.note}</div>
              </div>
            ) : (
              <div style={empty}>
                No historical card for {m?.name ?? "this good"} yet — it trades as a regional commodity in this world.
              </div>
            )}
            <div style={{ color: "#5a6a80", fontSize: 9, marginTop: 8 }}>
              Real-world context for inspiration — your world's story is its own.
            </div>
          </div>
        )}

        {/* SCARCITY */}
        {tab === "scar" && (
          <div>
            <button onClick={toggleScarcity} style={{ ...btn, background: scarcityOn ? "#7a3a3a" : "#2060a0" }}>
              {scarcityOn ? "Hide scarcity overlay" : "Show scarcity on map"}
            </button>
            <div style={{ display: "flex", alignItems: "center", gap: 8, margin: "8px 0", fontSize: 9, color: "#9ab0c8" }}>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><span style={dot("#46d07a")} /> cheap</span>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><span style={dot("#9aa0a0")} /> par</span>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><span style={dot("#e85b5b")} /> dear</span>
            </div>
            {premiums.length === 0 ? (
              <div style={empty}>No price data for this good.</div>
            ) : (
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
                <div>
                  <div style={sectionHdr}>Cheapest</div>
                  {cheapest.map((c) => (
                    <div key={c.name} style={rankRow}><span>{c.name}</span><span style={{ color: "#7fd0a0" }}>×{c.premium.toFixed(2)}</span></div>
                  ))}
                </div>
                <div>
                  <div style={sectionHdr}>Dearest</div>
                  {dearest.map((c) => (
                    <div key={c.name} style={rankRow}><span>{c.name}</span><span style={{ color: "#ff9a6a" }}>×{c.premium.toFixed(2)}</span></div>
                  ))}
                </div>
              </div>
            )}
            <div style={{ color: "#5a6a80", fontSize: 9, marginTop: 8 }}>
              Premium = local price ÷ world base value. A merchant buys low and carries to the dear markets.
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

const dot = (c: string): React.CSSProperties => ({ width: 9, height: 9, borderRadius: "50%", background: c, display: "inline-block" });

const fmtn = (n: number) => Math.round(n).toLocaleString();

/** A compact stat chip for the Atlas facet headers. */
function AtlasStat({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ flex: 1, background: "#0c1622", border: "1px solid #16283c", borderRadius: 5, padding: "4px 6px" }}>
      <div style={{ color: "#cfe2f6", fontWeight: 700, fontSize: 12 }}>{value}</div>
      <div style={{ color: "#6a86a6", fontSize: 8.5 }}>{label}</div>
    </div>
  );
}

/** A ranked horizontal bar list (producers, consumers, control, volume). `norm` fixes
 *  the full-scale value (e.g. 1 for a share); otherwise it's the largest row. */
function AtlasBars({ rows, color, fmt, norm }: {
  rows: { label: string; value: number }[]; color: string; fmt: (v: number) => string; norm?: number;
}) {
  if (rows.length === 0) return <div style={empty}>—</div>;
  const mx = norm ?? Math.max(1e-6, ...rows.map((r) => r.value));
  return (
    <div>
      {rows.slice(0, 10).map((r, i) => (
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

const panel: React.CSSProperties = {
  position: "absolute", top: 60, left: 70, width: 340, maxHeight: "80vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 40,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#cfe0f4", fontWeight: 700, fontSize: 12,
};
const select: React.CSSProperties = {
  flex: 1, background: "#080c12", border: "1px solid #1e2e42", color: "#cfe0f4",
  padding: "3px 5px", borderRadius: 4, fontSize: 11, minWidth: 0,
};
const sectionHdr: React.CSSProperties = { color: "#8aa0b8", fontSize: 10, fontWeight: 600, marginBottom: 2 };
const routeCard: React.CSSProperties = {
  background: "#0c1622", border: "1px solid #16243400", borderBottom: "1px solid #131e2a",
  padding: "5px 2px",
};
const tag: React.CSSProperties = {
  background: "#0c1622", border: "1px solid #1e2e42", color: "#cfe0f4", borderRadius: 5,
  padding: "3px 7px", fontSize: 10, cursor: "pointer",
};
const rankRow: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", fontSize: 10, color: "#bcd0e4", padding: "2px 0",
};
const btn: React.CSSProperties = {
  width: "100%", padding: "6px", color: "#fff", border: "none", borderRadius: 4,
  cursor: "pointer", fontSize: 11, fontWeight: 600,
};
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "8px 0" };
