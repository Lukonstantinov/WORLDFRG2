import { useMemo, useRef, useState } from "react";
import { useWorldStore } from "../state/worldStore";
import { useViewportStore } from "../state/viewportStore";
import type { Settlement } from "../types";

/** Header search box: type a settlement name → live prefix-matched results with
 *  population, culture and region; ↑/↓ + Enter or click flies the camera there
 *  and drops a highlight pin. Sourced from the world's settlement list. */
export function SettlementSearch() {
  const settlements = useWorldStore((s) => s.settlements);
  const focusOn = useViewportStore((s) => s.focusOn);
  const setSearchPin = useViewportStore((s) => s.setSearchPin);

  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const blurTimer = useRef<number | null>(null);

  const results = useMemo<Settlement[]>(() => {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    return settlements
      .filter((s) => s.name.toLowerCase().includes(q))
      .sort((a, b) => {
        // Prefix matches first, then by population.
        const ap = a.name.toLowerCase().startsWith(q) ? 0 : 1;
        const bp = b.name.toLowerCase().startsWith(q) ? 0 : 1;
        if (ap !== bp) return ap - bp;
        return b.population - a.population;
      })
      .slice(0, 8);
  }, [query, settlements]);

  const choose = (s: Settlement) => {
    focusOn(s.x, s.y);
    setSearchPin(s.x, s.y);
    setQuery("");
    setOpen(false);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!open || results.length === 0) {
      if (e.key === "Escape") { setQuery(""); setOpen(false); }
      return;
    }
    if (e.key === "ArrowDown") { e.preventDefault(); setActive((a) => (a + 1) % results.length); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setActive((a) => (a - 1 + results.length) % results.length); }
    else if (e.key === "Enter") { e.preventDefault(); choose(results[Math.min(active, results.length - 1)]); }
    else if (e.key === "Escape") { setQuery(""); setOpen(false); }
  };

  const showDrop = open && query.trim().length > 0;

  return (
    <div style={{ position: "relative", width: 240 }}>
      <input
        value={query}
        placeholder="🔍 Find a settlement…"
        onChange={(e) => { setQuery(e.target.value); setActive(0); setOpen(true); }}
        onFocus={() => setOpen(true)}
        onBlur={() => { blurTimer.current = window.setTimeout(() => setOpen(false), 150); }}
        onKeyDown={onKeyDown}
        style={inputStyle}
      />
      {showDrop && (
        <div style={dropStyle}
          onMouseDown={() => { if (blurTimer.current) window.clearTimeout(blurTimer.current); }}>
          {results.length === 0 && (
            <div style={{ padding: "8px 10px", color: "#56708e", fontSize: 11 }}>No settlement matches “{query}”.</div>
          )}
          {results.map((s, i) => (
            <div key={s.id}
              onMouseEnter={() => setActive(i)}
              onClick={() => choose(s)}
              style={{ ...rowStyle, background: i === active ? "#16273c" : "transparent" }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
                <span style={{ fontSize: 11 }}>📍</span>
                <span style={{ color: "#e8dcc0", fontWeight: 600, fontSize: 12, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{s.name}</span>
                <span style={{ color: "#9ab0c8", fontSize: 10 }}>{s.population.toLocaleString()}</span>
                {s.culture && <span style={cultureTag}>{s.culture}</span>}
                <span style={{ color: "#5a7090", fontSize: 11 }}>▸</span>
              </div>
              {(s.site || s.region) && (
                <div style={{ color: "#6a86a6", fontSize: 9, marginLeft: 17 }}>
                  {[s.site, s.region].filter(Boolean).join(" · ")}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  width: "100%", boxSizing: "border-box", height: 24,
  background: "#0c141e", border: "1px solid #243650", borderRadius: 5,
  color: "#cfe0f4", fontSize: 12, padding: "0 8px", outline: "none",
};
const dropStyle: React.CSSProperties = {
  position: "absolute", top: 28, right: 0, width: 300, maxHeight: "60vh", overflowY: "auto",
  background: "#0b1320", border: "1px solid #243650", borderRadius: 6,
  boxShadow: "0 10px 30px rgba(0,0,0,0.55)", zIndex: 200, padding: "3px 0",
};
const rowStyle: React.CSSProperties = {
  padding: "4px 9px", cursor: "pointer", borderBottom: "1px solid #111c2a",
};
const cultureTag: React.CSSProperties = {
  fontSize: 8, color: "#7fd0c0", border: "1px solid #2e5a52", borderRadius: 3, padding: "0 3px",
};
