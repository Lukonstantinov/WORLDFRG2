import { useSettingsStore, COLOR_PRESETS, type PresetKey } from "@state/settingsStore";
import { LINE_COLOR_DEFAULTS, type LineColorKey } from "@canvas/OverlayManager";

/** Top-screen Appearance settings: recolour every trade/connection overlay line.
 *  Edits flow straight into the renderer (settingsStore → OverlayManager) and are
 *  saved to localStorage (and, when implemented, with the world/campaign file). */

const GROUPS: { label: string; keys: { k: LineColorKey; label: string }[] }[] = [
  {
    label: "Route primitives",
    keys: [
      { k: "tradeLand", label: "Overland caravan" },
      { k: "tradeSea", label: "Maritime" },
      { k: "tradeRiver", label: "River" },
      { k: "tradeTrunk", label: "Commodity trunk" },
      { k: "tradeTrunkMinor", label: "Minor trunk" },
    ],
  },
  {
    label: "Live flows & networks",
    keys: [
      { k: "dynamicFlow", label: "Dynamic trade flow" },
      { k: "corridor", label: "Trade corridor" },
      { k: "corridorArrow", label: "Corridor / flow arrow" },
      { k: "merchantIn", label: "Merchant — inbound" },
      { k: "merchantOut", label: "Merchant — outbound" },
    ],
  },
  {
    label: "Holdings",
    keys: [
      { k: "manufactory", label: "Manufactory export" },
      { k: "estate", label: "Estate export" },
    ],
  },
  {
    label: "Colonies",
    keys: [
      { k: "settlementColony", label: "Settlement colony" },
      { k: "houseOutpost", label: "House outpost" },
      { k: "colonyLane", label: "Colony supply lane" },
    ],
  },
];

const muted = "#6a86a6";

export function SettingsPanel({ onClose }: { onClose: () => void }) {
  const { lineColors, preset, setLineColor, resetLineColor, resetAll, applyPreset } =
    useSettingsStore();

  return (
    <div style={{
      position: "absolute", inset: 0, display: "flex",
      alignItems: "center", justifyContent: "center",
      background: "rgba(0,0,0,0.75)", zIndex: 100,
    }} onClick={onClose}>
      <div onClick={(e) => e.stopPropagation()} style={{
        background: "#111820", border: "1px solid #1e2e42", borderRadius: 10,
        padding: "22px 26px", width: 460, maxHeight: "82%", overflowY: "auto",
        boxShadow: "0 12px 40px rgba(0,0,0,0.5)", color: "#cfe2f6",
      }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
          <h2 style={{ margin: 0, color: "#c0d8f0", fontSize: 17, fontWeight: 600 }}>⚙ Appearance</h2>
          <span onClick={onClose} title="Close"
            style={{ color: "#7090b0", cursor: "pointer", fontSize: 20, lineHeight: 1 }}>×</span>
        </div>
        <div style={{ color: muted, fontSize: 11, marginBottom: 14 }}>
          Recolour the map's trade &amp; connection lines. Changes apply instantly and
          persist on this machine.
        </div>

        {/* Theme presets */}
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 16 }}>
          <span style={{ color: muted, fontSize: 12 }}>Theme</span>
          <select value={preset in COLOR_PRESETS ? preset : "Custom"}
            onChange={(e) => { const v = e.target.value; if (v in COLOR_PRESETS) applyPreset(v as PresetKey); }}
            style={{ background: "#0e1a27", color: "#cfe2f6", border: "1px solid #1e2e42", borderRadius: 7, padding: "5px 9px", fontSize: 12 }}>
            {Object.keys(COLOR_PRESETS).map((p) => <option key={p} value={p}>{p}</option>)}
            {!(preset in COLOR_PRESETS) && <option value="Custom">Custom</option>}
          </select>
          <button onClick={resetAll}
            style={{ marginLeft: "auto", padding: "5px 11px", borderRadius: 7, border: "1px solid #1e2e42", background: "#0d1219", color: "#9fb6cc", cursor: "pointer", fontSize: 12 }}>
            Reset all
          </button>
        </div>

        {GROUPS.map((g) => (
          <div key={g.label} style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 10.5, textTransform: "uppercase", letterSpacing: 1, color: muted, marginBottom: 4 }}>{g.label}</div>
            {g.keys.map(({ k, label }) => {
              const hex = lineColors[k];
              const changed = hex !== LINE_COLOR_DEFAULTS[k];
              return (
                <div key={k} style={{ display: "flex", alignItems: "center", gap: 10, padding: "5px 0", borderBottom: "1px solid #14222f" }}>
                  <span style={{ flex: 1, fontSize: 12.5 }}>{label}</span>
                  <span style={{ fontFamily: "ui-monospace, monospace", fontSize: 11, color: muted, width: 64 }}>{hex}</span>
                  <input type="color" value={hex} onChange={(e) => setLineColor(k, e.target.value)}
                    style={{ width: 30, height: 24, border: "1px solid #1e2e42", borderRadius: 5, background: "none", padding: 0, cursor: "pointer" }} />
                  <span onClick={() => resetLineColor(k)} title="Reset to default"
                    style={{ width: 16, textAlign: "center", cursor: "pointer", fontSize: 14, color: changed ? "#9fb6cc" : "#2c4055" }}>↺</span>
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}
