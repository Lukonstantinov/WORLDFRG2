import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { useGoodsStore } from "../state/goodsStore";
import { campaignFuturesLanes } from "../bridge/tauri";
import type { FuturesLane } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";
import { T, FZ, RADIUS } from "./chronicleTheme";
import { Panel, PanelHeader, PanelBody, Meter, EmptyNote, FootNote } from "./kit";

/** The futures CONTRACTS list — every active supply contract in the world. Click a
 *  row to ISOLATE that one lane on the map (the rest fade). Click a city or the
 *  holder to FOCUS — every lane touching that city / run by that house stays bold
 *  and the others dim (a city's inbound + outbound contract network; a warehouse's
 *  whole distribution). Opening the panel turns the 📜 Futures map layer on.
 *  Built on the shared UI kit (src/ui/kit.tsx). */
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
  const term = (t: number) => t >= 7 ? T.gold : t >= 5 ? "#f0b54a" : t >= 3 ? "#d8a05a" : "#c8b486";
  const focusLabel = focus?.city ?? focus?.holder ?? focus?.good ?? null;

  return (
    <Panel width={312} maxHeight="78vh" style={{ top: 60, right: 360, zIndex: 42, ...rootStyle }}>
      <PanelHeader icon="📜" title="Futures Contracts" onDragStart={onPointerDown} onClose={() => setOpen(false)} />
      <div style={{ padding: "6px 8px", borderBottom: `1px solid ${T.line}`, display: "flex", gap: 6, alignItems: "center", flex: "0 0 auto" }}>
        <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="filter city / good / house…" data-no-drag
          style={{ flex: 1, background: T.card, border: `1px solid ${T.line}`, borderRadius: RADIUS.sm, color: T.ink, fontSize: FZ.body, padding: "3px 6px" }} />
        {(focusLabel || q) && (
          <span data-no-drag onClick={() => { setFocus(null); setSelectedLane(null); setQ(""); }}
            style={{ cursor: "pointer", color: "#8ec2ee", fontSize: FZ.small }} title="Clear focus & selection">show all</span>
        )}
      </div>
      {focusLabel && (
        <div style={{ padding: "3px 9px", fontSize: FZ.small, color: T.gold, background: "rgba(216,178,74,0.08)", flex: "0 0 auto" }}>
          focus: {focusLabel} — matching lanes highlighted
        </div>
      )}
      <PanelBody style={{ padding: "2px 6px 10px" }}>
        {!active && <EmptyNote>Begin the campaign (Step 11) to see contracts.</EmptyNote>}
        {active && filtered.length === 0 && <EmptyNote>No futures contracts yet.</EmptyNote>}
        {filtered.map((r, i) => {
          const m = goodMeta(r.good);
          return (
            <div key={i} onClick={() => { setSelectedLane(r); setFocus(null); }}
              title="Isolate this contract lane on the map"
              style={{ padding: "4px 2px", borderBottom: `1px solid ${T.lineSoft}`, cursor: "pointer" }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
                <span style={{ width: 8, height: 8, borderRadius: 2, background: r.color, alignSelf: "center" }} />
                <span onClick={(e) => { e.stopPropagation(); setFocus({ holder: r.holder }); }}
                  style={{ color: T.parchment, fontSize: FZ.body, fontWeight: 600, cursor: "pointer", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 130 }}
                  title="Focus this house's whole distribution">{r.holder}</span>
                {r.is_guild && <span style={{ fontSize: FZ.micro, color: T.goodInk }}>GUILD</span>}
                <span style={{ flex: 1 }} />
                <span style={{ color: term(r.term), fontSize: FZ.tiny, fontWeight: 700 }}>{r.term}y</span>
                {r.suspended && <span style={{ color: T.bad, fontSize: FZ.tiny }}>☣</span>}
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: FZ.small, color: T.inkMid, marginTop: 1 }}>
                <span>{m.icon} {m.name}</span>
                <span style={{ color: T.goodInk }}>{fmt(r.qty)}/mo</span>
                <span style={{ flex: 1 }} />
                <span style={{ color: T.inkDim, fontSize: FZ.tiny }}>Y{r.end_year}</span>
              </div>
              <div style={{ fontSize: FZ.small, color: T.inkMid, marginTop: 1 }}>
                <span onClick={(e) => { e.stopPropagation(); setFocus({ city: r.a_name }); }}
                  style={{ cursor: "pointer", color: T.ink }} title="Focus this city's contracts">{r.a_name}</span>
                <span> → </span>
                <span onClick={(e) => { e.stopPropagation(); setFocus({ city: r.b_name }); }}
                  style={{ cursor: "pointer", color: T.ink }} title="Focus this city's contracts">{r.b_name}</span>
              </div>
              {r.fulfilled_pct != null && (
                <div style={{ marginTop: 3 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: FZ.tiny, color: T.inkMid }}>
                    <span style={{ minWidth: 60 }}>{Math.round(r.fulfilled_pct)}% fulfilled</span>
                    <Meter value={r.fulfilled_pct} max={100} height={5}
                      color={r.fulfilled_pct >= 80 ? T.good : r.fulfilled_pct >= 40 ? T.warn : T.bad} />
                    <span style={{ color: T.goodInk }} title="grain-eq value moved so far">{fmt(r.value ?? 0)}</span>
                  </div>
                  <div style={{ fontSize: FZ.micro, color: T.inkFaint, marginTop: 1 }}>
                    sealed in {r.sealed_at || r.b_name} · {fmt(r.delivered ?? 0)} delivered
                  </div>
                </div>
              )}
            </div>
          );
        })}
        {filtered.length > 0 && (
          <FootNote style={{ fontSize: FZ.tiny }}>
            Σ {fmt(total)} units/mo · {filtered.length} lanes · click a row to isolate · click a city/house to focus
          </FootNote>
        )}
      </PanelBody>
    </Panel>
  );
}
