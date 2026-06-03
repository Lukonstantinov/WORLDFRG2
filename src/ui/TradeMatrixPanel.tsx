import { useEffect, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { computeTradeMatrix } from "../bridge/tauri";
import type { TradeMatrix } from "../types";
import { GOOD_DEFS } from "../goods";

const emojiFor = (name: string) => GOOD_DEFS.find((g) => g.name === name)?.emoji ?? "";
const labelFor = (name: string) => GOOD_DEFS.find((g) => g.name === name)?.label ?? name;

/** Color a net balance: green = surplus/export, red = deficit/import. */
function netColor(v: number): string {
  if (v > 0.05) return `rgba(80, 200, 110, ${Math.min(0.85, 0.25 + v)})`;
  if (v < -0.05) return `rgba(220, 90, 80, ${Math.min(0.85, 0.25 - v)})`;
  return "transparent";
}

export function TradeMatrixPanel() {
  const show = useUIStore((s) => s.showTradeMatrix);
  const setShow = useUIStore((s) => s.setShowTradeMatrix);
  const bioParams = useUIStore((s) => s.bioParams);
  const settlements = useWorldStore((s) => s.settlements);
  const rivers = useWorldStore((s) => s.rivers);
  const [matrix, setMatrix] = useState<TradeMatrix | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!show) return;
    setLoading(true);
    computeTradeMatrix(
      settlements.map((s) => ({ x: s.x, y: s.y, score: s.score })),
      rivers.map((r) => ({ points: r.points })),
      bioParams.tradeReach,
      bioParams.maxCrossing,
    )
      .then(setMatrix)
      .catch(() => setMatrix(null))
      .finally(() => setLoading(false));
  }, [show, settlements, rivers, bioParams.tradeReach, bioParams.maxCrossing]);

  if (!show) return null;

  return (
    <div style={{
      position: "absolute", inset: 0, display: "flex",
      alignItems: "center", justifyContent: "center",
      background: "rgba(0,0,0,0.7)", zIndex: 120,
    }}>
      <div style={{
        background: "#0e141d", border: "1px solid #1e2e42", borderRadius: 10,
        padding: "18px 20px", maxWidth: "92%", maxHeight: "88%", overflow: "auto",
        boxShadow: "0 12px 40px rgba(0,0,0,0.55)",
      }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
          <h2 style={{ margin: 0, color: "#c0d8f0", fontSize: 16, fontWeight: 600 }}>
            Global Trade Matrix
          </h2>
          <span onClick={() => setShow(false)}
            style={{ color: "#7090b0", cursor: "pointer", fontSize: 18, lineHeight: 1 }} title="Close">
            {"×"}
          </span>
        </div>

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
                    <td style={{ ...td, textAlign: "left", color: "#a8c0d8", whiteSpace: "nowrap", position: "sticky", left: 0, background: "#0e141d" }}>
                      {r.name}
                    </td>
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
