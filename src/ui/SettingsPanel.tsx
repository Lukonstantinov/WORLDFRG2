import { useState } from "react";
import {
  useSettingsStore, COLOR_PRESETS, type PresetKey,
  LABEL_THEME_NAMES,
} from "@state/settingsStore";
import {
  LINE_COLOR_DEFAULTS, type LineColorKey,
  LABEL_STYLE_DEFAULTS, LABEL_FONTS, LABEL_FONT_LABELS,
  type LabelKey, type LabelFontKey,
} from "@canvas/OverlayManager";

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

/** Map-label classes, grouped the way you read a map: who governs it, what water is
 *  there, what the land is, and whose it is. */
const LABEL_GROUPS: { label: string; keys: { k: LabelKey; label: string }[] }[] = [
  {
    label: "Administrative",
    keys: [
      { k: "province", label: "Province" },
      { k: "settlement", label: "City / town" },
      { k: "state", label: "State" },
    ],
  },
  {
    label: "Water",
    keys: [
      { k: "river", label: "River" },
      { k: "lake", label: "Lake" },
    ],
  },
  {
    label: "Terrain",
    keys: [
      { k: "mountain", label: "Mountain / peak" },
      { k: "desert", label: "Desert" },
      { k: "forest", label: "Forest" },
      { k: "tundra", label: "Tundra" },
    ],
  },
  {
    label: "Cultural & trade",
    keys: [
      { k: "cultureRegion", label: "Culture region" },
      { k: "peopleTerritory", label: "People territory" },
      { k: "tradeBasin", label: "Trade basin" },
    ],
  },
];

/** A font stack → its key, so the dropdown can show which one is selected even
 *  after a theme set it. Unrecognised stacks fall through to "Custom". */
function fontKeyOf(family: string): LabelFontKey | "" {
  const hit = (Object.keys(LABEL_FONTS) as LabelFontKey[]).find((k) => LABEL_FONTS[k] === family);
  return hit ?? "";
}

const muted = "#6a86a6";
const inputStyle: React.CSSProperties = {
  background: "#0e1a27", color: "#cfe2f6", border: "1px solid #1e2e42",
  borderRadius: 7, padding: "5px 9px", fontSize: 12,
};

export function SettingsPanel({ onClose }: { onClose: () => void }) {
  const { lineColors, preset, setLineColor, resetLineColor, resetAll, applyPreset,
    labelStyles, labelTheme, setLabelStyle, resetLabelStyle, applyLabelTheme, resetLabels } =
    useSettingsStore();
  const [tab, setTab] = useState<"lines" | "labels">("lines");

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
        <div style={{ color: muted, fontSize: 11, marginBottom: 12 }}>
          Restyle the map's lines and place names. Changes apply instantly, persist on
          this machine, and are saved with the world.
        </div>

        {/* Section tabs */}
        <div style={{ display: "flex", gap: 6, marginBottom: 14 }}>
          {([["lines", "Overlay lines"], ["labels", "Map labels"]] as const).map(([id, lbl]) => (
            <button key={id} onClick={() => setTab(id)} style={{
              padding: "5px 12px", borderRadius: 7, fontSize: 12, cursor: "pointer",
              border: `1px solid ${tab === id ? "#2c5a86" : "#1e2e42"}`,
              background: tab === id ? "#16293c" : "#0d1219",
              color: tab === id ? "#cfe2f6" : "#7d97b0",
            }}>{lbl}</button>
          ))}
        </div>

        {tab === "labels" ? (
          <LabelSection
            labelStyles={labelStyles} labelTheme={labelTheme}
            setLabelStyle={setLabelStyle} resetLabelStyle={resetLabelStyle}
            applyLabelTheme={applyLabelTheme} resetLabels={resetLabels}
          />
        ) : (
        <>
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
        </>
        )}
      </div>
    </div>
  );
}

/** Map-label typography: a theme picker plus a per-class face and colour. Each row
 *  renders the class's own name IN ITS OWN STYLE, so the list doubles as a live
 *  specimen sheet — you see the face, case, tracking and colour you are choosing. */
function LabelSection({
  labelStyles, labelTheme, setLabelStyle, resetLabelStyle, applyLabelTheme, resetLabels,
}: {
  labelStyles: ReturnType<typeof useSettingsStore.getState>["labelStyles"];
  labelTheme: string;
  setLabelStyle: ReturnType<typeof useSettingsStore.getState>["setLabelStyle"];
  resetLabelStyle: ReturnType<typeof useSettingsStore.getState>["resetLabelStyle"];
  applyLabelTheme: ReturnType<typeof useSettingsStore.getState>["applyLabelTheme"];
  resetLabels: ReturnType<typeof useSettingsStore.getState>["resetLabels"];
}) {
  const known = LABEL_THEME_NAMES.includes(labelTheme);
  return (
    <>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 14 }}>
        <span style={{ color: muted, fontSize: 12 }}>Theme</span>
        <select value={known ? labelTheme : "Custom"}
          onChange={(e) => { const v = e.target.value; if (LABEL_THEME_NAMES.includes(v)) applyLabelTheme(v); }}
          style={inputStyle}>
          {LABEL_THEME_NAMES.map((t) => <option key={t} value={t}>{t}</option>)}
          {!known && <option value="Custom">Custom</option>}
        </select>
        <button onClick={resetLabels}
          style={{ marginLeft: "auto", padding: "5px 11px", borderRadius: 7, border: "1px solid #1e2e42", background: "#0d1219", color: "#9fb6cc", cursor: "pointer", fontSize: 12 }}>
          Reset labels
        </button>
      </div>
      <div style={{ color: muted, fontSize: 11, marginBottom: 12, lineHeight: 1.5 }}>
        Each row is set in its own style, so this list previews exactly how the map will
        read. The default follows the atlas convention: nature is serif and leans, human
        works are sans and stand upright.
      </div>

      {LABEL_GROUPS.map((g) => (
        <div key={g.label} style={{ marginBottom: 12 }}>
          <div style={{ fontSize: 10.5, textTransform: "uppercase", letterSpacing: 1, color: muted, marginBottom: 4 }}>{g.label}</div>
          {g.keys.map(({ k, label }) => {
            const st = labelStyles[k];
            const d = LABEL_STYLE_DEFAULTS[k];
            const changed = (Object.keys(d) as (keyof typeof d)[]).some((f) => st[f] !== d[f]);
            return (
              <div key={k} style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 0", borderBottom: "1px solid #14222f" }}>
                {/* Live specimen — the class's own name, in the class's own style. */}
                <span style={{
                  flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                  fontFamily: st.family, fontWeight: st.weight,
                  fontStyle: st.italic ? "italic" : "normal",
                  letterSpacing: `${st.tracking}em`, color: st.color, fontSize: 14,
                }}>
                  {st.caps ? label.toUpperCase() : label}
                </span>
                <select value={fontKeyOf(st.family)}
                  onChange={(e) => setLabelStyle(k, { family: LABEL_FONTS[e.target.value as LabelFontKey] })}
                  style={{ ...inputStyle, fontSize: 11, padding: "3px 5px", maxWidth: 152 }}>
                  {fontKeyOf(st.family) === "" && <option value="">Custom</option>}
                  {(Object.keys(LABEL_FONTS) as LabelFontKey[]).map((f) => (
                    <option key={f} value={f}>{LABEL_FONT_LABELS[f]}</option>
                  ))}
                </select>
                <input type="color" value={st.color} onChange={(e) => setLabelStyle(k, { color: e.target.value })}
                  style={{ width: 30, height: 24, border: "1px solid #1e2e42", borderRadius: 5, background: "none", padding: 0, cursor: "pointer" }} />
                <span onClick={() => resetLabelStyle(k)} title="Reset to default"
                  style={{ width: 16, textAlign: "center", cursor: "pointer", fontSize: 14, color: changed ? "#9fb6cc" : "#2c4055" }}>↺</span>
              </div>
            );
          })}
        </div>
      ))}
    </>
  );
}
