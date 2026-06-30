import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useViewportStore } from "../state/viewportStore";
import { computeItinerary } from "../bridge/tauri";
import type { Itinerary, Settlement } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

/** #23 · Travel-time / itinerary calculator. Pick an origin and a destination
 *  settlement, optionally forbid open-sea crossings, and get the realistic journey
 *  time by foot / horse / cart (water legs always go by boat/ship) over the same
 *  coarse cost grid trade uses. The routed polyline is drawn on the map. */
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
      <span style={{ color: "#8aa0b8", fontSize: 10, width: 64 }}>{label}</span>
      <select value={value} onChange={(e) => onChange(e.target.value)} style={select}>
        <option value="">— choose a settlement —</option>
        {sorted.map((s: Settlement) => (
          <option key={s.id} value={s.id}>{s.name}{s.size === "capital" ? " ★" : ""}</option>
        ))}
      </select>
    </div>
  );

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>🧭 Itinerary · travel time</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>

      <div style={{ padding: "10px 12px" }}>
        {settlements.length < 2 && (
          <div style={empty}>Generate settlements first (Step 7) to plan a journey.</div>
        )}
        {settlements.length >= 2 && (
          <>
            {picker("From", originId, setOriginId, "🟢")}
            {picker("To", destId, setDestId, "🔴")}
            <label style={{ display: "flex", alignItems: "center", gap: 6, margin: "4px 0 8px", color: "#9ab0c8", fontSize: 10, cursor: "pointer" }}>
              <input type="checkbox" checked={avoidSea} onChange={(e) => setAvoidSea(e.target.checked)}
                style={{ accentColor: "#3a80c0" }} />
              Avoid open-sea crossings (stay on one landmass)
            </label>
            <button onClick={run} disabled={busy || !origin || !dest || origin.id === dest.id} style={btn}>
              {busy ? "Routing…" : "Plot journey"}
            </button>

            {err && <div style={{ color: "#e08a8a", fontSize: 10, marginTop: 8 }}>{err}</div>}

            {result && !result.reachable && (
              <div style={{ color: "#e0b070", fontSize: 11, marginTop: 10 }}>
                No overland/sea route found between these settlements
                {avoidSea ? " without crossing open sea — try allowing sea crossings." : "."}
              </div>
            )}

            {result && result.reachable && (
              <div style={{ marginTop: 12 }}>
                <div style={{ display: "flex", alignItems: "baseline", gap: 8, marginBottom: 6 }}>
                  <span style={{ color: "#cfe2f6", fontWeight: 700, fontSize: 13 }}>{fmtKm(result.km)}</span>
                  <span style={{ color: "#7fa0c0", fontSize: 10 }}>· mostly {MODE[result.dominant_mode]}</span>
                </div>

                <div style={breakdown}>
                  {bar("Overland", result.land_km, result.km, "#c98a4a")}
                  {bar("River", result.river_km, result.km, "#4a9ad0")}
                  {bar("Sea", result.sea_km, result.km, "#3a7a9a")}
                </div>

                <div style={{ marginTop: 10, display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 6 }}>
                  {modeCard("🚶 On foot", result.days_foot)}
                  {modeCard("🐎 By horse", result.days_horse)}
                  {modeCard("🛒 By cart", result.days_cart)}
                </div>
                <div style={{ color: "#5a6a80", fontSize: 9, marginTop: 8, lineHeight: 1.5 }}>
                  Water legs travel by boat/ship; the mode only sets the land pace.
                  Terrain (relief, storms) already slows the affected legs.
                </div>
                <button onClick={() => origin && focusOn(origin.x, origin.y)} style={{ ...btn, marginTop: 8, background: "#16283a" }}>
                  Center map on origin
                </button>
              </div>
            )}
          </>
        )}
      </div>
    </div>
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
    <div style={{ background: "#0c1622", border: "1px solid #1a2a3e", borderRadius: 6, padding: "6px 4px", textAlign: "center" }}>
      <div style={{ color: "#8aa0b8", fontSize: 9 }}>{label}</div>
      <div style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12, marginTop: 2 }}>{fmtDays(days)}</div>
    </div>
  );
}

function bar(label: string, km: number, total: number, color: string) {
  const pct = total > 0 ? (km / total) * 100 : 0;
  if (km <= 0) return null;
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
      <span style={{ width: 56, color: "#8aa0b8", fontSize: 9 }}>{label}</span>
      <span style={{ flex: 1, height: 7, background: "#0a131d", borderRadius: 3, overflow: "hidden" }}>
        <span style={{ display: "block", height: "100%", width: `${pct}%`, background: color }} />
      </span>
      <span style={{ width: 56, textAlign: "right", color: "#9ab0c8", fontSize: 9 }}>{fmtKm(km)}</span>
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 318,
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 40,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#cfe0f4", fontWeight: 700, fontSize: 12,
};
const select: React.CSSProperties = {
  flex: 1, background: "#080c12", border: "1px solid #1e2e42", color: "#cfe0f4",
  padding: "3px 5px", borderRadius: 4, fontSize: 11, minWidth: 0,
};
const btn: React.CSSProperties = {
  width: "100%", padding: "6px", background: "#2060a0", color: "#fff", border: "none",
  borderRadius: 4, cursor: "pointer", fontSize: 11, fontWeight: 600,
};
const breakdown: React.CSSProperties = { marginTop: 4 };
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "8px 0" };
