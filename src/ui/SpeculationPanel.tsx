import { useEffect, useState } from "react";
import { useCampaignStore } from "../state/campaignStore";
import { useUIStore } from "../state/uiStore";
import { campaignGetSpeculation, campaignGetPoleis } from "../bridge/tauri";
import type { SpecCenter, PolisBrief } from "../types";

/** DLC 3 · Finance, the Polis & Speculation.
 *  Two tabs:
 *   • Speculation — the once-a-year bubble-risk read with the generated causal
 *     reason-chain per polis (the why-engine output), and
 *   • Poleis — each seat city as an actor: treasury, council-set tariffs, mint
 *     fineness and the governing house.
 *  Both read straight from the live campaign sim. */
export function SpeculationPanel() {
  const open = useUIStore((s) => s.showSpeculation);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;
  const [tab, setTab] = useState<"spec" | "poleis">("spec");
  const [centers, setCenters] = useState<SpecCenter[]>([]);
  const [poleis, setPoleis] = useState<PolisBrief[]>([]);

  useEffect(() => {
    if (!open || !active) return;
    campaignGetSpeculation().then(setCenters).catch(() => setCenters([]));
    campaignGetPoleis().then(setPoleis).catch(() => setPoleis([]));
  }, [open, active, tick]);

  if (!open) return null;
  const close = () => useUIStore.getState().setShowSpeculation(false);
  // Turn the matching map overlay on when the user opens the Speculation tab.
  const showRiskOverlay = () => useUIStore.getState().setOverlayVisible("speculation", true);

  return (
    <div style={panel}>
      <div style={header}>
        <span>🫧 Finance · the Polis &amp; Speculation</span>
        <span style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>
      <div style={{ display: "flex", gap: 2, padding: "0 8px", borderBottom: "1px solid #1e2e42" }}>
        {([["spec", "🫧 Speculation"], ["poleis", "🏛 Poleis"]] as const).map(([id, lbl]) => (
          <div key={id} onClick={() => setTab(id)}
            style={{ padding: "4px 9px", cursor: "pointer", fontSize: 11, fontWeight: tab === id ? 700 : 400,
              color: tab === id ? "#cfe2f6" : "#6a86a6",
              borderBottom: tab === id ? "2px solid #3a80c0" : "2px solid transparent" }}>
            {lbl}
          </div>
        ))}
      </div>

      {!active && <div style={empty}>Begin the campaign (Step 11) and let it run — the speculation read is computed once a year.</div>}

      {active && tab === "spec" && (
        <div style={{ overflowY: "auto", padding: "6px 8px 10px" }}>
          {centers.length === 0 && (
            <div style={empty}>No reading yet — advance the campaign past its first New Year.</div>
          )}
          {centers.length > 0 && (
            <div style={{ fontSize: 9, color: "#6a86a6", marginBottom: 6, display: "flex", justifyContent: "space-between" }}>
              <span>Year {centers[0]?.year} · top {centers.length} at-risk poleis</span>
              <span style={{ cursor: "pointer", color: "#7fb0e0" }} onClick={showRiskOverlay}>show on map</span>
            </div>
          )}
          {centers.map((c) => <SpecCard key={c.hub} c={c} />)}
        </div>
      )}

      {active && tab === "poleis" && (
        <div style={{ overflowY: "auto", padding: "6px 8px 10px" }}>
          {poleis.length === 0 && <div style={empty}>No poleis yet.</div>}
          {poleis.map((p) => <PolisCard key={p.hub} p={p} />)}
        </div>
      )}
    </div>
  );
}

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
      {/* Ranked causal reason-chain — the generated "why". */}
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
        <div style={{ color: "#e0b060", fontSize: 9, marginTop: 3 }}>
          Watch: {c.watch_goods.join(", ")}
        </div>
      )}
    </div>
  );
}

function PolisCard({ p }: { p: PolisBrief }) {
  const debased = p.mint_fineness < 0.999;
  return (
    <div style={card}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
        <span style={{ width: 9, height: 9, borderRadius: 2, background: p.council_color, alignSelf: "center", flex: "0 0 auto" }} />
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12 }}>{p.name}</span>
        <span style={{ color: "#6a86a6", fontSize: 9 }}>· {p.population.toLocaleString()}</span>
        <span style={{ flex: 1 }} />
        <span style={{ color: "#d8c878", fontSize: 10 }} title="Retained civic treasury">🏛 {fmtk(p.treasury)}</span>
      </div>
      <div style={{ color: "#9ab0c8", fontSize: 10, marginTop: 1 }}>
        Council: <span style={{ color: "#cfe0f4" }}>{p.council}</span>
        {p.council_archetype ? <span style={{ color: "#7fa0c0" }}> · {p.council_archetype}</span> : null}
      </div>
      <div style={{ display: "flex", gap: 10, fontSize: 9, color: "#8aa8c8", marginTop: 2 }}>
        <span title="Export tariff levied here">⬆ tariff {(p.tariff_export * 100).toFixed(1)}%</span>
        <span title="Import tariff levied here">⬇ tariff {(p.tariff_import * 100).toFixed(1)}%</span>
        <span title="Mint fineness — below 100% is a debased 'cheap money' coinage"
          style={{ color: debased ? "#e0a020" : "#8aa8c8" }}>
          🪙 mint {(p.mint_fineness * 100).toFixed(0)}%{debased ? " (debased)" : ""}
        </span>
      </div>
    </div>
  );
}

const fmtk = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 340, maxHeight: "78vh",
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
  display: "flex", flexDirection: "column", padding: "6px 4px",
  borderBottom: "1px solid #131e2a",
};
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "12px 10px" };
