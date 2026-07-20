import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { useViewportStore } from "@state/viewportStore";
import { computeItinerary } from "@bridge";
import type { Itinerary, Settlement } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, RADIUS } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, Meter, Button, EmptyNote } from "@ui/kit";

/** #23 · Travel-time / itinerary calculator. Pick an origin and a destination
 *  settlement, optionally forbid open-sea crossings, and get the realistic journey
 *  time by foot / horse / cart (water legs always go by boat/ship) over the same
 *  coarse cost grid trade uses. The routed polyline is drawn on the map.
 *  Built on the shared UI kit (src/ui/kit.tsx). */
export function ItineraryPanel() {
  const open = useUIStore((s) => s.showItinerary);
  const settlements = useWorldStore((s) => s.settlements);
  const rivers = useWorldStore((s) => s.rivers);
  const setTravelRoute = useUIStore((s) => s.setTravelRoute);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const focusOn = useViewportStore((s) => s.focusOn);

  const [originId, setOriginId] = useState<string>("");
  const [destId, setDestId] = useState<string>("");
  const [avoidSea, setAvoidSea] = useState(false);
  const [result, setResult] = useState<Itinerary | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string>("");

  const sorted = useMemo(
    () => [...settlements].sort((a, b) => a.name.localeCompare(b.name)),
    [settlements],
  );
  const origin = settlements.find((s) => s.id === originId) ?? null;
  const dest = settlements.find((s) => s.id === destId) ?? null;

  // Clear the drawn route when the panel closes.
  useEffect(() => {
    if (!open) { setTravelRoute(null); }
  }, [open, setTravelRoute]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.route);
  if (!open) return null;
  const close = () => { setTravelRoute(null); useUIStore.getState().setShowItinerary(false); };

  const run = async () => {
    if (!origin || !dest || origin.id === dest.id) return;
    setBusy(true); setErr(""); setResult(null);
    try {
      const it = await computeItinerary(
        origin.x, origin.y, dest.x, dest.y,
        rivers.map((r) => ({ points: r.points })),
        avoidSea ? 2 : 0,
        false,
      );
      setResult(it);
      if (it.reachable) {
        setTravelRoute(it.points as [number, number][]);
        setOverlayVisible("travelRoute", true);
      } else {
        setTravelRoute(null);
      }
    } catch (e) { setErr(String(e)); }
    setBusy(false);
  };

  const picker = (label: string, value: string, onChange: (v: string) => void, pin: string) => (
    <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
      <span style={{ width: 14, textAlign: "center" }}>{pin}</span>
      <span style={{ color: T.inkMid, fontSize: FZ.small, width: 64 }}>{label}</span>
      <select value={value} onChange={(e) => onChange(e.target.value)} data-no-drag style={select}>
        <option value="">— choose a settlement —</option>
        {sorted.map((s: Settlement) => (
          <option key={s.id} value={s.id}>{s.name}{s.size === "capital" ? " ★" : ""}</option>
        ))}
      </select>
    </div>
  );

  return (
    <Panel width={318} style={{ top: 60, right: 360, zIndex: 40, ...rootStyle }}>
      <PanelHeader icon="🧭" title="Itinerary · travel time" onDragStart={onPointerDown} onClose={close} />

      <div style={{ padding: "10px 12px", overflowY: "auto" }}>
        {settlements.length < 2 && (
          <EmptyNote>Generate settlements first (Step 7) to plan a journey.</EmptyNote>
        )}
        {settlements.length >= 2 && (
          <>
            {picker("From", originId, setOriginId, "🟢")}
            {picker("To", destId, setDestId, "🔴")}
            <label style={{ display: "flex", alignItems: "center", gap: 6, margin: "4px 0 8px", color: T.inkMid, fontSize: FZ.small, cursor: "pointer" }}>
              <input type="checkbox" checked={avoidSea} onChange={(e) => setAvoidSea(e.target.checked)}
                style={{ accentColor: T.accent }} />
              Avoid open-sea crossings (stay on one landmass)
            </label>
            <Button variant="primary" onClick={run} disabled={busy || !origin || !dest || origin?.id === dest?.id} style={{ width: "100%", padding: 6 }}>
              {busy ? "Routing…" : "Plot journey"}
            </Button>

            {err && <div style={{ color: T.badInk, fontSize: FZ.small, marginTop: 8 }}>{err}</div>}

            {result && !result.reachable && (
              <div style={{ color: T.warn, fontSize: FZ.body, marginTop: 10 }}>
                No overland/sea route found between these settlements
                {avoidSea ? " without crossing open sea — try allowing sea crossings." : "."}
              </div>
            )}

            {result && result.reachable && (
              <div style={{ marginTop: 12 }}>
                <div style={{ display: "flex", alignItems: "baseline", gap: 8, marginBottom: 6 }}>
                  <span style={{ color: T.parchment, fontWeight: 700, fontSize: FZ.head }}>{fmtKm(result.km)}</span>
                  <span style={{ color: T.inkMid, fontSize: FZ.small }}>· mostly {MODE[result.dominant_mode]}</span>
                </div>

                <div style={{ marginTop: 4 }}>
                  {legBar("Overland", result.land_km, result.km, "#c98a4a")}
                  {legBar("River", result.river_km, result.km, "#4a9ad0")}
                  {legBar("Sea", result.sea_km, result.km, "#3a7a9a")}
                </div>

                <div style={{ marginTop: 10, display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 6 }}>
                  {modeCard("🚶 On foot", result.days_foot)}
                  {modeCard("🐎 By horse", result.days_horse)}
                  {modeCard("🛒 By cart", result.days_cart)}
                </div>
                <div style={{ color: T.inkDim, fontSize: FZ.tiny, marginTop: 8, lineHeight: 1.5 }}>
                  Water legs travel by boat/ship; the mode only sets the land pace.
                  Terrain (relief, storms) already slows the affected legs.
                </div>
                <Button variant="ghost" onClick={() => origin && focusOn(origin.x, origin.y)} style={{ width: "100%", marginTop: 8, padding: 6 }}>
                  Center map on origin
                </Button>
              </div>
            )}
          </>
        )}
      </div>
    </Panel>
  );
}

const MODE = ["overland", "by sea", "by river"] as const;

function fmtKm(km: number): string {
  return km >= 1000 ? `${(km / 1000).toFixed(1)}k km` : `${Math.round(km)} km`;
}
function fmtDays(d: number): string {
  if (d < 1) return "<1 day";
  if (d < 10) return `${d.toFixed(1)} days`;
  if (d < 60) return `${Math.round(d)} days`;
  return `${(d / 7).toFixed(0)} wks`;
}

function modeCard(label: string, days: number) {
  return (
    <div style={{ background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: RADIUS.md, padding: "6px 4px", textAlign: "center" }}>
      <div style={{ color: T.inkMid, fontSize: FZ.tiny }}>{label}</div>
      <div style={{ color: T.parchment, fontWeight: 700, fontSize: FZ.base, marginTop: 2 }}>{fmtDays(days)}</div>
    </div>
  );
}

function legBar(label: string, km: number, total: number, color: string) {
  if (km <= 0) return null;
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
      <span style={{ width: 56, color: T.inkMid, fontSize: FZ.tiny }}>{label}</span>
      <Meter value={km} max={total} color={color} height={7} />
      <span style={{ width: 56, textAlign: "right", color: T.inkMid, fontSize: FZ.tiny }}>{fmtKm(km)}</span>
    </div>
  );
}

const select: React.CSSProperties = {
  flex: 1, background: T.card, border: `1px solid ${T.line}`, color: T.ink,
  padding: "3px 5px", borderRadius: RADIUS.sm, fontSize: FZ.body, minWidth: 0,
};
