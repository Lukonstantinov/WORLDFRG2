import { useMemo, useRef, useState, useEffect } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useGoodsStore } from "../state/goodsStore";
import { useViewportStore } from "../state/viewportStore";
import { goodCategory, CATEGORY_ORDER } from "../goods";

/** Browsable catalogue of EVERY trade good, opened from the toolbar (independent
 *  of clicking the map). Pick a good → every place it originates (each producing
 *  hub) with its local price/quality → click an origin to see where it ships and
 *  the delivered price at each market. */
export function GoodsBrowserPanel() {
  const open = useUIStore((s) => s.showGoodsBrowser);
  const setOpen = useUIStore((s) => s.setShowGoodsBrowser);
  const setSelectedGood = useUIStore((s) => s.setSelectedGood);
  const economy = useWorldStore((s) => s.economy);
  const goodMeta = useGoodsStore((s) => s.meta);
  const focusOn = useViewportStore((s) => s.focusOn);

  const [good, setGood] = useState<string | null>(null);
  const [origin, setOrigin] = useState<number | null>(null);
  const [query, setQuery] = useState("");

  const [pos, setPos] = useState({ x: 90, y: 90 });
  const dragRef = useRef<{ dx: number; dy: number } | null>(null);
  useEffect(() => {
    const move = (e: PointerEvent) => {
      if (!dragRef.current) return;
      setPos({ x: e.clientX - dragRef.current.dx, y: e.clientY - dragRef.current.dy });
    };
    const up = () => { dragRef.current = null; };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => { window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up); };
  }, []);

  // All good ids: prefer the world's economy snapshot (ground truth of what was
  // generated), else fall back to the goods metadata library.
  const allGoods: string[] = useMemo(() => {
    if (economy && economy.goods.length > 0) return economy.goods;
    return [];
  }, [economy]);

  const hubName = (id: number) => economy?.hubs.find((h) => h.id === id)?.name ?? `Hub ${id}`;

  // Origins of the selected good = producing hubs, richest first.
  const origins = useMemo(() => {
    if (!economy || !good) return [];
    return economy.hubs
      .map((h) => ({ hub: h, p: h.produces.find((pp) => pp.good_name === good) }))
      .filter((o) => o.p)
      .sort((a, b) => (b.p!.amount - a.p!.amount));
  }, [economy, good]);

  // Where the selected origin ships the good (delivered prices per market).
  const shipments = useMemo(() => {
    if (!economy || !good || origin === null) return [];
    return economy.chains
      .filter((c) => c.good_name === good && c.stops.length > 0 && c.stops[0].hub === origin)
      .sort((a, b) => b.value - a.value)
      .slice(0, 20);
  }, [economy, good, origin]);

  if (!open) return null;

  return (
    <div style={{ ...panel, left: pos.x, top: pos.y }}>
      <div onPointerDown={(e) => { dragRef.current = { dx: e.clientX - pos.x, dy: e.clientY - pos.y }; }} style={header}>
        <span>📖 Trade Goods — by origin</span>
        <span onClick={() => setOpen(false)} style={closeBtn} title="Close">×</span>
      </div>

      {!economy && <div style={emptyTxt}>Run the Economy step (10) first to see origins & prices.</div>}

      {economy && (
        <div style={{ display: "flex", gap: 8 }}>
          {/* Left column: searchable, categorized good list */}
          <div style={{ width: 150, flex: "0 0 auto" }}>
            <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search…"
              style={{ width: "100%", boxSizing: "border-box", marginBottom: 4, padding: "2px 5px",
                background: "#0b1622", border: "1px solid #24364e", borderRadius: 4, color: "#cfe0f4", fontSize: 11 }} />
            <div style={{ maxHeight: 320, overflowY: "auto" }}>
              {CATEGORY_ORDER.map((cat) => {
                const items = allGoods.filter((g) => goodCategory(g) === cat
                  && goodMeta(g).name.toLowerCase().includes(query.toLowerCase()));
                if (items.length === 0) return null;
                return (
                  <div key={cat}>
                    <div style={catHdr}>{cat}</div>
                    {items.map((g) => (
                      <div key={g} onClick={() => { setGood(g); setOrigin(null); setSelectedGood(g); }}
                        style={{ ...goodRow, background: good === g ? "#1a2c40" : "transparent" }}>
                        <span>{goodMeta(g).icon}</span>
                        <span style={{ flex: 1, color: good === g ? "#e8d8b0" : "#c0d0e0",
                          overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{goodMeta(g).name}</span>
                      </div>
                    ))}
                  </div>
                );
              })}
            </div>
          </div>

          {/* Right column: origins of the selected good + per-origin shipments */}
          <div style={{ flex: 1, minWidth: 220 }}>
            {!good && <div style={emptyTxt}>Pick a good to see every place it comes from.</div>}
            {good && (
              <>
                <div style={sectionHdr}>{goodMeta(good).icon} {goodMeta(good).name} — origins ({origins.length})</div>
                {origins.length === 0 && <div style={emptyTxt}>Not produced anywhere in this world.</div>}
                <div style={{ maxHeight: origin !== null ? 130 : 300, overflowY: "auto" }}>
                  {origins.map(({ hub, p }) => (
                    <div key={hub.id}
                      onClick={() => { setOrigin(origin === hub.id ? null : hub.id); focusOn(hub.x, hub.y); }}
                      style={{ ...row, cursor: "pointer", background: origin === hub.id ? "#1a2c40" : "transparent" }}>
                      <span style={{ flex: 1, color: origin === hub.id ? "#e8d8b0" : "#c0d0e0",
                        overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {hub.emporium ? "🔴 " : ""}{hub.name}
                        {hub.top_export === good && (
                          <span title="This city's most valuable trade" style={{ marginLeft: 4 }}>💰</span>
                        )}
                      </span>
                      <span style={{ color: "#9ab0c8", fontSize: 9 }}>{p!.grade}</span>
                      <span style={{ color: "#e0c060", fontSize: 10, minWidth: 36, textAlign: "right" }}
                        title="local wholesale price (× base)">{p!.price.toFixed(2)}×</span>
                    </div>
                  ))}
                </div>

                {origin !== null && (
                  <>
                    <div style={{ ...sectionHdr, marginTop: 6 }}>Ships from {hubName(origin)} →</div>
                    {shipments.length === 0 && <div style={emptyTxt}>Consumed locally — no recorded shipments.</div>}
                    <div style={{ maxHeight: 150, overflowY: "auto" }}>
                      {shipments.map((c) => {
                        const dest = c.stops[c.stops.length - 1];
                        return (
                          <div key={c.id} style={row}>
                            <span style={{ flex: 1, color: "#c0d0e0", overflow: "hidden",
                              textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{hubName(dest.hub)}</span>
                            <span style={{ color: "#9ab0c8", fontSize: 9, minWidth: 30, textAlign: "right" }}>{Math.round(c.days)}d</span>
                            <span style={{ color: "#ff9a6a", fontSize: 10, minWidth: 38, textAlign: "right" }}
                              title="delivered price (× base)">{dest.price.toFixed(2)}×</span>
                          </div>
                        );
                      })}
                    </div>
                  </>
                )}
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", width: 430, background: "rgba(12,18,26,0.97)",
  border: "1px solid #24364e", borderRadius: 8, padding: "8px 10px", zIndex: 125,
  boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  marginBottom: 6, color: "#e8d8b0", fontWeight: 700, cursor: "grab", userSelect: "none",
};
const closeBtn: React.CSSProperties = { color: "#7090b0", cursor: "pointer", fontSize: 18, lineHeight: 1 };
const sectionHdr: React.CSSProperties = {
  color: "#6a86a6", fontSize: 10, fontWeight: 700, textTransform: "uppercase",
  letterSpacing: 0.5, borderBottom: "1px solid #1e2e42", paddingBottom: 2, marginBottom: 4,
};
const catHdr: React.CSSProperties = {
  color: "#5f7390", fontSize: 8, textTransform: "uppercase", letterSpacing: 0.4, margin: "3px 0 1px",
};
const goodRow: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 5, fontSize: 11, padding: "2px 3px",
  borderRadius: 3, cursor: "pointer",
};
const row: React.CSSProperties = { display: "flex", alignItems: "center", gap: 5, fontSize: 11, padding: "2px 3px", borderRadius: 3 };
const emptyTxt: React.CSSProperties = { color: "#506680", fontSize: 10, fontStyle: "italic", padding: "4px 3px" };
