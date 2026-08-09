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
import { useUIStore } from "@state/uiStore";
import { MAP_THEMES, applyMapTheme, themeReady } from "@ui/world/mapThemes";

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
  const [tab, setTab] = useState<"plates" | "lines" | "labels">("plates");

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
          {([["plates", "Map plates"], ["lines", "Overlay lines"], ["labels", "Map labels"]] as const).map(([id, lbl]) => (
            <button key={id} onClick={() => setTab(id)} style={{
              padding: "5px 12px", borderRadius: 7, fontSize: 12, cursor: "pointer",
              border: `1px solid ${tab === id ? "#2c5a86" : "#1e2e42"}`,
              background: tab === id ? "#16293c" : "#0d1219",
              color: tab === id ? "#cfe2f6" : "#7d97b0",
            }}>{lbl}</button>
          ))}
        </div>

        {tab === "plates" ? (
          <PlateSection />
        ) : tab === "labels" ? (
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

/** MAP PLATES — the full picker (the Toolbar carries a compact twin of this).
 *
 *  An atlas plate is a COMPOSITION, not a layer, so each row states what it
 *  composes: how many overlays it lights, and which label typography it sets. A
 *  plate whose data has not been generated yet reads dimmed with the step it is
 *  waiting on rather than being hidden — you can see what the finished world will
 *  offer before you have built it, which is the same "show the shape of the whole
 *  pipeline" logic the workflow panel uses. */
function PlateSection() {
  const activeMapTheme = useUIStore((s) => s.activeMapTheme);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  return (
    <>
      <div style={{ color: muted, fontSize: 11, marginBottom: 12, lineHeight: 1.5 }}>
        A plate sets the base layer, its overlays and the label typography together — the
        way a published atlas is organised. Applying one changes only what you see; it
        never alters the world.
      </div>

      {MAP_THEMES.map((t) => {
        const ready = themeReady(t, stepCompleted);
        const on = activeMapTheme === t.id;
        return (
          <div
            key={t.id}
            onClick={() => { if (ready) applyMapTheme(t); }}
            style={{
              display: "flex", gap: 10, alignItems: "flex-start",
              padding: "9px 10px", marginBottom: 5, borderRadius: 8,
              cursor: ready ? "pointer" : "not-allowed",
              border: `1px solid ${on ? "#2c5a86" : "#17222f"}`,
              background: on ? "#16293c" : "#0d1219",
              opacity: ready ? 1 : 0.5,
            }}
          >
            <span style={{
              width: 26, height: 26, flex: "0 0 auto", borderRadius: 6,
              display: "flex", alignItems: "center", justifyContent: "center",
              fontSize: 14, background: on ? "#1e3a58" : "#111b26",
              border: `1px solid ${on ? "#3c6f9e" : "#1c2836"}`,
              color: on ? "#cfe2f6" : "#6a86a6",
            }}>{t.glyph}</span>
            <div style={{ minWidth: 0, flex: 1 }}>
              <div style={{ fontSize: 13, fontWeight: on ? 600 : 500, color: on ? "#cfe2f6" : "#a8bed4" }}>
                {t.name}
              </div>
              <div style={{ fontSize: 11, color: muted, lineHeight: 1.45, marginTop: 1 }}>
                {t.blurb}
              </div>
              <div style={{ fontSize: 10, color: "#4d6480", marginTop: 4, fontFamily: "ui-monospace, monospace" }}>
                {ready
                  ? `${t.overlays.length} overlays${t.labelTheme ? ` · ${t.labelTheme} labels` : ""}`
                  : `needs step ${t.requires}`}
              </div>
            </div>
          </div>
        );
      })}
    </>
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
