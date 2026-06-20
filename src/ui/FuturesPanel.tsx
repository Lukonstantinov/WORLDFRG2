import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { useGoodsStore } from "../state/goodsStore";
import { campaignFuturesLanes } from "../bridge/tauri";
import type { FuturesLane } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** The futures CONTRACTS list — every active supply contract in the world. Click a
 *  row to ISOLATE that one lane on the map (the rest fade). Click a city or the
 *  holder to FOCUS — every lane touching that city / run by that house stays bold
 *  and the others dim (a city's inbound + outbound contract network; a warehouse's
 *  whole distribution). Opening the panel turns the 📜 Futures map layer on. */
export function FuturesPanel() {
  const open = useUIStore((s) => s.showFutures);
  const setOpen = useUIStore((s) => s.setShowFutures);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const setSelectedLane = useUIStore((s) => s.setSelectedFuturesLane);
  const focus = useUIStore((s) => s.futuresFocus);
  const setFocus = useUIStore((s) => s.setFuturesFocus);
  const active = useCampaignStore((s) => s.snapshot?.active ?? false);
  const tick = useCampaignStore((s) => s.snapshot?.clock.tick ?? 0);
  const goodMeta = useGoodsStore((s) => s.meta);
  const [rows, setRows] = useState<FuturesLane[]>([]);
  const [q, setQ] = useState("");

  // Opening the panel switches the map's Futures layer on so the lanes are visible.
  useEffect(() => { if (open) setOverlayVisible("futures", true); }, [open, setOverlayVisible]);

  useEffect(() => {
    if (!open || !active) return;
    let alive = true;
    campaignFuturesLanes().then((r) => { if (alive) setRows(r); }).catch(() => {});
    return () => { alive = false; };
  }, [open, active, tick]);

  const filtered = useMemo(() => {
    const s = q.trim().toLowerCase();
    if (!s) return rows;
    return rows.filter((r) =>
      r.holder.toLowerCase().includes(s) || r.good.toLowerCase().includes(s)
      || r.a_name.toLowerCase().includes(s) || r.b_name.toLowerCase().includes(s));
  }, [rows, q]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.futures);
  if (!open) return null;
  const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));
  const total = filtered.reduce((a, r) => a + r.qty, 0);
  const term = (t: number) => t >= 7 ? "#ffcf3f" : t >= 5 ? "#f0b54a" : t >= 3 ? "#d8a05a" : "#c8b486";
  const focusLabel = focus?.city ?? focus?.holder ?? focus?.good ?? null;

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>📜 Futures Contracts</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={() => setOpen(false)}>✕</span>
      </div>
      <div style={{ padding: "6px 8px", borderBottom: "1px solid #1a2a3e", display: "flex", gap: 6, alignItems: "center" }}>
        <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="filter city / good / house…"
          style={{ flex: 1, background: "#0a1018", border: "1px solid #23364c", borderRadius: 5, color: "#cfe0f4", fontSize: 11, padding: "3px 6px" }} />
        {(focusLabel || q) && (
          <span onClick={() => { setFocus(null); setSelectedLane(null); setQ(""); }}
            style={{ cursor: "pointer", color: "#7fb0d0", fontSize: 10 }} title="Clear focus & selection">show all</span>
        )}
      </div>
      {focusLabel && (
        <div style={{ padding: "3px 9px", fontSize: 10, color: "#ffcf3f", background: "#1a1606" }}>
          focus: {focusLabel} — matching lanes highlighted
        </div>
      )}
      <div style={{ overflowY: "auto", padding: "2px 6px 10px" }}>
        {!active && <div style={hint}>Begin the campaign (Step 11) to see contracts.</div>}
        {active && filtered.length === 0 && <div style={hint}>No futures contracts yet.</div>}
        {filtered.map((r, i) => {
          const m = goodMeta(r.good);
          return (
            <div key={i} onClick={() => { setSelectedLane(r); setFocus(null); }}
              title="Isolate this contract lane on the map"
              style={{ padding: "4px 2px", borderBottom: "1px solid #131e2a", cursor: "pointer" }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
                <span style={{ width: 8, height: 8, borderRadius: 2, background: r.color, alignSelf: "center" }} />
                <span onClick={(e) => { e.stopPropagation(); setFocus({ holder: r.holder }); }}
                  style={{ color: "#e8d8b0", fontSize: 11, fontWeight: 600, cursor: "pointer", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 130 }}
                  title="Focus this house's whole distribution">{r.holder}</span>
                {r.is_guild && <span style={{ fontSize: 8, color: "#7fd0c0" }}>GUILD</span>}
                <span style={{ flex: 1 }} />
                <span style={{ color: term(r.term), fontSize: 9, fontWeight: 700 }}>{r.term}y</span>
                {r.suspended && <span style={{ color: "#ff9a6a", fontSize: 9 }}>☣</span>}
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 10, color: "#c0d0e0", marginTop: 1 }}>
                <span>{m.icon} {m.name}</span>
                <span style={{ color: "#7fd0a0" }}>{fmt(r.qty)}/mo</span>
                <span style={{ flex: 1 }} />
                <span style={{ color: "#6a86a6", fontSize: 9 }}>Y{r.end_year}</span>
              </div>
              <div style={{ fontSize: 10, color: "#9ab0c8", marginTop: 1 }}>
                <span onClick={(e) => { e.stopPropagation(); setFocus({ city: r.a_name }); }}
                  style={{ cursor: "pointer", color: "#cfe0f4" }} title="Focus this city's contracts">{r.a_name}</span>
                <span> → </span>
                <span onClick={(e) => { e.stopPropagation(); setFocus({ city: r.b_name }); }}
                  style={{ cursor: "pointer", color: "#cfe0f4" }} title="Focus this city's contracts">{r.b_name}</span>
              </div>
            </div>
          );
        })}
        {filtered.length > 0 && (
          <div style={{ color: "#56708e", fontSize: 9, marginTop: 5 }}>
            Σ {fmt(total)} units/mo · {filtered.length} lanes · click a row to isolate · click a city/house to focus
          </div>
        )}
      </div>
    </div>
  );
}

const hint: React.CSSProperties = { color: "#506080", fontSize: 11, padding: 10 };
const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 312, maxHeight: "78vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #3a3214", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 42,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#ffcf3f", fontWeight: 700, fontSize: 12,
};
