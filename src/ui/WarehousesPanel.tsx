import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { useGoodsStore } from "../state/goodsStore";
import { campaignWarehouses } from "../bridge/tauri";
import type { WarehouseInfo } from "../types";

const TIER_NAME = ["pool", "Depot", "Storehouse", "Warehouse", "Entrepôt", "Grand Entrepôt"];
const KIND_ICON: Record<string, string> = {
  warehouse: "🏬", farm: "🌾", mine: "⛏️", plantation: "🌴", fishery: "🎣",
  vineyard: "🍇", manufactory: "🏭", estate: "🏛️",
};

/** The Warehouses infographic — every house/guild depot in the world: where it sits,
 *  its tier & fill, the goods it holds and how many futures contracts it supplies.
 *  Click a depot to FOCUS its distribution: the 📜 Futures layer highlights every
 *  contract lane it sources (its outbound supply network), fading the rest. */
export function WarehousesPanel() {
  const open = useUIStore((s) => s.showWarehouses);
  const setOpen = useUIStore((s) => s.setShowWarehouses);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const setFocus = useUIStore((s) => s.setFuturesFocus);
  const setSelectedLane = useUIStore((s) => s.setSelectedFuturesLane);
  const active = useCampaignStore((s) => s.snapshot?.active ?? false);
  const tick = useCampaignStore((s) => s.snapshot?.clock.tick ?? 0);
  const goodMeta = useGoodsStore((s) => s.meta);
  const [rows, setRows] = useState<WarehouseInfo[]>([]);
  const [q, setQ] = useState("");

  useEffect(() => {
    if (!open || !active) return;
    let alive = true;
    campaignWarehouses().then((r) => { if (alive) setRows(r); }).catch(() => {});
    return () => { alive = false; };
  }, [open, active, tick]);

  const filtered = useMemo(() => {
    const s = q.trim().toLowerCase();
    if (!s) return rows;
    return rows.filter((r) => r.owner.toLowerCase().includes(s) || r.city.toLowerCase().includes(s)
      || r.goods.some(([g]) => g.toLowerCase().includes(s)));
  }, [rows, q]);

  if (!open) return null;
  const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));

  const focusDepot = (r: WarehouseInfo) => {
    setSelectedLane(null);
    // A warehouse → focus its city's lanes for that house; an estate (no capacity)
    // → focus the whole house's distribution (its city label carries a suffix).
    setFocus(r.capacity > 0 ? { holder: r.owner, city: r.city } : { holder: r.owner });
    setOverlayVisible("futures", true);
  };

  return (
    <div style={panel}>
      <div style={header}>
        <span>🏬 Warehouses & Estates</span>
        <span style={{ cursor: "pointer", color: "#7a90a8" }} onClick={() => setOpen(false)}>✕</span>
      </div>
      <div style={{ padding: "6px 8px", borderBottom: "1px solid #1a2a3e" }}>
        <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="filter house / city / good…"
          style={{ width: "100%", boxSizing: "border-box", background: "#0a1018", border: "1px solid #23364c", borderRadius: 5, color: "#cfe0f4", fontSize: 11, padding: "3px 6px" }} />
      </div>
      <div style={{ overflowY: "auto", padding: "2px 6px 10px" }}>
        {!active && <div style={hint}>Begin the campaign (Step 11) to see warehouses.</div>}
        {active && filtered.length === 0 && <div style={hint}>No house warehouses yet.</div>}
        {filtered.map((r, i) => {
          const fill = r.capacity > 0 ? Math.min(1, r.used / r.capacity) : 0;
          return (
            <div key={i} onClick={() => focusDepot(r)}
              title="Focus this depot's contract distribution on the map"
              style={{ padding: "4px 2px", borderBottom: "1px solid #131e2a", cursor: "pointer" }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
                <span style={{ width: 8, height: 8, borderRadius: 2, background: r.color, alignSelf: "center" }} />
                <span style={{ color: "#e8d8b0", fontSize: 11, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 120 }}>{r.owner}</span>
                {r.is_guild && <span style={{ fontSize: 8, color: "#7fd0c0" }}>GUILD</span>}
                <span style={{ flex: 1 }} />
                {r.contracts > 0 && <span style={{ color: "#ffcf3f", fontSize: 9 }} title="futures contracts supplied">📜 {r.contracts}</span>}
                {r.damage > 0.05 && <span style={{ color: "#ff7a6a", fontSize: 9 }} title="storm/fire damage">🔥</span>}
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 10, color: "#9ab0c8", marginTop: 1 }}>
                <span style={{ color: "#cfe0f4" }}>{r.city}</span>
                <span style={{ color: "#6a86a6" }}>
                  · {KIND_ICON[r.kind] ?? "🏬"} {r.kind === "warehouse"
                    ? `T${r.tier} ${TIER_NAME[r.tier] ?? ""}`
                    : `${r.kind}${r.tier > 0 ? ` T${r.tier}` : ""}`}
                </span>
              </div>
              {r.capacity > 0 ? (
                <div style={{ display: "flex", alignItems: "center", gap: 5, marginTop: 2 }}>
                  <div style={{ flex: 1, height: 5, background: "#0a1018", borderRadius: 3, overflow: "hidden" }}>
                    <div style={{ width: `${fill * 100}%`, height: "100%", background: fill > 0.85 ? "#e0a020" : "#5a9bd0" }} />
                  </div>
                  <span style={{ color: "#7a90a8", fontSize: 9, minWidth: 76, textAlign: "right" }}>{fmt(r.used)} / {fmt(r.capacity)}</span>
                </div>
              ) : (
                <div style={{ fontSize: 9, color: "#7fb88a", marginTop: 1 }}>produces {fmt(r.used)}/yr</div>
              )}
              {r.goods.length > 0 && (
                <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 2 }}>
                  {r.goods.slice(0, 6).map(([g, v]) => (
                    <span key={g} style={{ fontSize: 9, color: "#b8c8d8" }}>{goodMeta(g).icon} {fmt(v)}</span>
                  ))}
                </div>
              )}
            </div>
          );
        })}
        {filtered.length > 0 && (
          <div style={{ color: "#56708e", fontSize: 9, marginTop: 5 }}>
            {filtered.length} depots · click one to highlight its futures distribution
          </div>
        )}
      </div>
    </div>
  );
}

const hint: React.CSSProperties = { color: "#506080", fontSize: 11, padding: 10 };
const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 680, width: 312, maxHeight: "78vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #243a2e", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 42,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#9fe0b0", fontWeight: 700, fontSize: 12,
};
