import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { useGoodsStore } from "@state/goodsStore";
import { campaignWarehouses } from "@bridge";
import type { WarehouseInfo } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";
import { T, FZ, RADIUS } from "./chronicleTheme";
import { Panel, PanelHeader, PanelBody, Meter, EmptyNote, FootNote } from "./kit";

const TIER_NAME = ["pool", "Depot", "Storehouse", "Warehouse", "Entrepôt", "Grand Entrepôt"];
const KIND_ICON: Record<string, string> = {
  warehouse: "🏬", farm: "🌾", mine: "⛏️", plantation: "🌴", fishery: "🎣",
  vineyard: "🍇", manufactory: "🏭", estate: "🏛️",
};

/** The Warehouses infographic — every house/guild depot in the world: where it sits,
 *  its tier & fill, the goods it holds and how many futures contracts it supplies.
 *  Click a depot to FOCUS its distribution: the 📜 Futures layer highlights every
 *  contract lane it sources (its outbound supply network), fading the rest.
 *  Built on the shared UI kit (src/ui/kit.tsx). */
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

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.warehouses);
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
    <Panel width={312} maxHeight="78vh" style={{ top: 60, right: 680, zIndex: 42, ...rootStyle }}>
      <PanelHeader icon="🏬" title="Warehouses & Estates" onDragStart={onPointerDown} onClose={() => setOpen(false)} />
      <div style={{ padding: "6px 8px", borderBottom: `1px solid ${T.line}`, flex: "0 0 auto" }}>
        <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="filter house / city / good…"
          data-no-drag
          style={{ width: "100%", boxSizing: "border-box", background: T.card, border: `1px solid ${T.line}`, borderRadius: RADIUS.sm, color: T.ink, fontSize: FZ.body, padding: "3px 6px" }} />
      </div>
      <PanelBody style={{ padding: "2px 6px 10px" }}>
        {!active && <EmptyNote>Begin the campaign (Step 11) to see warehouses.</EmptyNote>}
        {active && filtered.length === 0 && <EmptyNote>No house warehouses yet.</EmptyNote>}
        {filtered.map((r, i) => {
          const fill = r.capacity > 0 ? Math.min(1, r.used / r.capacity) : 0;
          return (
            <div key={i} onClick={() => focusDepot(r)}
              title="Focus this depot's contract distribution on the map"
              style={{ padding: "4px 2px", borderBottom: `1px solid ${T.lineSoft}`, cursor: "pointer" }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
                <span style={{ width: 8, height: 8, borderRadius: 2, background: r.color, alignSelf: "center" }} />
                <span style={{ color: T.parchment, fontSize: FZ.body, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 120 }}>{r.owner}</span>
                {r.is_guild && <span style={{ fontSize: FZ.micro, color: T.goodInk }}>GUILD</span>}
                <span style={{ flex: 1 }} />
                {r.contracts > 0 && <span style={{ color: T.gold, fontSize: FZ.tiny }} title="futures contracts supplied">📜 {r.contracts}</span>}
                {r.damage > 0.05 && <span style={{ color: T.badInk, fontSize: FZ.tiny }} title="storm/fire damage">🔥</span>}
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: FZ.small, color: T.inkMid, marginTop: 1 }}>
                <span style={{ color: T.ink }}>{r.city}</span>
                <span style={{ color: T.inkDim }}>
                  · {KIND_ICON[r.kind] ?? "🏬"} {r.kind === "warehouse"
                    ? `T${r.tier} ${TIER_NAME[r.tier] ?? ""}`
                    : `${r.kind}${r.tier > 0 ? ` T${r.tier}` : ""}`}
                </span>
              </div>
              {r.capacity > 0 ? (
                <div style={{ display: "flex", alignItems: "center", gap: 5, marginTop: 2 }}>
                  <Meter value={fill} color={fill > 0.85 ? T.warn : T.accent} height={5} />
                  <span style={{ color: T.inkMid, fontSize: FZ.tiny, minWidth: 76, textAlign: "right" }}>{fmt(r.used)} / {fmt(r.capacity)}</span>
                </div>
              ) : (
                <div style={{ fontSize: FZ.tiny, color: T.goodInk, marginTop: 1 }}>produces {fmt(r.used)}/yr</div>
              )}
              {r.goods.length > 0 && (
                <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 2 }}>
                  {r.goods.slice(0, 6).map(([g, v]) => (
                    <span key={g} style={{ fontSize: FZ.tiny, color: T.inkMid }}>{goodMeta(g).icon} {fmt(v)}</span>
                  ))}
                </div>
              )}
            </div>
          );
        })}
        {filtered.length > 0 && (
          <FootNote style={{ fontSize: FZ.tiny }}>
            {filtered.length} depots · click one to highlight its futures distribution
          </FootNote>
        )}
      </PanelBody>
    </Panel>
  );
}
