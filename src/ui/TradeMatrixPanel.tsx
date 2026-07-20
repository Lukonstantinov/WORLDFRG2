import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { computeTradeMatrix, exportTradeData } from "@bridge";
import type { TradeMatrix } from "@types";
import { useGoodsStore } from "@state/goodsStore";

/** Color a net balance: green = surplus/export, red = deficit/import. */
function netColor(v: number): string {
  if (v > 0.05) return `rgba(80, 200, 110, ${Math.min(0.85, 0.25 + v)})`;
  if (v < -0.05) return `rgba(220, 90, 80, ${Math.min(0.85, 0.25 - v)})`;
  return "transparent";
}

type Tab = "regions" | "hubs" | "corridors" | "prices";

export function TradeMatrixPanel() {
  const show = useUIStore((s) => s.showTradeMatrix);
  const setShow = useUIStore((s) => s.setShowTradeMatrix);
  const setStatus = useUIStore((s) => s.setStatus);
  const bioParams = useUIStore((s) => s.bioParams);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const settlements = useWorldStore((s) => s.settlements);
  const rivers = useWorldStore((s) => s.rivers);
  const economy = useWorldStore((s) => s.economy);
  const goodMeta = useGoodsStore((s) => s.meta);
  const emojiFor = (id: string) => goodMeta(id).icon;
  const labelFor = (id: string) => goodMeta(id).name;
  const [matrix, setMatrix] = useState<TradeMatrix | null>(null);
  const [loading, setLoading] = useState(false);
  const [tab, setTab] = useState<Tab>("regions");

  useEffect(() => {
    if (!show) return;
    setLoading(true);
    computeTradeMatrix(
      settlements.map((s) => ({ x: s.x, y: s.y, score: s.score })),
      rivers.map((r) => ({ points: r.points })),
      bioParams.tradeReach,
      bioParams.maxCrossing,
      bioParams.desertRoutes,
      bioParams.economicRegions,
      bioParams.luxuryBias,
      bioParams.piracyLevel,
    )
      .then(setMatrix)
      .catch(() => setMatrix(null))
      .finally(() => setLoading(false));
  }, [show, settlements, rivers, bioParams.tradeReach, bioParams.maxCrossing, bioParams.desertRoutes, bioParams.economicRegions, bioParams.luxuryBias, bioParams.piracyLevel]);

  // Per-good price dynamics: cheapest origin → most expensive market + its hub.
  const priceRows = useMemo(() => {
    if (!economy) return [];
    const rows = economy.goods.map((id, gi) => {
      let origin = Infinity; let maxP = 0; let maxHub = "";
      for (const h of economy.hubs) {
        for (const p of h.produces) if (p.good === gi && p.price < origin) origin = p.price;
        for (const r of h.receives) if (r.good === gi && r.price > maxP) { maxP = r.price; maxHub = h.name; }
      }
      return { id, gi, origin: isFinite(origin) ? origin : 0, maxP, maxHub };
    }).filter((r) => r.maxP > 0 || r.origin > 0);
    rows.sort((a, b) => b.maxP - a.maxP);
    return rows;
  }, [economy]);

  const hubsByPower = useMemo(
    () => (economy ? [...economy.hubs].sort((a, b) => b.power - a.power) : []),
    [economy],
  );

  const handleExport = async () => {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const path = await save({
        defaultPath: "trade.csv",
        filters: [{ name: "CSV", extensions: ["csv"] }, { name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      await exportTradeData(path);
      setStatus(`Trade data exported to ${path}`);
    } catch (e) { setStatus(`Export failed: ${e}`); }
  };

  if (!show) return null;

  const hasEconomy = !!economy && economy.hubs.length > 0;

  return (
    <div style={{
      position: "absolute", inset: 0, display: "flex",
      alignItems: "center", justifyContent: "center",
      background: "rgba(0,0,0,0.7)", zIndex: 120,
    }}>
      <div style={{
        background: "#0e141d", border: "1px solid #1e2e42", borderRadius: 10,
        padding: "18px 20px", maxWidth: "92%", maxHeight: "88%", overflow: "auto",
        boxShadow: "0 12px 40px rgba(0,0,0,0.55)", minWidth: 360,
      }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
          <h2 style={{ margin: 0, color: "#c0d8f0", fontSize: 16, fontWeight: 600 }}>Global Trade</h2>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            {hasEconomy && (
              <button onClick={handleExport} style={{ ...tabBtn, background: "#16324a", color: "#9cc4e4" }}
                title="Export the economy as CSV or JSON">⤓ Export</button>
            )}
            <span onClick={() => setShow(false)}
              style={{ color: "#7090b0", cursor: "pointer", fontSize: 18, lineHeight: 1 }} title="Close">×</span>
          </div>
        </div>

        {/* Tabs */}
        <div style={{ display: "flex", gap: 4, marginBottom: 10 }}>
          {(["regions", "hubs", "corridors", "prices"] as Tab[]).map((t) => (
            <button key={t} onClick={() => setTab(t)} style={{
              ...tabBtn,
              background: tab === t ? "#1e3a58" : "#0b1320",
              color: tab === t ? "#dce6f0" : "#6a86a6",
            }}>{t[0].toUpperCase() + t.slice(1)}</button>
          ))}
        </div>

        {tab === "regions" && (
          <>
            {loading && <div style={{ color: "#5a7898", fontSize: 12 }}>Computing…</div>}
            {!loading && (!matrix || matrix.regions.length === 0) && (
              <div style={{ color: "#7a6a4a", fontSize: 12, maxWidth: 360 }}>
                No trade regions yet. Generate settlements (Step 7) and the biological
                layer (Step 8) first — regions are formed by clustering settlements.
              </div>
            )}
            {!loading && matrix && matrix.regions.length > 0 && (
              <>
                <div style={{ color: "#5a7898", fontSize: 11, marginBottom: 8 }}>
                  Net balance per good (green = export surplus, red = import demand).
                  {matrix.regions.length} regions, {matrix.flows.length} active flows.
                </div>
                <table style={{ borderCollapse: "collapse", fontSize: 11 }}>
                  <thead>
                    <tr>
                      <th style={{ ...th, textAlign: "left", position: "sticky", left: 0, background: "#0e141d" }}>Region</th>
                      {matrix.goods.map((g) => (
                        <th key={g} style={th} title={labelFor(g)}>{emojiFor(g)}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {matrix.regions.map((r) => (
                      <tr key={r.id}>
                        <td style={{ ...td, textAlign: "left", color: "#a8c0d8", whiteSpace: "nowrap", position: "sticky", left: 0, background: "#0e141d" }}>{r.name}</td>
                        {r.net.map((v, gi) => (
                          <td key={gi} style={{ ...td, background: netColor(v), color: "#dce6f0" }}
                            title={`${labelFor(matrix.goods[gi])}: prod ${r.production[gi].toFixed(2)} / dem ${r.demand[gi].toFixed(2)}`}>
                            {Math.abs(v) < 0.05 ? "" : (v > 0 ? "+" : "") + v.toFixed(1)}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </>
            )}
          </>
        )}

        {tab !== "regions" && !hasEconomy && (
          <div style={{ color: "#7a6a4a", fontSize: 12, maxWidth: 380 }}>
            No economy yet. Run the <b>Economy</b> step (10) to compute hubs,
            prices, wealth and chokepoints.
          </div>
        )}

        {tab === "hubs" && hasEconomy && (
          <table style={{ borderCollapse: "collapse", fontSize: 11, width: "100%" }}>
            <thead><tr>
              <th style={{ ...th, textAlign: "left" }}>Hub</th>
              <th style={th}>Power</th><th style={th}>Wealth</th><th style={th}>Pop</th>
              <th style={{ ...th, textAlign: "left" }}>Top exports</th>
            </tr></thead>
            <tbody>
              {hubsByPower.slice(0, 40).map((h) => (
                <tr key={h.id} onClick={() => { setSelectedHub(h.id); setShow(false); }}
                  style={{ cursor: "pointer" }}>
                  <td style={{ ...td, textAlign: "left", color: "#dce6f0", whiteSpace: "nowrap" }}>
                    <span style={{ color: "#ffd24a" }}>{"★".repeat(Math.max(1, Math.min(5, h.stars)))}</span> {h.name}
                  </td>
                  <td style={td}>{h.power.toFixed(2)}</td>
                  <td style={td}>{Math.round(h.wealth * 100)}%</td>
                  <td style={td}>{h.population.toLocaleString()}</td>
                  <td style={{ ...td, textAlign: "left" }}>
                    {h.produces.slice(0, 5).map((p) => emojiFor(p.good_name)).join(" ")}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {tab === "corridors" && hasEconomy && (
          <>
            <div style={{ color: "#5a7898", fontSize: 11, marginBottom: 8 }}>
              Strategic chokepoints — the busiest straits &amp; passes, by share of all routed volume.
            </div>
            <table style={{ borderCollapse: "collapse", fontSize: 11, width: "100%" }}>
              <thead><tr>
                <th style={{ ...th, textAlign: "left" }}>Gateway</th>
                <th style={th}>Share</th><th style={th}>Volume</th>
              </tr></thead>
              <tbody>
                {economy!.chokepoints.map((cp, i) => (
                  <tr key={i}>
                    <td style={{ ...td, textAlign: "left", color: "#e0c0a0" }}>⚓ {cp.name}</td>
                    <td style={td}>{Math.round(cp.share * 100)}%</td>
                    <td style={td}>{cp.volume.toFixed(1)}</td>
                  </tr>
                ))}
                {economy!.chokepoints.length === 0 && (
                  <tr><td style={{ ...td, color: "#7a6a4a" }} colSpan={3}>No dominant chokepoints — trade is well distributed.</td></tr>
                )}
              </tbody>
            </table>
          </>
        )}

        {tab === "prices" && hasEconomy && (
          <>
            <div style={{ color: "#5a7898", fontSize: 11, marginBottom: 8 }}>
              Price dynamics — cheapest at source, dearest at the market that pays most.
            </div>
            <table style={{ borderCollapse: "collapse", fontSize: 11, width: "100%" }}>
              <thead><tr>
                <th style={{ ...th, textAlign: "left" }}>Good</th>
                <th style={th}>Origin</th><th style={th}>Peak</th>
                <th style={{ ...th, textAlign: "left" }}>Dearest market</th>
                <th style={th}>×</th>
              </tr></thead>
              <tbody>
                {priceRows.map((r) => (
                  <tr key={r.gi}>
                    <td style={{ ...td, textAlign: "left", color: "#dce6f0", whiteSpace: "nowrap" }}>
                      {emojiFor(r.id)} {labelFor(r.id)}
                    </td>
                    <td style={{ ...td, color: "#80dc8c" }}>{r.origin > 0 ? r.origin.toFixed(1) + "×" : "—"}</td>
                    <td style={{ ...td, color: "#ff9a6a" }}>{r.maxP > 0 ? r.maxP.toFixed(1) + "×" : "—"}</td>
                    <td style={{ ...td, textAlign: "left", color: "#c0a890" }}>{r.maxHub || "—"}</td>
                    <td style={{ ...td, color: "#e0c060" }}>{r.origin > 0 && r.maxP > 0 ? (r.maxP / r.origin).toFixed(1) : "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </>
        )}
      </div>
    </div>
  );
}

const th: React.CSSProperties = {
  padding: "4px 6px", color: "#6a86a6", fontWeight: 600, fontSize: 12,
  borderBottom: "1px solid #1e2e42", textAlign: "center",
};
const td: React.CSSProperties = {
  padding: "3px 6px", textAlign: "center", borderBottom: "1px solid #14202e",
  fontFamily: "monospace",
};
const tabBtn: React.CSSProperties = {
  padding: "3px 10px", fontSize: 11, border: "1px solid #1e2e42", borderRadius: 5,
  cursor: "pointer", fontWeight: 600,
};
