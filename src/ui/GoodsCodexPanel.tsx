import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useGoodsStore } from "../state/goodsStore";
import type { EconChain } from "../types";
import { commodityHistory } from "../commodityHistory";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

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
  const economy = useWorldStore((s) => s.economy);
  const specs = useGoodsStore((s) => s.specs);
  const meta = useGoodsStore((s) => s.meta);

  const [tab, setTab] = useState<"prov" | "hist" | "scar">("prov");

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
  const close = () => { setOverlayVisible("goodScarcity", false); useUIStore.getState().setShowGoodsCodex(false); };

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
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>📖 Goods Codex</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>

      {/* Good picker */}
      <div style={{ padding: "8px 10px 6px", display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ fontSize: 14 }}>{m?.icon ?? "📦"}</span>
        <select value={codexGood ?? ""} onChange={(e) => setCodexGood(e.target.value || null)} style={select}>
          {goodIds.map((id) => (
            <option key={id} value={id}>{meta(id).name}</option>
          ))}
        </select>
      </div>

      {/* Tabs */}
      <div style={{ display: "flex", gap: 2, padding: "0 8px", borderBottom: "1px solid #1e2e42" }}>
        {([["prov", "🧭 Provenance"], ["hist", "📜 History"], ["scar", "⚖ Scarcity"]] as const).map(([id, lbl]) => (
          <div key={id} onClick={() => setTab(id)}
            style={{ padding: "4px 8px", cursor: "pointer", fontSize: 10.5, fontWeight: tab === id ? 700 : 400,
              color: tab === id ? "#cfe2f6" : "#6a86a6",
              borderBottom: tab === id ? "2px solid #6a9adf" : "2px solid transparent" }}>
            {lbl}
          </div>
        ))}
      </div>

      <div style={{ overflowY: "auto", padding: "8px 10px 12px", maxHeight: "60vh" }}>
        {!economy && tab !== "hist" && (
          <div style={empty}>Run the Economy step (10) so prices and trade routes are available.</div>
        )}

        {/* PROVENANCE */}
        {tab === "prov" && (
          <>
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
