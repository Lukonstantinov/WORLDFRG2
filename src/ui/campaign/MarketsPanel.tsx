import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { useViewportStore } from "@state/viewportStore";
import { campaignGetHub, campaignMarketCities } from "@bridge";
import type { HubDetail, MarketCity } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { Panel, PanelHeader, PanelBody, EmptyNote } from "@ui/kit";
import { T, FZ } from "@ui/campaign/chronicleTheme";
import { CityMarketView } from "@ui/campaign/CityMarketView";

/** The floating MARKETS window — `docs/TRADE_AND_MARKET_REVIEW.md` Part 3.
 *
 *  Renders the SAME `CityMarketView` the settlement window's Trade tab does, but
 *  behind its own city picker: type a name and read that market without moving the
 *  map or the settlement selection. That independence is the whole point — it is
 *  what lets you hold one city's book open while looking at another, which is the
 *  only way to compare two markets in an app whose every other view is bound to
 *  the map's current selection.
 *
 *  Seeded from `selectedHub` when first opened, then never re-bound to it. */
export function MarketsPanel() {
  const open = useUIStore((s) => s.showMarkets);
  const hubId = useUIStore((s) => s.marketsHub);
  const setHubId = useUIStore((s) => s.setMarketsHub);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const focusOn = useViewportStore((s) => s.focusOn);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [detail, setDetail] = useState<HubDetail | null>(null);
  const [cities, setCities] = useState<MarketCity[]>([]);
  const [query, setQuery] = useState("");
  const [picking, setPicking] = useState(false);

  // Live cities, biggest first — refreshed on the YEAR rather than the tick, since
  // towns are founded and abandoned yearly and a per-tick refetch of the whole list
  // would be pure churn.
  const year = Math.floor(tick / 365);
  useEffect(() => {
    let alive = true;
    if (!open || !active) return;
    campaignMarketCities().then((c) => { if (alive) setCities(c); }).catch(() => { if (alive) setCities([]); });
    return () => { alive = false; };
  }, [open, active, year]);

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return cities.slice(0, 12);
    return cities
      .filter((h) => h.name.toLowerCase().includes(q))
      .sort((a, b) => {
        const ap = a.name.toLowerCase().startsWith(q) ? 0 : 1;
        const bp = b.name.toLowerCase().startsWith(q) ? 0 : 1;
        return ap !== bp ? ap - bp : b.population - a.population;
      })
      .slice(0, 12);
  }, [cities, query]);

  // Fall back to the largest city rather than opening blank when nothing is picked.
  useEffect(() => {
    if (!open || !active) return;
    if (hubId === null && cities.length > 0) setHubId(cities[0].id);
  }, [open, active, hubId, cities, setHubId]);

  useEffect(() => {
    let alive = true;
    if (!open || !active || hubId === null) { setDetail(null); return; }
    campaignGetHub(hubId).then((d) => { if (alive) setDetail(d); }).catch(() => { if (alive) setDetail(null); });
    return () => { alive = false; };
  }, [open, active, hubId, tick]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.goods);
  if (!open) return null;
  const close = () => useUIStore.getState().setShowMarkets(false);
  const current = cities.find((c) => c.id === hubId);

  return (
    <Panel onPointerDown={onPointerDown} width={720} maxHeight="82vh"
      style={{ top: 74, left: 320, zIndex: 41, ...rootStyle }}>
      <PanelHeader icon="⚖" title="Markets — the buy/sell book" onDragStart={onPointerDown} onClose={close} />
      {!active && <EmptyNote>Begin the campaign (Step 11) — a market needs merchants.</EmptyNote>}
      {active && (
        <>
          {/* ── city picker: independent of the map selection ─────────────── */}
          <div style={{ padding: "5px 10px", borderBottom: `1px solid ${T.line}`, position: "relative", flex: "0 0 auto" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <span style={{ color: T.inkDim, fontSize: FZ.small }}>city</span>
              <input
                value={picking ? query : (current?.name ?? "")}
                placeholder="type a city name…"
                onFocus={() => { setPicking(true); setQuery(""); }}
                onBlur={() => window.setTimeout(() => setPicking(false), 140)}
                onChange={(e) => { setQuery(e.target.value); setPicking(true); }}
                style={{
                  flex: 1, background: T.card, border: `1px solid ${T.line}`, borderRadius: 4,
                  color: T.parchment, fontSize: FZ.body, padding: "3px 7px", outline: "none",
                }}
              />
              {current && (
                <span onClick={() => focusOn(current.x, current.y)} title="Centre the map here"
                  style={{ cursor: "pointer", color: T.inkMid, fontSize: FZ.small, padding: "2px 6px",
                    border: `1px solid ${T.line}`, borderRadius: 4 }}>◎ find</span>
              )}
              {current && (
                <span style={{ color: T.inkDim, fontSize: FZ.tiny }}>
                  pop {Math.round(current.population).toLocaleString()}
                </span>
              )}
            </div>
            {picking && results.length > 0 && (
              <div style={{
                position: "absolute", top: "100%", left: 40, right: 10, zIndex: 5,
                background: T.raised, border: `1px solid ${T.line}`, borderRadius: 4,
                maxHeight: 220, overflowY: "auto", boxShadow: "0 6px 18px rgba(0,0,0,0.5)",
              }}>
                {results.map((h) => (
                  <div key={h.id}
                    onMouseDown={(e) => { e.preventDefault(); setHubId(h.id); setPicking(false); setQuery(""); }}
                    style={{ display: "flex", gap: 8, padding: "3px 8px", cursor: "pointer",
                      fontSize: FZ.body, color: h.id === hubId ? T.gold : T.parchment,
                      borderBottom: `1px solid ${T.lineSoft}` }}>
                    <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{h.name}</span>
                    <span style={{ color: T.inkDim, fontSize: FZ.tiny }}>{Math.round(h.population).toLocaleString()}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
          <PanelBody style={{ padding: "6px 10px 10px" }}>
            {!detail && <EmptyNote>Pick a city to read its market.</EmptyNote>}
            {detail && <CityMarketView detail={detail} />}
          </PanelBody>
        </>
      )}
    </Panel>
  );
}
